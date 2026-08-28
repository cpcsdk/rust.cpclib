// Debug session wiring: the adapter, the emulator tab, and the one guard that
// stops a red dot editing your source while a session is running.
//
// The adapter itself is `cpclib-lsp`'s sibling binary `cpclib-dap`; everything
// source-level (turning a line into an address, putting a file and line back
// into a stack frame) happens there, in Rust, where it is tested.

import * as vscode from 'vscode';

/** The debug type contributed in package.json. */
export const DEBUG_TYPE = 'basm';

/**
 * True while a CPC debug session is running.
 *
 * The breakpoint-directive writer consults this: during a session a red dot is
 * a *live* breakpoint sent to the emulator, and the source must not be touched.
 * With no session it keeps writing the `BREAKPOINT` directive, which is what
 * makes a breakpoint survive into a snapshot.
 */
export function isDebugSessionActive(): boolean {
    return vscode.debug.activeDebugSession?.type === DEBUG_TYPE;
}

/**
 * Register everything the debugger needs.
 *
 * `resolveAdapterPath` is passed in rather than imported so this file does not
 * duplicate the per-platform binary search the extension already does.
 */
/** Start a debug session for one .asm or .bas file. */
export async function debugActiveFile(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    const languageId = editor?.document.languageId;
    if (!editor || (languageId !== 'basm' && languageId !== 'locomotive-basic')) {
        void vscode.window.showWarningMessage('Open a .asm or .bas file to debug it.');
        return;
    }
    // Saving first is not politeness: the adapter assembles the file *on disk*,
    // and an unsaved buffer would be debugged as its previous contents.
    await editor.document.save();
    const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri);
    await vscode.debug.startDebugging(folder, {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${editor.document.fileName}`,
        program: editor.document.fileName,
    });
}

/**
 * Debug a named `.asm` file - the "🐞 Debug" CodeLens at the top of it.
 *
 * Distinct from {@link debugActiveFile}, which takes whatever the editor is
 * focused on: a CodeLens names the file it sits in, and honouring that is what
 * makes clicking it do what it looks like it does even if focus has since moved.
 */
/**
 * The program a document belongs to.
 *
 * The file you are looking at is usually not the program: `events.asm` is
 * included by something that is, and assembling it on its own gets an object
 * with no entry point. The server walks the include graph; when more than one
 * program reaches the file, the answer genuinely depends on which is meant, so
 * the choice is put to the user rather than guessed.
 *
 * `undefined` means the user dismissed the question.
 */
export async function resolveEntry(fileName: string): Promise<string | undefined> {
    let answer: { entry?: string; candidates?: string[] } | undefined;
    try {
        // vscode-languageclient auto-registers a bridge command for every
        // name the server advertises, so this reaches the server directly.
        answer = await vscode.commands.executeCommand('cpclib.resolveEntry', fileName);
    } catch {
        // The server could not say; the file itself is the best guess left.
        return fileName;
    }
    if (answer?.entry) { return answer.entry; }
    const candidates = answer?.candidates ?? [];
    if (candidates.length === 0) { return fileName; }

    const picked = await vscode.window.showQuickPick(
        candidates.map(path => ({
            label: vscode.workspace.asRelativePath(path),
            description: path,
        })),
        { placeHolder: `Which program should be built for ${vscode.workspace.asRelativePath(fileName)}?` },
    );
    return picked?.description;
}

export async function debugAssembly(fileName?: string): Promise<void> {
    if (!fileName) {
        await debugActiveFile();
        return;
    }
    const entry = await resolveEntry(fileName);
    if (!entry) { return; }
    fileName = entry;
    const uri = vscode.Uri.file(fileName);
    // Saving first is not politeness: the adapter assembles the file *on disk*,
    // and an unsaved buffer would be debugged as its previous contents.
    const open = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
    if (open?.isDirty) { await open.save(); }

    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${fileName}`,
        program: fileName,
    });
}

/**
 * Debug a named `.bas` file - the "🐞 Debug in emulator" CodeLens at the top
 * of it.
 *
 * Unlike {@link debugAssembly}, there is no entry-point resolution: a `.bas`
 * file is never `#include`d into another, so the file the lens sits in is
 * always the program to run.
 */
export async function debugBasic(fileName?: string): Promise<void> {
    if (!fileName) {
        await debugActiveFile();
        return;
    }
    const uri = vscode.Uri.file(fileName);
    // Saving first is not politeness: the adapter loads the file *on disk*,
    // and an unsaved buffer would be debugged as its previous contents.
    const open = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
    if (open?.isDirty) { await open.save(); }

    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${fileName}`,
        program: fileName,
    });
}

/**
 * Prompts for a `.sna` file: every one found in the open workspace,
 * fuzzy-filterable as you type (`showQuickPick`'s own built-in matching -
 * there is no separate "autocomplete" API for a bare file path in VS Code),
 * plus a "Browse..." entry for one outside it.
 */
async function pickSnapshotFile(): Promise<string | undefined> {
    const found = await vscode.workspace.findFiles(
        '**/*.sna',
        '**/{node_modules,.git,out}/**',
        200,
    );
    const BROWSE = '$(folder-opened) Browse for a .sna file...';
    const picked = await vscode.window.showQuickPick(
        [
            BROWSE,
            ...found.map(uri => vscode.workspace.asRelativePath(uri)),
        ],
        { placeHolder: 'Which .sna snapshot should be run or debugged?' },
    );
    if (picked === undefined) { return undefined; }
    if (picked === BROWSE) {
        const chosen = await vscode.window.showOpenDialog({
            canSelectMany: false,
            filters: { 'CPC snapshot': ['sna'] },
            openLabel: 'Run/Debug',
        });
        return chosen?.[0]?.fsPath;
    }
    const match = found.find(uri => vscode.workspace.asRelativePath(uri) === picked);
    return match?.fsPath;
}

/**
 * Run or debug a raw `.sna` snapshot directly - no build, no source, no
 * assembly. `stopOnEntry` is the entire difference between the two: a
 * snapshot's own `PC` is already mid-program, so "run" is just "don't stop
 * before executing it", the same existing launch property `debugAssembly`'s
 * own `stopOnEntry: false` default already gives .asm/.bas files - nothing
 * new needed on the adapter side for that half. With no `fileName` (the
 * Command Palette case), {@link pickSnapshotFile} offers every `.sna` in
 * the workspace plus a file-browser fallback.
 */
export async function debugSnapshot(fileName?: string, stopOnEntry = true): Promise<void> {
    if (!fileName) {
        fileName = await pickSnapshotFile();
        if (!fileName) { return; }
    }
    const uri = vscode.Uri.file(fileName);
    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `${stopOnEntry ? 'Debug' : 'Run'} ${fileName}`,
        program: fileName,
        stopOnEntry,
    });
}

/**
 * Prompts for a `.dsk` file: every one found in the open workspace, plus a
 * "Browse..." entry - the disk equivalent of {@link pickSnapshotFile}.
 */
async function pickDiskFile(): Promise<string | undefined> {
    const found = await vscode.workspace.findFiles(
        '**/*.dsk',
        '**/{node_modules,.git,out}/**',
        200,
    );
    const BROWSE = '$(folder-opened) Browse for a .dsk file...';
    const picked = await vscode.window.showQuickPick(
        [
            BROWSE,
            ...found.map(uri => vscode.workspace.asRelativePath(uri)),
        ],
        { placeHolder: 'Which .dsk disk should be run or debugged?' },
    );
    if (picked === undefined) { return undefined; }
    if (picked === BROWSE) {
        const chosen = await vscode.window.showOpenDialog({
            canSelectMany: false,
            filters: { 'CPC disk': ['dsk'] },
            openLabel: 'Run/Debug',
        });
        return chosen?.[0]?.fsPath;
    }
    const match = found.find(uri => vscode.workspace.asRelativePath(uri) === picked);
    return match?.fsPath;
}

/**
 * Run or debug a raw `.dsk` disk image directly - mounted in drive A at a
 * cold boot, landing at `Ready` exactly like a real machine with a disk in
 * the drive and no `!BOOT` file: nothing auto-runs, `RUN"..."` is still the
 * user's job. Unlike {@link debugSnapshot}, `stopOnEntry` makes no real
 * difference here (there is no known entry point for a raw disk to stop
 * at) - kept for symmetry with the `.sna` command pair and because the
 * adapter already degrades a no-op `stopOnEntry` to a harmless notice rather
 * than an error. With no `fileName` (the Command Palette case),
 * {@link pickDiskFile} offers every `.dsk` in the workspace plus a
 * file-browser fallback.
 */
export async function debugDisk(fileName?: string, stopOnEntry = true): Promise<void> {
    if (!fileName) {
        fileName = await pickDiskFile();
        if (!fileName) { return; }
    }
    const uri = vscode.Uri.file(fileName);
    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `${stopOnEntry ? 'Debug' : 'Run'} ${fileName}`,
        program: fileName,
        stopOnEntry,
    });
}

/**
 * Start a debug session from a bndbuild rule that launches an emulator.
 *
 * With no rule given, the server is asked which ones actually end in an
 * emulator command and the list is offered - remembering rule names is not the
 * user's job, and typing one that cannot be debugged only fails later.
 */
export async function debugRule(rule?: string, buildFile?: string): Promise<void> {
    if (!rule) {
        const candidates = await debuggableRules();
        if (candidates.length === 0) {
            void vscode.window.showWarningMessage(
                'No rule in the open build files launches an emulator with `run`.',
            );
            return;
        }
        const picked = await vscode.window.showQuickPick(
            candidates.map(c => ({
                label: c.rule,
                description: vscode.workspace.asRelativePath(c.buildFile),
                candidate: c,
            })),
            { placeHolder: 'Which rule should be debugged?' },
        );
        if (!picked) { return; }
        rule = picked.candidate.rule;
        buildFile = picked.candidate.buildFile;
    }
    await vscode.debug.startDebugging(undefined, {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${rule}`,
        rule,
        ...(buildFile ? { buildFile } : {}),
    });
}

export function registerDebugging(
    context: vscode.ExtensionContext,
    resolveAdapterPath: () => string,
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.debugThisFile', debugActiveFile),
        vscode.commands.registerCommand('cpclib.debugAssembly', (fileName?: string) =>
            debugAssembly(fileName)),
        vscode.commands.registerCommand('cpclib.debugBasic', (fileName?: string) =>
            debugBasic(fileName)),
        // A file-explorer context-menu command's own argument is a `Uri`,
        // not a plain path the way a CodeLens's is - both forms are
        // accepted here so the same handler serves the context menu, the
        // Command Palette (no argument at all) and any future CodeLens.
        vscode.commands.registerCommand(
            'cpclib.debugSnapshot',
            (target?: string | vscode.Uri) =>
                debugSnapshot(target instanceof vscode.Uri ? target.fsPath : target, true),
        ),
        vscode.commands.registerCommand(
            'cpclib.runSnapshot',
            (target?: string | vscode.Uri) =>
                debugSnapshot(target instanceof vscode.Uri ? target.fsPath : target, false),
        ),
        vscode.commands.registerCommand(
            'cpclib.debugDisk',
            (target?: string | vscode.Uri) =>
                debugDisk(target instanceof vscode.Uri ? target.fsPath : target, true),
        ),
        vscode.commands.registerCommand(
            'cpclib.runDisk',
            (target?: string | vscode.Uri) =>
                debugDisk(target instanceof vscode.Uri ? target.fsPath : target, false),
        ),
        // The Run lens asks the same question before building: the file you are
        // looking at is usually not the program.
        vscode.commands.registerCommand('cpclib.runAsm', async (fileName?: string) => {
            if (!fileName) {
                fileName = vscode.window.activeTextEditor?.document.fileName;
            }
            if (!fileName) { return; }
            const entry = await resolveEntry(fileName);
            if (!entry) { return; }
            await vscode.commands.executeCommand('cpclib.runAssembly', entry);
        }),
        vscode.commands.registerCommand('cpclib.debugRule', (rule?: string, buildFile?: string) =>
            debugRule(rule, buildFile)),
        vscode.commands.registerCommand('cpclib.openEmulatorInBrowser', async () => {
            const session = vscode.debug.activeDebugSession;
            const url = session ? emulatorUrls.get(session.id) : undefined;
            if (!url) {
                void vscode.window.showWarningMessage(
                    'No emulator is running. Start a debug session first.',
                );
                return;
            }
            // The full machine, with sound - the same one the editor is
            // debugging, since it is served over loopback.
            await vscode.env.openExternal(vscode.Uri.parse(url.replace(/\/debug$/, '/')));
        }),
        // The two views, from the palette as well as from the debug console.
        // They are the same `-dv` and `-mv` the console takes, sent the same
        // way - one implementation, so the panel that opens is the one that
        // knows how to follow `PC` and how to be clicked back into the source.
        vscode.commands.registerCommand('cpclib.openDisassembly', () => consoleCommand('-dv')),
        vscode.commands.registerCommand('cpclib.openMemoryView', () => consoleCommand('-mv')),
        vscode.commands.registerCommand(
            'cpclib.openAllRegisterMemoryViews',
            () => consoleCommand('-mv all,follow'),
        ),
        vscode.commands.registerCommand('cpclib.openCrtcView', () => consoleCommand('-crtcview')),
        vscode.commands.registerCommand('cpclib.openBasicListing', () => consoleCommand('-bv')),
        vscode.commands.registerCommand('cpclib.revealProgramCounter', async () => {
            const where = lastStop;
            if (!where) {
                void vscode.window.showWarningMessage(
                    'The program has not stopped yet, so there is no line to go to.',
                );
                return;
            }
            await revealStop(where);
        }),
    );

    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory(DEBUG_TYPE, {
            createDebugAdapterDescriptor() {
                // Logging is configured in `cpclib-lsp.toml` (`[dap] log`), not
                // here: a session is started from a CodeLens, the palette, F5 or
                // a launch.json, and a setting that must be repeated in each of
                // them is one nobody turns on when they actually need it.
                return new vscode.DebugAdapterExecutable(resolveAdapterPath(), ['dap'], {
                    // The adapter reads the project's configuration from here.
                    cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
                });
            },
        }),
    );

    // F5 on a .asm file with no launch.json at all: synthesise the obvious
    // configuration rather than making the user write one.
    context.subscriptions.push(
        vscode.debug.registerDebugConfigurationProvider(DEBUG_TYPE, {
            resolveDebugConfiguration(_folder, config) {
                if (!config.type && !config.request && !config.name) {
                    const editor = vscode.window.activeTextEditor;
                    const languageId = editor?.document.languageId;
                    if (editor && (languageId === 'basm' || languageId === 'locomotive-basic')) {
                        return {
                            type: DEBUG_TYPE,
                            request: 'launch',
                            name: `Debug this ${languageId === 'basm' ? '.asm' : '.bas'} file`,
                            program: editor.document.fileName,
                        };
                    }
                }
                return config;
            },
        }),
    );

    // VS Code's own Disassembly view, shut before it can take the stop.
    //
    // This adapter deliberately does not advertise `supportsDisassembleRequest`
    // - an editor told it can disassemble keeps stepping at instruction
    // granularity and shows that view instead of your source, which is the
    // opposite of what a source-level debugger is for. But the view is an
    // *editor tab*: once it has been opened it is restored with the window,
    // and it then sits there empty (this session will never fill it), takes
    // the stop, and quietly turns stepping into instruction stepping. Closing
    // it when a session starts is the only way to be rid of it; `-dv` opens a
    // disassembly that actually has contents.
    context.subscriptions.push(
        vscode.debug.onDidStartDebugSession(async session => {
            if (session.type === DEBUG_TYPE) { await closeBuiltInDisassemblyView(); }
        }),
        // ...and again whenever it comes back. Closing it once is not enough:
        // once the editor has switched a session into instruction stepping it
        // re-opens the view on the next step, so the tab has to be answered
        // where it appears rather than where it was first opened.
        vscode.window.tabGroups.onDidChangeTabs(async change => {
            if (vscode.debug.activeDebugSession?.type !== DEBUG_TYPE) { return; }
            if (change.opened.some(looksLikeDisassembly)) {
                await closeBuiltInDisassemblyView();
            }
        }),
    );

    // The resolved-instruction hint describes the address the program is sitting
    // on, so it has to go the moment it is no longer sitting there. `continued`
    // is the only message that says so - the request that caused it may not have
    // been the editor's (a breakpoint hit is resumed by the emulator itself).
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterTrackerFactory(DEBUG_TYPE, {
            createDebugAdapterTracker(): vscode.DebugAdapterTracker {
                return {
                    onDidSendMessage(message: unknown) {
                        const event = (message as { event?: string })?.event;
                        if (event === 'continued') { clearInstructionHint(); }
                    },
                };
            },
        }),
        // A hint set in a file that is then hidden cannot be taken out of an
        // editor nobody can see, so it is caught on the way back: every visible
        // editor showing something other than the hinted file is cleared. The
        // other half is the split made *after* the stop - an editor that was
        // never there to be decorated - which is given the hint here instead.
        vscode.window.onDidChangeVisibleTextEditors(editors => {
            if (!instructionHint) { return; }
            for (const editor of editors) {
                if (currentHint && editor.document.uri.toString() === currentHint.uri) {
                    drawInstructionHint(editor, currentHint);
                } else {
                    editor.setDecorations(instructionHint, []);
                }
            }
        }),
        // The decoration type outlives any one session, so it is the extension
        // that owns it.
        { dispose: () => instructionHint?.dispose() },
    );

    // The emulator's own window, inside the editor.
    context.subscriptions.push(
        vscode.debug.onDidReceiveDebugSessionCustomEvent(async event => {
            if (event.session.type !== DEBUG_TYPE) { return; }
            if (event.event === 'cpclib/emulatorReady') {
                emulatorUrls.set(event.session.id, event.body?.url ?? '');
                await showEmulator(event.session, event.body?.url);
            }
            if (event.event === 'cpclib/memoryView') {
                showMemory(event.session, event.body);
            }
            if (event.event === 'cpclib/disassemblyView') {
                showDisassembly(event.session, event.body);
            }
            if (event.event === 'cpclib/crtcView') {
                showCrtc(event.session, event.body);
            }
            if (event.event === 'cpclib/basicListingView') {
                showBasicListing(event.session, event.body);
            }
            if (event.event === 'cpclib/screenView') {
                showScreen(event.session, event.body);
            }
            // The adapter opened a disassembly view by itself because the
            // program had left the source, and the program is back on a line it
            // was built from. The view has done its job.
            if (event.event === 'cpclib/closeDisassemblyView') {
                disassemblyPanels.get(event.session.id)?.dispose();
            }
            if (event.event === 'cpclib/stoppedAt') {
                lastStop = event.body;
                await revealStop(event.body);
            }
            // Stopped where no source line exists - inside the firmware, most
            // often. There is nothing to reveal, and leaving the previous stop
            // decorated says the program is on a line it left several
            // instructions ago. The disassembly view the adapter opens is what
            // shows where it really is.
            if (event.event === 'cpclib/stoppedWithoutSource') {
                lastStop = undefined;
                clearInstructionHint();
            }
            // The hint the adapter read out of the emulator's own memory, which
            // it could only send once the read came back. Deliberately a second
            // message: re-announcing the stop would drag the cursor back to it
            // for the sake of a decoration.
            if (event.event === 'cpclib/stoppedInstruction') {
                const where = event.body as StopLocation | undefined;
                // `lastStop` is the very object `revealStop` was handed, so
                // updating it here also settles the case where this arrives
                // while that reveal is still opening the document.
                if (lastStop && lastStop.path === where?.path && lastStop.line === where?.line) {
                    lastStop.instruction = where?.instruction ?? null;
                }
                applyInstructionHint(where);
            }
        }),
        vscode.debug.onDidTerminateDebugSession(session => {
            if (session.type === DEBUG_TYPE) {
                disposeEmulator(session.id);
                memoryPanels.get(session.id)?.dispose();
                memoryPanels.delete(session.id);
                disassemblyPanels.get(session.id)?.dispose();
                disassemblyPanels.delete(session.id);
                screenPanels.get(session.id)?.dispose();
                screenPanels.delete(session.id);
                emulatorUrls.delete(session.id);
                lastStop = undefined;
                clearInstructionHint();
            }
        }),
    );
}

interface DebuggableRule { rule: string; buildFile: string }

/** The rules the server considers debuggable, across the open build files. */
async function debuggableRules(): Promise<DebuggableRule[]> {
    if (!client) { return []; }
    try {
        return await client.sendRequest<DebuggableRule[]>(
            'workspace/executeCommand',
            { command: 'cpclib.getDebuggableRules', arguments: [] },
        ) ?? [];
    } catch {
        return [];
    }
}

let client: { sendRequest: <T>(method: string, param: unknown) => Promise<T> } | undefined;

/** Give this module the language client, so it can ask the server questions. */
export function setDebugClient(
    languageClient: { sendRequest: <T>(method: string, param: unknown) => Promise<T> },
): void {
    client = languageClient;
}

const panels = new Map<string, vscode.WebviewPanel>();

async function showEmulator(session: vscode.DebugSession, url: string | undefined): Promise<void> {
    if (!url) { return; }
    if (!vscode.workspace.getConfiguration('cpclib').get<boolean>('debug.openInWebview', true)) {
        await vscode.env.openExternal(vscode.Uri.parse(url));
        return;
    }

    const existing = panels.get(session.id);
    if (existing) { existing.reveal(vscode.ViewColumn.Beside); return; }

    const panel = vscode.window.createWebviewPanel(
        'cpclib.emulator',
        `CPC — ${session.name}`,
        vscode.ViewColumn.Beside,
        // The emulator keeps running while the tab is hidden; without this its
        // whole state would be thrown away every time the tab loses focus.
        { enableScripts: true, retainContextWhenHidden: true },
    );

    // The adapter serves the emulator over loopback; `asExternalUri` is what
    // makes that work in a remote or codespace session too.
    // `toString(true)` skips encoding: the URL is already well-formed, and
    // re-encoding it is how a working address becomes one the page cannot use.
    const external = await vscode.env.asExternalUri(vscode.Uri.parse(url));
    panel.webview.html = emulatorHtml(external.toString(true));

    // Closing the tab must not kill the session - the program is still running,
    // and the user may just want the space back.
    panel.onDidDispose(() => panels.delete(session.id));
    panels.set(session.id, panel);
}

interface MemoryDump {
    viewId?: string;
    /** `-mv all,follow`'s own views all carry the same group name - the
     * editor renders them together in one panel instead of one apiece. */
    group?: string | null;
    /** A person typed this, rather than a stop silently refreshing an
     * already-open panel. */
    requested?: boolean;
    address: number;
    label?: string | null;
    bytes: number[];
    marks?: { offset: number; name: string }[];
    changed?: number[];
}

const memoryPanels = new Map<string, vscode.WebviewPanel>();
// A grouped panel's members, keyed the same way as the panel itself - kept
// separately from the panel so a group's *other* members are still known
// when only one of them just got a fresh read.
const memoryGroupMembers = new Map<string, Map<string, MemoryDump>>();

/**
 * A memory dump, in a tab of its own.
 *
 * The *command* is typed in the debug console (`-mv 0xC000 0x20`) because that
 * is where your hands already are, but the dump belongs in a panel: it is
 * something you keep open and glance at while stepping, and console output
 * scrolls away the moment anything else is printed. One panel per view - the
 * adapter's own `viewId` (an address or a followed register) tells two open
 * views apart, so `-mv HL,follow` and `-mv DE,follow` open two panels side by
 * side rather than one replacing the other; repeating the same view's command
 * still refreshes its own panel rather than opening a duplicate.
 *
 * `-mv all,follow`'s views are the exception: they share a `group`, and all
 * land in one panel together instead - see `showGroupedMemory`.
 */
function showMemory(session: vscode.DebugSession, dump: MemoryDump | undefined): void {
    if (!dump || !Array.isArray(dump.bytes)) { return; }
    if (dump.group) {
        showGroupedMemory(session, dump.group, dump);
        return;
    }

    const key = `${session.id}:${dump.viewId ?? 'default'}`;
    let panel = memoryPanels.get(key);
    const isNew = panel === undefined;
    if (!panel) {
        const title = dump.label ?? `&${hex(dump.address, 4)}`;
        panel = vscode.window.createWebviewPanel(
            'cpclib.memory',
            `CPC memory: ${title} — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: false, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (memoryPanels.get(key) === owned) { memoryPanels.delete(key); }
        });
        memoryPanels.set(key, panel);
    }

    panel.webview.html = memoryHtml(dump);
    // New, or a person just typed the command that reused it (`-mv HL,follow`
    // again brings the HL panel forward instead of leaving it wherever it
    // was). A stop's own silent refresh does neither - revealing it every
    // step would pull it in front of whatever shares its column.
    // `preserveFocus` keeps the keyboard, not the view.
    if (isNew || dump.requested) { panel.reveal(vscode.ViewColumn.Beside, true); }
}

/**
 * One panel for a whole `-mv all,follow` group: each register's read updates
 * its own section without disturbing the others', since the seven reads
 * this triggers do not all complete at once.
 */
function showGroupedMemory(session: vscode.DebugSession, group: string, dump: MemoryDump): void {
    const key = `${session.id}:group:${group}`;
    const members = memoryGroupMembers.get(key) ?? new Map<string, MemoryDump>();
    members.set(dump.viewId ?? dump.label ?? String(dump.address), dump);
    memoryGroupMembers.set(key, members);

    let panel = memoryPanels.get(key);
    const isNew = panel === undefined;
    if (!panel) {
        panel = vscode.window.createWebviewPanel(
            'cpclib.memory',
            `CPC memory: registers — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: false, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (memoryPanels.get(key) === owned) {
                memoryPanels.delete(key);
                memoryGroupMembers.delete(key);
            }
        });
        memoryPanels.set(key, panel);
    }

    panel.webview.html = groupedMemoryHtml(members);
    if (isNew || dump.requested) { panel.reveal(vscode.ViewColumn.Beside, true); }
}

const hex = (value: number, width: number) =>
    value.toString(16).toUpperCase().padStart(width, '0');

const escapeHtml = (text: string) =>
    text.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]!));

/**
 * Sixteen bytes to a row, hex and ASCII, with the program's own labels marked
 * where they start - which is what turns a wall of digits into "this is
 * `animation_state`, and this is the four bytes after it". Just the table -
 * shared between a single view's own page and a grouped panel's several.
 */
function memoryTableHtml(dump: MemoryDump): string {
    const marks = new Map((dump.marks ?? []).map(m => [m.offset, m.name]));
    const changed = new Set(dump.changed ?? []);
    const rows: string[] = [];

    for (let offset = 0; offset < dump.bytes.length; offset += 16) {
        const slice = dump.bytes.slice(offset, offset + 16);
        const cells = slice
            .map((byte, i) => {
                const name = marks.get(offset + i);
                const classes = [name ? 'mark' : '', changed.has(offset + i) ? 'changed' : '']
                    .filter(Boolean)
                    .join(' ');
                const cell = hex(byte, 2);
                return classes
                    ? `<span class="${classes}"${name ? ` title="${escapeHtml(name)}"` : ''}>${cell}</span>`
                    : cell;
            })
            .join(' ');
        // Padding keeps the ASCII column aligned on a short last row.
        const padding = '&nbsp;&nbsp;&nbsp;'.repeat(16 - slice.length);
        const ascii = slice
            .map(byte => (byte >= 0x20 && byte < 0x7f ? escapeHtml(String.fromCharCode(byte)) : '.'))
            .join('');
        const labelled = [...Array(slice.length).keys()]
            .map(i => marks.get(offset + i))
            .filter((name): name is string => !!name);

        rows.push(
            `<tr><td class="addr">&amp;${hex(dump.address + offset, 4)}</td>` +
            `<td class="hex">${cells}${padding}</td>` +
            `<td class="ascii">${ascii}</td>` +
            `<td class="label">${escapeHtml(labelled.join(', '))}</td></tr>`,
        );
    }

    return `<table>${rows.join('')}</table>`;
}

const memoryPageStyle = `
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 8px 12px; }
  h2 { font-size: 1em; font-weight: 600; margin: 0 0 8px; }
  table { border-collapse: collapse; font-variant-numeric: tabular-nums; margin-bottom: 4px; }
  td { padding: 1px 10px 1px 0; white-space: pre; }
  .addr { color: var(--vscode-descriptionForeground); }
  .ascii { color: var(--vscode-descriptionForeground); }
  .label { color: var(--vscode-symbolIcon-variableForeground, inherit); }
  .mark { text-decoration: underline; font-weight: 700; }
  /* What moved since the last stop - the reason to keep this open at all. */
  .changed { background: var(--vscode-diffEditor-insertedTextBackground, #2a4);
             color: var(--vscode-editor-foreground); border-radius: 2px; }
  section { margin-bottom: 18px; }
  footer { margin-top: 10px; color: var(--vscode-descriptionForeground); font-size: 0.9em; }
`;

function memoryHtml(dump: MemoryDump): string {
    const title = dump.label
        ? `${escapeHtml(dump.label)} &mdash; &amp;${hex(dump.address, 4)}`
        : `&amp;${hex(dump.address, 4)}`;

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<style>${memoryPageStyle}</style>
</head>
<body>
<h2>${title} &nbsp;<span class="addr">${dump.bytes.length} bytes</span></h2>
${memoryTableHtml(dump)}
<footer>Refreshed on every stop; highlighted bytes changed since the last one.
Point it elsewhere with <code>-mv</code> in the debug console; <code>-help</code> lists the commands.</footer>
</body>
</html>`;
}

interface ScreenDump {
    png: string;
    address: number;
    width: number;
    height: number;
    mode: number;
    bytes: string;
    charRowHeight: number;
    palette: string[];
    hardwarePalette: string[];
    encoding: number;
}

const screenPanels = new Map<string, vscode.WebviewPanel>();

/**
 * CPC video memory rendered as an actual image (WinAPE-style) - `-sv` in the
 * debug console, or the panel's own controls re-issuing it. Server-side PNG,
 * not a client-side pixel decoder: the mode-aware bit layout stays in one
 * place (`cpclib-image`'s own, already-tested `ColorMatrix`), not duplicated
 * in TypeScript - see the WinAPE-style screen viewer plan's own "reuse,
 * don't reimplement" reasoning. The one piece of address arithmetic that
 * *is* duplicated here, in the page's own script, is the mouse-over
 * readout's coordinate math (screen X/Y -> byte address) - the plan's own
 * explicit exception, since it is simple and low-risk next to the full
 * pixel decode. One panel per session, like `-bv` - there is only ever one
 * screen worth looking at.
 */
function showScreen(session: vscode.DebugSession, dump: ScreenDump | undefined): void {
    if (!dump || typeof dump.png !== 'string') { return; }

    const key = session.id;
    const existing = screenPanels.get(key);
    // An already-open panel gets the new frame pushed into its *existing*
    // page instead of a fresh one - reported live: replacing the whole
    // `webview.html` on every update reloads the page from scratch, which
    // re-runs its own script, which (now that a stop can trigger this on
    // its own, via `refresh_screen_view`) posts another render request back
    // - a full HTML reload for every single step is a visible flicker on
    // its own, and re-entering the script on every one of them turned that
    // into a self-sustaining reload loop that never let a click or a typed
    // character land before the next reload arrived.
    if (existing) {
        void existing.webview.postMessage({ type: 'cpclib.screenFrame', dump });
        return;
    }

    const panel = vscode.window.createWebviewPanel(
        'cpclib.screen',
        `CPC screen — ${session.name}`,
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
        { enableScripts: true, retainContextWhenHidden: true },
    );
    panel.onDidDispose(() => {
        if (screenPanels.get(key) === panel) { screenPanels.delete(key); }
    });
    // The control row posts back exactly the six `-sv` arguments to
    // re-render with - same round trip the console command itself takes,
    // just triggered from the panel instead of typed. `_` is a placeholder
    // for "no override, use the live default" - `-sv`'s own argument
    // parser already treats anything that fails to parse as a number (or,
    // for `palette`, as a comma-separated ink-index list) that way, and
    // unlike an empty string, `_` survives the adapter's plain
    // `split_whitespace()` tokenising, so a *middle* argument (e.g.
    // address, with width set after it) can be left at its default
    // without shifting every argument after it out of position.
    // `totalHeight` - the one field the page computes from its own
    // available space rather than anything the user typed - is always
    // sent, since the adapter has no way to know that itself.
    panel.webview.onDidReceiveMessage((message: {
        address?: string; width?: string; gap?: string; mode?: string; totalHeight?: number;
        palette?: string; encoding?: string;
    }) => {
        if (!message) { return; }
        const raw = [
            message.address, message.width, String(message.totalHeight ?? ''),
            message.mode, message.gap, message.palette, message.encoding,
        ];
        const parts = raw.map(v => {
            const trimmed = (v ?? '').trim();
            return trimmed === '' ? '_' : trimmed;
        });
        while (parts.length > 0 && parts[parts.length - 1] === '_') { parts.pop(); }
        void consoleCommand(`-sv ${parts.join(' ')}`.trimEnd());
    });
    screenPanels.set(key, panel);

    panel.webview.html = screenHtml(dump);
    panel.reveal(vscode.ViewColumn.Beside, true);
}

const SCREEN_MODE_NAMES = [
    '0 (16 colours)', '1 (4 colours)', '2 (2 colours)', '3 (4 colours)',
];

/** How many of the 16 palette pens each mode actually uses. */
const PENS_PER_MODE = [16, 4, 2, 4];

function screenHtml(dump: ScreenDump): string {
    const nonce = Math.random().toString(36).slice(2);
    const modeName = SCREEN_MODE_NAMES[dump.mode] ?? String(dump.mode);
    const pensShown = PENS_PER_MODE[dump.mode] ?? 16;
    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
<style>
html, body { height: 100%; }
body {
  font-family: var(--vscode-editor-font-family, monospace); color: var(--vscode-editor-foreground);
  background: var(--vscode-editor-background); box-sizing: border-box; margin: 0; padding: 0.5em 1em;
  display: flex; flex-direction: column; min-height: 0;
}
h2 { flex: 0 0 auto; margin: 0.2em 0; }
.addr { color: var(--vscode-descriptionForeground); }
footer { flex: 0 0 auto; color: var(--vscode-descriptionForeground); margin-top: 0.4em; font-size: 0.9em; }
code { background: var(--vscode-textCodeBlock-background); padding: 0 0.3em; }
form { flex: 0 0 auto; display: flex; gap: 0.8em; align-items: baseline; flex-wrap: wrap; margin-bottom: 0.3em; }
label { display: flex; gap: 0.35em; align-items: baseline; font-size: 0.9em; }
input, select {
  font-family: var(--vscode-editor-font-family, monospace);
  background: var(--vscode-input-background); color: var(--vscode-input-foreground);
  border: 1px solid var(--vscode-input-border, transparent); border-radius: 2px; padding: 2px 4px;
}
input[type="number"] { width: 4.5em; }
input[type="text"] { width: 5em; text-align: right; }
.stepper { display: inline-flex; align-items: stretch; }
.stepper input { border-radius: 2px 0 0 2px; }
.stepper .arrows { display: flex; flex-direction: column; }
.stepper .arrows button {
  flex: 1; padding: 0 4px; line-height: 1; font-size: 0.6em; border-radius: 0;
}
.stepper .arrows button:first-child { border-radius: 0 2px 0 0; }
.stepper .arrows button:last-child { border-radius: 0 0 2px 0; }
button {
  background: var(--vscode-button-background); color: var(--vscode-button-foreground);
  border: none; border-radius: 2px; padding: 3px 10px; cursor: pointer; font-size: 0.9em;
}
button:hover { background: var(--vscode-button-hoverBackground); }
#palette { flex: 0 0 auto; display: flex; gap: 3px; margin-bottom: 0.4em; flex-wrap: wrap; position: relative; }
#palette .swatch {
  width: 1.1em; height: 1.1em; border: 1px solid var(--vscode-panel-border, #444);
  border-radius: 2px; padding: 0; cursor: pointer;
}
#palette .swatch.overridden { outline: 2px solid var(--vscode-focusBorder, #007acc); outline-offset: 1px; }
#picker {
  position: absolute; top: 1.5em; left: 0; z-index: 10; display: none; flex-wrap: wrap; width: 12em;
  gap: 3px; padding: 4px; background: var(--vscode-editorWidget-background, #252526);
  border: 1px solid var(--vscode-panel-border, #444); border-radius: 3px;
}
#picker.open { display: flex; }
#picker .swatch { width: 1.1em; height: 1.1em; border: 1px solid var(--vscode-panel-border, #444); border-radius: 2px; padding: 0; cursor: pointer; }
#picker .reset { width: 100%; font-size: 0.8em; padding: 2px 4px; }
#imgWrap { flex: 1 1 auto; min-height: 0; overflow: auto; }
canvas#screen { image-rendering: pixelated; border: 1px solid var(--vscode-panel-border, #444); cursor: crosshair; display: block; }
#readout { flex: 0 0 auto; min-height: 1.2em; font-size: 0.9em; }
</style>
</head>
<body>
<h2>Screen &nbsp;<span class="addr">mode ${modeName}</span></h2>
<form id="controls">
  <label>Address (hex) <span class="stepper">
    <input type="text" id="address" value="${hex(dump.address, 4)}" maxlength="4">
    <span class="arrows">
      <button type="button" id="addressUp" title="+1">▲</button>
      <button type="button" id="addressDown" title="-1">▼</button>
    </span>
  </span></label>
  <label>Width (bytes) <input type="number" id="width" min="1" max="255" value="${dump.width}"></label>
  <label>Char row height (lines) <input type="number" id="charRowHeight" min="0" max="2048" value="${dump.charRowHeight}"></label>
  <label>Mode <select id="mode">
    ${[0, 1, 2, 3].map(m => `<option value="${m}"${m === dump.mode ? ' selected' : ''}>${m}</option>`).join('')}
  </select></label>
  <label>Encoding <select id="encoding">
    <option value="0"${dump.encoding === 0 ? ' selected' : ''}>Screen</option>
    <option value="1"${dump.encoding === 1 ? ' selected' : ''}>CPC</option>
  </select></label>
  <button type="button" id="auto">Auto-detect</button>
</form>
<div id="palette" title="This window's own palette - starts from the live Gate Array, click a swatch to change it. Never written back to the emulator: the CPC itself never hears about it."></div>
<div id="imgWrap"><canvas id="screen"></canvas></div>
<div id="readout">&nbsp;</div>
<footer>Move the mouse over the image for the address/value under the cursor. The image fills the panel
automatically, tiling into more columns when there is room for them; everything else applies the moment
it changes. Point it elsewhere with
<code>-sv &lt;address&gt; &lt;width&gt; &lt;height&gt; &lt;mode&gt; &lt;gap&gt; &lt;palette&gt; &lt;encoding&gt;</code>
in the debug console - <code>-help</code> lists every command.</footer>
<script nonce="${nonce}">
  const vscode = acquireVsCodeApi();
  // \`dump\` and \`bytes\` are reassigned by \`applyDump\` on every new frame -
  // including the automatic ones a stop can now trigger on its own
  // (\`refresh_screen_view\`) - so this page is built once and only ever
  // updates in place from here on. Reported live: replacing the whole page
  // (\`panel.webview.html = ...\`) on every frame reloaded it from scratch,
  // which re-ran this very script, which posted another render request of
  // its own - a full-page flicker on every single step, escalating into a
  // reload loop that never let a click or a keystroke land before the next
  // reload arrived.
  let dump = null;
  let bytes = new Uint8Array(0);
  // This window's own palette overrides - pen index -> CPC ink number
  // (0-26), or null for "no override, follow the live Gate Array". Lives
  // only here: never sent to the emulator, and since the page itself no
  // longer reloads on a refresh, a plain JS variable is enough to survive
  // every automatic re-render on its own.
  const paletteOverride = new Array(16).fill(null);
  let openPickerPen = null;

  // WinAPE's own multi-column tiling, generalised to both axes: the panel
  // lays out a full grid of \`rowHeightValue\`-real-lines-tall, \`width\`-
  // bytes-wide tiles, as many as fit both vertically and horizontally, all
  // separated by the *same* padding - reported live: the very first cut of
  // this drew a black pixel row between vertically-stacked tiles but only
  // empty padding between columns, which read as two different features
  // rather than one consistent grid. The server renders one tall,
  // uninterrupted image with no padding or "grid" concept of its own at
  // all (\`columns * rows * rowHeightValue\` real lines, column-major:
  // column 0's own \`rows\` tiles first, in address order, then column 1's);
  // slicing that into the visible grid is done here, on a \`<canvas>\`,
  // entirely client-side - the pixel *decode* still lives in exactly one
  // place (\`cpclib-image\`'s own \`ColorMatrix\`), this is pure re-layout of
  // pixels the server already produced.
  const PADDING = 8;
  let currentColumns = 1;
  let currentRows = 1;
  let currentRowHeightValue = 8;

  const addressField = document.getElementById('address');
  const widthField = document.getElementById('width');
  const charRowHeightField = document.getElementById('charRowHeight');
  const modeField = document.getElementById('mode');
  const encodingField = document.getElementById('encoding');
  const imgWrap = document.getElementById('imgWrap');
  const canvas = document.getElementById('screen');
  const ctx = canvas.getContext('2d');
  const readout = document.getElementById('readout');
  const paletteDiv = document.getElementById('palette');

  // How many \`unitSize\`-plus-padding units fit \`available\` space - the one
  // piece of arithmetic both grid axes share.
  function computeUnitsFitting(available, unitSize) {
    return Math.max(1, Math.floor((Math.max(available, unitSize) + PADDING) / (unitSize + PADDING)));
  }

  // One tile's own real-line count - typed value if there is one, else the
  // live CRTC's own \`charRowHeight\` - and how many of those fit one
  // column's available height.
  function currentRowHeightAndRows() {
    const typed = parseInt(charRowHeightField.value, 10);
    const rowHeightValue = Number.isFinite(typed) && typed > 0 ? typed : (dump ? dump.charRowHeight : 8);
    const rows = computeUnitsFitting(imgWrap.clientHeight, rowHeightValue * 2);
    return { rowHeightValue, rows };
  }

  function paletteArgument(useDefaults) {
    if (useDefaults) { return ''; }
    if (paletteOverride.every(v => v === null)) { return ''; }
    return paletteOverride.map(v => (v === null ? '' : String(v))).join(',');
  }

  // No "Refresh" button: every control applies the moment it changes -
  // 'change' rather than 'input' so typing a multi-digit number does not
  // re-render on every keystroke, while a dropdown, the spinner arrows or
  // the address stepper still apply at once. \`useDefaults\` is what
  // "Auto-detect" asks for: every typed field, and every palette override,
  // is dropped - only the freshly computed total height still goes out
  // (the adapter has no other way to learn the panel's own available
  // space, or how many columns fit it).
  function requestRender(useDefaults) {
    if (useDefaults) { paletteOverride.fill(null); }
    // The address field shows and edits bare hex ("C000"), but the
    // adapter's own \`parse_number\`/\`parse_address\` only read a value as
    // hex when it carries a prefix (\`0x\`/\`&\`/...) - anything else parses
    // as decimal, which "C000" is not, so it silently failed to parse at
    // all and the override was always dropped. Reported live: editing the
    // address field, or the stepper, had no visible effect.
    const addressValue = addressField.value.trim();
    const width = parseInt(widthField.value, 10) || (dump ? dump.width : 80);
    const { rowHeightValue, rows } = currentRowHeightAndRows();
    const columns = computeUnitsFitting(imgWrap.clientWidth, width * 8);
    vscode.postMessage({
      address: useDefaults || addressValue === '' ? '' : ('0x' + addressValue),
      width: useDefaults ? '' : widthField.value,
      gap: useDefaults ? '' : charRowHeightField.value,
      mode: useDefaults ? '' : modeField.value,
      encoding: useDefaults ? '' : encodingField.value,
      totalHeight: columns * rows * rowHeightValue,
      palette: paletteArgument(useDefaults),
    });
  }

  document.getElementById('controls').addEventListener('change', () => requestRender(false));
  document.getElementById('controls').addEventListener('submit', event => event.preventDefault());
  document.getElementById('auto').addEventListener('click', () => requestRender(true));

  function stepAddress(delta) {
    const current = parseInt(addressField.value, 16);
    const next = ((Number.isFinite(current) ? current : (dump ? dump.address : 0)) + delta) & 0xFFFF;
    addressField.value = next.toString(16).toUpperCase().padStart(4, '0');
    requestRender(false);
  }

  // Reported live: one click already re-rendered, but holding the button
  // down did not keep going the way a typical spinner control would - a
  // single \`click\` handler fires exactly once per press, however long it
  // is held. \`mousedown\` starts a real repeat instead: one immediate step,
  // then a short pause before a steady auto-repeat, stopped by \`mouseup\`/
  // \`mouseleave\` alike (releasing outside the button never gets stuck).
  function holdToRepeat(button, delta) {
    let repeatTimer = null;
    const stop = () => {
      clearTimeout(repeatTimer);
      repeatTimer = null;
    };
    button.addEventListener('mousedown', () => {
      stop();
      stepAddress(delta);
      repeatTimer = setTimeout(function repeat() {
        stepAddress(delta);
        repeatTimer = setTimeout(repeat, 80);
      }, 400);
    });
    button.addEventListener('mouseup', stop);
    button.addEventListener('mouseleave', stop);
  }
  holdToRepeat(document.getElementById('addressUp'), 1);
  holdToRepeat(document.getElementById('addressDown'), -1);

  function closePicker() {
    const existing = document.getElementById('picker');
    if (existing) { existing.remove(); }
    openPickerPen = null;
  }

  function openPicker(pen, anchor) {
    if (openPickerPen === pen) { closePicker(); return; }
    closePicker();
    openPickerPen = pen;
    const picker = document.createElement('div');
    picker.id = 'picker';
    picker.className = 'open';
    const reset = document.createElement('button');
    reset.type = 'button';
    reset.className = 'reset';
    reset.textContent = 'Live Gate Array colour';
    reset.addEventListener('click', () => {
      paletteOverride[pen] = null;
      closePicker();
      requestRender(false);
    });
    picker.appendChild(reset);
    dump.hardwarePalette.forEach((colour, ink) => {
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.className = 'swatch';
      chip.style.background = colour;
      chip.title = 'Ink ' + ink + ': ' + colour;
      chip.addEventListener('click', () => {
        paletteOverride[pen] = ink;
        closePicker();
        requestRender(false);
      });
      picker.appendChild(chip);
    });
    anchor.parentElement.appendChild(picker);
  }

  document.addEventListener('click', event => {
    if (openPickerPen !== null && !event.target.closest('#picker') && !event.target.closest('.swatch')) {
      closePicker();
    }
  });

  function renderPaletteSwatches() {
    closePicker();
    paletteDiv.textContent = '';
    const pensShown = ${JSON.stringify(PENS_PER_MODE)}[dump.mode] ?? 16;
    dump.palette.slice(0, pensShown).forEach((colour, pen) => {
      const swatch = document.createElement('button');
      swatch.type = 'button';
      swatch.className = 'swatch' + (paletteOverride[pen] !== null ? ' overridden' : '');
      swatch.style.background = colour;
      swatch.title = 'Pen ' + pen + ': ' + colour +
        (paletteOverride[pen] !== null ? ' (window override)' : ' (live)');
      swatch.addEventListener('click', () => openPicker(pen, swatch));
      paletteDiv.appendChild(swatch);
    });
  }

  // Applies one rendered frame - the initial one, and every automatic
  // refresh after it - without touching anything the page itself owns
  // (scroll position, focus, the palette picker if one happens to be
  // open). A field the user has focus in right now is left alone even
  // though the server's own answer might disagree with what they are mid-
  // typing - the live address in particular legitimately changes stop to
  // stop (a scrolled screen), so updating the *display* on a refresh is
  // correct, just not while someone's cursor is sitting in that field.
  function applyDump(newDump) {
    dump = newDump;
    bytes = Uint8Array.from(atob(dump.bytes), c => c.charCodeAt(0));

    const focused = document.activeElement;
    if (focused !== addressField) { addressField.value = hexAddress(dump.address); }
    if (focused !== widthField) { widthField.value = String(dump.width); }
    if (focused !== charRowHeightField) { charRowHeightField.value = String(dump.charRowHeight); }
    if (focused !== modeField) { modeField.value = String(dump.mode); }
    if (focused !== encodingField) { encodingField.value = String(dump.encoding); }

    renderPaletteSwatches();
    readout.textContent = '\\u00a0';

    // The server rendered one tall, uninterrupted, ungapped image: exactly
    // \`columns * rows\` tiles' worth of real lines, column-major (column 0's
    // own \`rows\` tiles first, in address order, then column 1's, and so
    // on - \`dump.height\`, the total real lines actually rendered, divides
    // evenly by \`columns * rows\` for the same reason). Slicing that into
    // the visible grid, with real padding on both axes, happens only here -
    // the multi-column *or* multi-row tiling never touches the server at
    // all, and reported live, an in-image black row for one axis but real
    // padding for the other looked like two different features instead of
    // one grid.
    const source = new Image();
    source.onload = () => {
      const columnPixelWidth = dump.width * 8;
      const requestedColumns = computeUnitsFitting(imgWrap.clientWidth, columnPixelWidth);
      const { rowHeightValue, rows: requestedRows } = currentRowHeightAndRows();
      currentColumns = requestedColumns;
      currentRows = requestedRows;
      currentRowHeightValue = rowHeightValue;

      const tilePixelHeight =
        Math.floor(source.naturalHeight / (requestedColumns * requestedRows)) || source.naturalHeight;

      canvas.width = requestedColumns * columnPixelWidth + (requestedColumns - 1) * PADDING;
      canvas.height = requestedRows * tilePixelHeight + (requestedRows - 1) * PADDING;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      for (let c = 0; c < requestedColumns; c++) {
        for (let r = 0; r < requestedRows; r++) {
          const sy = (c * requestedRows + r) * tilePixelHeight;
          const dx = c * (columnPixelWidth + PADDING);
          const dy = r * (tilePixelHeight + PADDING);
          ctx.drawImage(
            source, 0, sy, source.naturalWidth, tilePixelHeight,
            dx, dy, columnPixelWidth, tilePixelHeight
          );
        }
      }
    };
    source.src = 'data:image/png;base64,' + dump.png;
  }

  function hexAddress(address) {
    return address.toString(16).toUpperCase().padStart(4, '0');
  }

  window.addEventListener('message', event => {
    if (event.data && event.data.type === 'cpclib.screenFrame') {
      applyDump(event.data.dump);
    }
  });

  // The panel's own available height changes with the window and with
  // every other VS Code tab/split the user opens - re-fit on any of that,
  // debounced so a drag-resize does not flood the adapter with requests.
  let resizeTimer = null;
  new ResizeObserver(() => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => requestRender(false), 250);
  }).observe(imgWrap);

  // The very first image was rendered before this script - and therefore
  // this panel's own available height - existed at all, so it used the
  // adapter's flat, un-sized fallback. Applying it directly (not through
  // \`requestRender\`, which would post a request nobody is waiting to
  // answer for the panel's very first paint) then correcting it once, right
  // away, is cheaper than restructuring the launch-time "run -sv, then open
  // the panel" order just to avoid one extra round trip.
  applyDump({
    png: ${JSON.stringify(dump.png)},
    address: ${dump.address},
    width: ${dump.width},
    height: ${dump.height},
    mode: ${dump.mode},
    charRowHeight: ${dump.charRowHeight},
    bytes: ${JSON.stringify(dump.bytes)},
    palette: ${JSON.stringify(dump.palette)},
    hardwarePalette: ${JSON.stringify(dump.hardwarePalette)},
    encoding: ${dump.encoding},
  });
  requestRender(false);

  canvas.addEventListener('mousemove', event => {
    const rect = canvas.getBoundingClientRect();
    const mx = Math.floor((event.clientX - rect.left) * (canvas.width / rect.width));
    const my = Math.floor((event.clientY - rect.top) * (canvas.height / rect.height));
    if (mx < 0 || my < 0 || mx >= canvas.width || my >= canvas.height) { return; }

    // Which grid tile was hovered, and where within it - the thin padding
    // between tiles, on either axis, has no byte of its own.
    const columnPixelWidth = dump.width * 8;
    const columnStep = columnPixelWidth + PADDING;
    const columnIndex = Math.floor(mx / columnStep);
    const px = mx - columnIndex * columnStep;

    const tilePixelHeight = currentRowHeightValue * 2;
    const rowStep = tilePixelHeight + PADDING;
    const rowIndex = Math.floor(my / rowStep);
    const py = my - rowIndex * rowStep;

    if (
      px >= columnPixelWidth || columnIndex >= currentColumns ||
      py >= tilePixelHeight || rowIndex >= currentRows
    ) {
      readout.textContent = '\\u00a0';
      return;
    }

    // The image is already stretched to the CPC's own pixel aspect ratio
    // server-side (\`render_screen_view\`, see its own comment): 8 displayed
    // dots per byte horizontally on every mode alike, and every row doubled
    // vertically - so undoing exactly that recovers the logical column and
    // the row within this tile.
    const col = Math.floor(px / 8);
    const localLine = Math.floor(py / 2);
    // The server rendered one continuous, column-major stream of
    // \`currentColumns * currentRows\` tiles and this page sliced it into a
    // grid for display - undo that slicing to get back the real line's own
    // position in that one continuous stream, which is what the address
    // formulas below (both encodings alike) were written against.
    const line = (columnIndex * currentRows + rowIndex) * currentRowHeightValue + localLine;

    let address;
    if (dump.encoding === 1) {
      // WinAPE's "CPC" encoding: plain sequential bytes, \`charRowHeight\`
      // plays no part at all, wrapped at the full 64K space - see
      // \`ColorMatrix::from_linear_memory\`'s own doc comment.
      address = (dump.address + line * dump.width + col) & 0xFFFF;
    } else {
      // WinAPE's "Screen" encoding, matching \`ColorMatrix::from_screen_at\`
      // server-side exactly (see its own doc comment for the hardware
      // reasoning): \`MA\` (row position) advances by \`dump.width\` once
      // every \`charRowHeight\` lines (the live \`R9 + 1\`), but the raster-
      // within-row term that multiplies \`0x800\` is only 3 bits wide on
      // real hardware - it wraps at 8 regardless of how tall the row is
      // configured to be. Both terms, plus \`col\`, wrap *within the
      // screen's own 16K bank* - real CRTC/Gate Array hardware never lets
      // this arithmetic spill into a different one.
      const rowHeight = dump.charRowHeight > 0 ? dump.charRowHeight : 1;
      const ra = (line % rowHeight) % 8;
      const pageBase = dump.address & 0xC000;
      const offsetInPage = dump.address & 0x3FFF;
      address =
        pageBase + ((offsetInPage + Math.floor(line / rowHeight) * dump.width + ra * 0x800 + col) & 0x3FFF);
    }
    const value = bytes[address];
    const tileNote = (currentColumns > 1 || currentRows > 1)
      ? \` (tile column \${columnIndex}, row \${rowIndex})\`
      : '';
    readout.textContent = \`&\${address.toString(16).toUpperCase().padStart(4, '0')}\` +
      \` = &\${value.toString(16).toUpperCase().padStart(2, '0')} (\${value})\` +
      \` — column \${col}, row \${line}\${tileNote}\`;
  });
  canvas.addEventListener('mouseleave', () => { readout.textContent = '\\u00a0'; });
</script>
</body>
</html>`;
}

interface BasicListingDump {
    text: string;
}

const basicListingPanels = new Map<string, vscode.WebviewPanel>();

/**
 * The live BASIC listing, straight from the emulator's own memory - `-bv` in
 * the debug console, one panel per session (there is only ever one program
 * loaded at a time, unlike memory views which can point anywhere).
 */
function showBasicListing(session: vscode.DebugSession, dump: BasicListingDump | undefined): void {
    if (!dump || typeof dump.text !== 'string') { return; }

    const key = session.id;
    let panel = basicListingPanels.get(key);
    const isNew = panel === undefined;
    if (!panel) {
        panel = vscode.window.createWebviewPanel(
            'cpclib.basicListing',
            `BASIC listing — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: false, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (basicListingPanels.get(key) === owned) { basicListingPanels.delete(key); }
        });
        basicListingPanels.set(key, panel);
    }

    panel.webview.html = basicListingHtml(dump.text);
    if (isNew) { panel.reveal(vscode.ViewColumn.Beside, true); }
}

/**
 * Locomotive BASIC keywords worth colouring in the `-bv` panel - a
 * pragmatic, reasonably wide list rather than an exhaustive grammar
 * (matched case-insensitively: AMSpiriT's own listing renders uppercase,
 * this session's own generic-peer decode preserves whatever case was
 * typed - see the case-folding investigation in `basic_session.rs`).
 */
const BASIC_KEYWORDS = new Set([
    'MODE', 'INK', 'BORDER', 'PAPER', 'PEN', 'DEFINT', 'DEFSTR', 'DEFREAL', 'DIM', 'ERASE',
    'FOR', 'TO', 'STEP', 'NEXT', 'IF', 'THEN', 'ELSE', 'GOTO', 'GOSUB', 'RETURN', 'ON',
    'PRINT', 'INPUT', 'LINE', 'LOCATE', 'CLS', 'CLEAR', 'RUN', 'STOP', 'END', 'REM', 'LET',
    'AND', 'OR', 'XOR', 'NOT', 'MOD', 'PLOT', 'PLOTR', 'DRAW', 'DRAWR', 'MOVE', 'MOVER',
    'ORIGIN', 'WINDOW', 'SYMBOL', 'WHILE', 'WEND', 'DATA', 'READ', 'RESTORE', 'ERROR',
    'RESUME', 'CALL', 'POKE', 'PEEK', 'OUT', 'INP', 'USR', 'RANDOMIZE', 'SOUND', 'ENV',
    'ENT', 'TAG', 'TAGOFF', 'WAIT', 'FRAME', 'EVERY', 'AFTER', 'SPEED', 'KEY', 'ZONE',
    'WIDTH', 'MASK', 'FILL', 'GRAPHICS', 'TROFF', 'TRON', 'LIST', 'NEW', 'SAVE', 'LOAD',
    'MERGE', 'CAT', 'ERA', 'OPENOUT', 'OPENIN', 'CLOSEIN', 'CLOSEOUT', 'CHAIN', 'RENUM',
    'CONT', 'DELETE', 'EDIT', 'AUTO', 'MID$', 'LEFT$', 'RIGHT$', 'STR$', 'CHR$', 'VAL',
    'LEN', 'ASC', 'INT', 'ABS', 'SGN', 'SQR', 'SIN', 'COS', 'TAN', 'PI', 'RND', 'EOF',
    'SPC', 'TAB', 'INKEY', 'INKEY$', 'JOY', 'FRE', 'HIMEM', 'XPOS', 'YPOS', 'TIME'
]);

/**
 * A pragmatic BASIC syntax highlighter for the `-bv` panel: pulls out
 * string literals first (so a keyword-looking word inside a quoted string
 * is never coloured), then colours the leading line number and any
 * recognised keyword in what remains. Not a real tokeniser - good enough
 * for readability, not meant to be authoritative the way the debugger's
 * own decode is.
 */
function basicListingHtml(text: string): string {
    const highlightedLines = text.split('\n').map(line => {
        const lineNumberMatch = line.match(/^(\d+)(\s*)/);
        let rest = line;
        let prefix = '';
        if (lineNumberMatch) {
            prefix = `<span class="linenum">${lineNumberMatch[1]}</span>${escapeHtml(lineNumberMatch[2])}`;
            rest = line.slice(lineNumberMatch[0].length);
        }

        // Split on string literals first, so nothing inside one gets
        // mistaken for a keyword.
        const parts = rest.split(/("[^"]*")/);
        const highlighted = parts
            .map((part, i) => {
                if (i % 2 === 1) {
                    // A captured string literal (odd indices from the
                    // split above).
                    return `<span class="string">${escapeHtml(part)}</span>`;
                }
                return part
                    .split(/([A-Za-z_][A-Za-z0-9_$]*)/)
                    .map(word => {
                        if (BASIC_KEYWORDS.has(word.toUpperCase())) {
                            return `<span class="keyword">${escapeHtml(word)}</span>`;
                        }
                        return escapeHtml(word);
                    })
                    .join('');
            })
            .join('');

        return prefix + highlighted;
    });

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<style>
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 8px 12px; }
  pre { white-space: pre-wrap; margin: 0; }
  .linenum { color: var(--vscode-editorLineNumber-foreground, var(--vscode-descriptionForeground)); }
  .keyword { color: var(--vscode-symbolIcon-keywordForeground, var(--vscode-debugTokenExpression-name, #569cd6)); font-weight: 600; }
  .string { color: var(--vscode-debugTokenExpression-string, #ce9178); }
  footer { margin-top: 10px; color: var(--vscode-descriptionForeground); font-size: 0.9em; }
</style>
</head>
<body>
<pre>${highlightedLines.join('\n')}</pre>
<footer>Read from the emulator's own memory, not from the source file on disk -
re-type <code>-bv</code> in the debug console to refresh it.</footer>
</body>
</html>`;
}

/**
 * `-mv all,follow`'s panel: every register's memory in one page, one section
 * apiece, instead of a tab per register - reusing `memoryTableHtml` per
 * member is the whole difference from the single-view page above.
 */
function groupedMemoryHtml(members: Map<string, MemoryDump>): string {
    // The order this was typed in - PC, SP, HL, DE, BC, IX, IY - not
    // whatever order the Map happens to iterate in.
    const order = ['register:PC', 'register:SP', 'register:HL', 'register:DE',
        'register:BC', 'register:IX', 'register:IY'];
    const ids = [...members.keys()].sort((a, b) => {
        const ia = order.indexOf(a);
        const ib = order.indexOf(b);
        return (ia === -1 ? order.length : ia) - (ib === -1 ? order.length : ib);
    });

    const sections = ids.map(id => {
        const dump = members.get(id)!;
        const title = dump.label
            ? `${escapeHtml(dump.label)} &mdash; &amp;${hex(dump.address, 4)}`
            : `&amp;${hex(dump.address, 4)}`;
        return `<section><h2>${title}</h2>${memoryTableHtml(dump)}</section>`;
    });

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<style>${memoryPageStyle}</style>
</head>
<body>
${sections.join('')}
<footer>Refreshed on every stop; highlighted bytes changed since the last one.
<code>-mv &lt;register&gt;,follow</code> opens one of these on its own instead;
<code>-help</code> lists the commands.</footer>
</body>
</html>`;
}

interface CrtcRegister {
    name: string;
    value: number;
}

interface CrtcWarning {
    registers: string[];
    severity: 'error' | 'warning';
    message: string;
}

interface CrtcDump {
    registers: CrtcRegister[];
    warnings: CrtcWarning[];
}

const crtcPanels = new Map<string, vscode.WebviewPanel>();

/**
 * The CRTC registers, with any combination `validate_crtc` (Rust side) knows
 * to misbehave on real hardware highlighted in red - the plain "CRTC" scope
 * in the Variables view shows the same registers, but DAP has no way to mark
 * one row differently from another, so this is where the red actually goes.
 *
 * Opened with `-crtcview` in the debug console; unlike the memory view it is
 * not re-read on every stop (reading it means saving a whole machine, the
 * same cost `-chips` already avoids paying automatically) - re-run the
 * command for a fresh look.
 */
function showCrtc(session: vscode.DebugSession, dump: CrtcDump | undefined): void {
    if (!dump || !Array.isArray(dump.registers)) { return; }

    let panel = crtcPanels.get(session.id);
    if (!panel) {
        panel = vscode.window.createWebviewPanel(
            'cpclib.crtc',
            `CPC CRTC — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: false, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (crtcPanels.get(session.id) === owned) { crtcPanels.delete(session.id); }
        });
        crtcPanels.set(session.id, panel);
    }

    panel.webview.html = crtcHtml(dump);
    // Unlike the memory view, -crtcview is never a silent per-stop refresh -
    // every call is a person asking, so it always comes forward, not only
    // when the panel is new.
    panel.reveal(vscode.ViewColumn.Beside, true);
}

function crtcHtml(dump: CrtcDump): string {
    const flagged = new Map<string, CrtcWarning[]>();
    for (const warning of dump.warnings) {
        for (const register of warning.registers) {
            const list = flagged.get(register) ?? [];
            list.push(warning);
            flagged.set(register, list);
        }
    }

    const cells = dump.registers.map(reg => {
        const warnings = flagged.get(reg.name) ?? [];
        const severity = warnings.some(w => w.severity === 'error')
            ? 'error'
            : warnings.length > 0 ? 'warning' : '';
        const title = warnings.map(w => w.message).join(' — ');
        return `<div class="reg${severity ? ` ${severity}` : ''}"${title ? ` title="${escapeHtml(title)}"` : ''}>` +
            `<span class="name">${escapeHtml(reg.name)}</span>` +
            `<span class="value">${hex(reg.value, 2)}</span></div>`;
    }).join('');

    const causes = dump.warnings.length
        ? `<ul class="causes">${dump.warnings
            .map(w => `<li class="${w.severity}"><b>${escapeHtml(w.registers.join(', '))}</b> — ${escapeHtml(w.message)}</li>`)
            .join('')}</ul>`
        : '<p class="ok">No known-bad register combination found.</p>';

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<style>
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 8px 12px; }
  h2 { font-size: 1em; font-weight: 600; margin: 0 0 8px; }
  .grid { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 12px; }
  .reg { display: flex; flex-direction: column; align-items: center; padding: 4px 8px;
         border: 1px solid var(--vscode-panel-border, #444); border-radius: 3px; min-width: 34px; }
  .reg .name { font-size: 0.75em; color: var(--vscode-descriptionForeground); }
  .reg .value { font-variant-numeric: tabular-nums; font-weight: 600; }
  .reg.error { border-color: var(--vscode-editorError-foreground, #f14c4c);
               background: var(--vscode-inputValidation-errorBackground, #5a1d1d); }
  .reg.warning { border-color: var(--vscode-editorWarning-foreground, #cca700);
                 background: var(--vscode-inputValidation-warningBackground, #5a4a1d); }
  .causes { margin: 0; padding-left: 1.2em; }
  .causes li.error { color: var(--vscode-editorError-foreground, #f14c4c); }
  .causes li.warning { color: var(--vscode-editorWarning-foreground, #cca700); }
  .ok { color: var(--vscode-descriptionForeground); }
  footer { margin-top: 10px; color: var(--vscode-descriptionForeground); font-size: 0.9em; }
</style>
</head>
<body>
<h2>CRTC registers</h2>
<div class="grid">${cells}</div>
${causes}
<footer>Not refreshed automatically - re-run <code>-crtcview</code> in the debug console for a
current look; <code>-help</code> lists the commands.</footer>
</body>
</html>`;
}

interface DisassembledInstruction {
    address: string;
    instruction: string;
    instructionBytes?: string;
    symbol?: string;
    line?: number;
    column?: number;
    endColumn?: number;
    /** Labels the addresses in this instruction's operands stand for. */
    symbols?: string[];
    /**
     * Other labels that share this row's own address with `symbol`. No
     * source line names a heading the way a call names its target, so there
     * is no evidence to pick between them - shown rather than guessed at.
     */
    symbolAlternatives?: string[];
    location?: { name?: string; path?: string };
}

interface Disassembly {
    address: number;
    label?: string | null;
    instructions: DisassembledInstruction[];
    /** Where the program actually is, so the row can be marked. */
    pc?: number | null;
    /** Whether this view moves with the program on every step. */
    followsPc?: boolean;
}

const disassemblyPanels = new Map<string, vscode.WebviewPanel>();

/**
 * Where each session's emulator is being served.
 *
 * Kept so the emulator can be opened in a real browser. That is the only place
 * its **sound** works: the page starts its AudioContext from a user gesture,
 * and a VS Code webview does not give it one that Chromium accepts - tried
 * directly, on resume, and behind a click-to-enable button, none of which
 * persuade it. A browser tab is not a workaround so much as the honest answer,
 * and the debugger keeps working while you listen: the emulator is served over
 * loopback, so the browser and the editor are looking at the same machine.
 */
const emulatorUrls = new Map<string, string>();

/**
 * Disassembled memory, with every row a link back to the source it came from.
 *
 * Memory and source are not the same thing in a demo, in two ways that matter
 * while debugging: a macro or a `REPEAT` turns one source line into a screenful
 * of opcodes, and self-modifying code means the bytes running are not the bytes
 * that were assembled. So this is worth having open *beside* the source rather
 * than instead of it, and clicking a row opens the line it came from.
 */
function showDisassembly(session: vscode.DebugSession, dump: Disassembly | undefined): void {
    if (!dump || !Array.isArray(dump.instructions)) { return; }

    let panel = disassemblyPanels.get(session.id);
    const isNew = panel === undefined;
    if (!panel) {
        panel = vscode.window.createWebviewPanel(
            'cpclib.disassembly',
            `CPC disassembly — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: true, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (disassemblyPanels.get(session.id) === owned) {
                disassemblyPanels.delete(session.id);
            }
        });
        // Clicking a row opens its source line - the "navigate both at once"
        // half of having this open at all.
        panel.webview.onDidReceiveMessage(async (
            message: { path?: string; line?: number; column?: number; endColumn?: number },
        ) => {
            if (!message?.path) { return; }
            try {
                const document = await vscode.workspace.openTextDocument(
                    vscode.Uri.file(message.path),
                );
                const line = Math.max(0, (message.line ?? 1) - 1);
                // The *instruction*, not the line. A line is often three of
                // them (`ld e,(hl) : inc hl : ld d,(hl)`) and landing on the
                // first when you clicked the third is landing in the wrong
                // place.
                const from = Math.max(0, (message.column ?? 1) - 1);
                const to = Math.max(from, (message.endColumn ?? message.column ?? 1) - 1);
                // Reuse the column the file is already open in, same as
                // `revealStop` below and for the same reason: omitting
                // `viewColumn` defaults to the *active* column, not wherever
                // the file already happens to be open, so it duplicated a tab
                // the file already had elsewhere instead of reusing it.
                const already = vscode.window.visibleTextEditors.find(
                    editor => editor.document.uri.toString() === document.uri.toString()
                );
                const opened = await vscode.window.showTextDocument(document, {
                    viewColumn: already?.viewColumn,
                    preview: false,
                    selection: new vscode.Range(line, from, line, to),
                });
                // Backwards, as on a stop: a click usually lands on the row the
                // program is sitting on, where the caret would otherwise share
                // its position with the hint and be drawn the width of it.
                opened.selection = new vscode.Selection(
                    new vscode.Position(line, to),
                    new vscode.Position(line, from),
                );
            } catch {
                vscode.window.showWarningMessage(`Cannot open ${message.path}`);
            }
        });
        disassemblyPanels.set(session.id, panel);
    }

    panel.webview.html = disassemblyHtml(dump);
    // Brought to the front only when it was just asked for. A `PC`-following
    // view refreshes on *every* stop, and revealing it each time pulls it over
    // whatever shares its column - which is how a breakpoint in your own code
    // ends up showing the disassembly instead of the line that stopped.
    if (isNew) { panel.reveal(vscode.ViewColumn.Beside, true); }
}

/**
 * Colour a Z80 instruction the way the editor colours the source it came from.
 *
 * Deliberately a small tokeniser rather than a grammar: the text comes from our
 * own disassembler, so it is already regular - `MNEMONIC operand, operand` with
 * numbers, registers and parenthesised indirections. Anything unrecognised is
 * left plain, which is the right failure for a view whose job is to be read.
 */
function highlightInstruction(text: string): string {
    const REGISTERS = new Set([
        'a', 'f', 'b', 'c', 'd', 'e', 'h', 'l', 'i', 'r',
        'af', 'bc', 'de', 'hl', 'ix', 'iy', 'sp', 'pc',
        "af'", 'ixh', 'ixl', 'iyh', 'iyl',
        'nz', 'z', 'nc', 'po', 'pe', 'p', 'm',
    ]);

    // Split on the boundaries that matter, keeping them, so the pieces can be
    // reassembled with nothing lost.
    const pieces = text.split(/([\s,()+\-]+)/);
    let first = true;
    return pieces
        .map(piece => {
            if (piece === '') { return ''; }
            if (/^[\s,()+\-]+$/.test(piece)) { return escapeHtml(piece); }
            const escaped = escapeHtml(piece);
            if (first) {
                first = false;
                return `<span class="mnemonic">${escaped}</span>`;
            }
            if (/^(0x|&|#|\$)?[0-9a-fA-F]+$/.test(piece) || /^\d+$/.test(piece)) {
                return `<span class="number">${escaped}</span>`;
            }
            if (REGISTERS.has(piece.toLowerCase())) {
                return `<span class="register">${escaped}</span>`;
            }
            return `<span class="symbol">${escaped}</span>`;
        })
        .join('');
}

function disassemblyHtml(dump: Disassembly): string {
    const nonce = Math.random().toString(36).slice(2);
    const pcReference = typeof dump.pc === 'number' ? `0x${hex(dump.pc, 4)}` : null;
    const rows = dump.instructions.map(entry => {
        const path = entry.location?.path ?? '';
        // The emulator writes addresses lowercase; compare on the number.
        const atPc = pcReference !== null &&
            parseInt(entry.address.replace(/^0x/i, ''), 16) === dump.pc;
        // `file:line:col` - the column is what tells you *which* instruction of
        // a shared line this opcode came from.
        const where = path && entry.line
            ? `${escapeHtml(entry.location?.name ?? '')}:${entry.line}` +
              (entry.column ? `:${entry.column}` : '')
            : '';
        const named = (entry.symbols ?? []).length
            ? `<span class="operand-symbol">; ${escapeHtml((entry.symbols ?? []).join(', '))}</span>`
            : '';
        // Rows for one source line repeat it; that repetition *is* the
        // information when a macro produced twenty opcodes from one line.
        const alternatives = (entry.symbolAlternatives ?? []).length
            ? ` <span class="operand-symbol">(also ${escapeHtml((entry.symbolAlternatives ?? []).join(', '))})</span>`
            : '';
        const heading = entry.symbol
            ? `<tr><td colspan="4" class="symbol">${escapeHtml(entry.symbol)}:${alternatives}</td></tr>`
            : '';
        return heading + `<tr class="${[path ? 'linked' : '', atPc ? 'at-pc' : ''].filter(Boolean).join(' ')}"` +
            (path
                ? ` data-path="${escapeHtml(path)}" data-line="${entry.line ?? 1}"` +
                  ` data-column="${entry.column ?? 1}" data-end-column="${entry.endColumn ?? entry.column ?? 1}"`
                : '') +
            `><td class="addr">${atPc ? '▶ ' : '\u00a0\u00a0'}${escapeHtml(entry.address)}</td>` +
            `<td class="bytes">${escapeHtml(entry.instructionBytes ?? '')}</td>` +
            `<td class="insn">${highlightInstruction(entry.instruction ?? '')}${named}</td>` +
            `<td class="src">${where}</td></tr>`;
    });

    const title = dump.label
        ? `${escapeHtml(dump.label)} &mdash; &amp;${hex(dump.address, 4)}`
        : `&amp;${hex(dump.address, 4)}`;

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
<style>
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 8px 12px; }
  h2 { font-size: 1em; font-weight: 600; margin: 0 0 8px; }
  table { border-collapse: collapse; width: 100%; }
  td { padding: 1px 12px 1px 0; white-space: pre; }
  .addr, .bytes, .src { color: var(--vscode-descriptionForeground); }
  /* The same roles the editor gives your source, so the two read alike. */
  .mnemonic { color: var(--vscode-debugTokenExpression-name, #569cd6); font-weight: 600; }
  .register { color: var(--vscode-symbolIcon-variableForeground, #9cdcfe); }
  .number   { color: var(--vscode-debugTokenExpression-number, #b5cea8); }
  .symbol   { color: var(--vscode-symbolIcon-functionForeground, #dcdcaa); }
  /* What an address in an operand stands for: the difference between
     CALL 0xBB5A and CALL 0xBB5A ; TXT_OUTPUT. */
  .operand-symbol { color: var(--vscode-descriptionForeground); font-style: italic;
                    margin-left: 10px; }
  .symbol { color: var(--vscode-symbolIcon-functionForeground, inherit); font-weight: 700;
            padding-top: 8px; }
  .linked { cursor: pointer; }
  /* Where the program actually is - the row you are comparing against your
     source, and the reason to keep this open while stepping. */
  .at-pc { background: var(--vscode-editor-selectionBackground); font-weight: 700; }
  .linked:hover { background: var(--vscode-list-hoverBackground); }
  footer { margin-top: 10px; color: var(--vscode-descriptionForeground); font-size: 0.9em; }
</style>
</head>
<body>
<h2>${title} &nbsp;<span class="addr">${dump.instructions.length} instructions</span></h2>
<table>${rows.join('')}</table>
<footer>Decoded by <code>basm</code>'s own tables, not by the emulator - so this
reads the same whichever emulator is underneath.
${dump.followsPc
    ? 'Following <strong>PC</strong>: this re-reads itself on every step.'
    : 'Anchored here; <code>-dv</code> with no argument follows <strong>PC</strong> instead.'}
Click a row to open the line it came from. This is what is <em>in memory</em>:
after self-modifying code, or a macro, it will not match your source one-for-one.</footer>
<script nonce="${nonce}">
  const vscode = acquireVsCodeApi();
  document.querySelectorAll('tr.linked').forEach(row => {
    row.addEventListener('click', () => vscode.postMessage({
      path: row.dataset.path,
      line: Number(row.dataset.line),
      column: Number(row.dataset.column),
      endColumn: Number(row.dataset.endColumn),
    }));
  });
</script>
</body>
</html>`;
}

function disposeEmulator(sessionId: string): void {
    panels.get(sessionId)?.dispose();
    panels.delete(sessionId);
}

function emulatorHtml(url: string): string {
    const origin = new URL(url).origin;
    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; frame-src ${origin}; style-src 'unsafe-inline';
               script-src 'unsafe-inline';">
<style>
  html, body { margin: 0; padding: 0; height: 100%; background: #000; }
  iframe { border: 0; width: 100%; height: 100%; display: block; }
</style>
</head>
<body><iframe src="${url}" allow="autoplay; gamepad; keyboard-map"
              allowfullscreen tabindex="0"></iframe>
<script>
  // Hand the keyboard to the emulator whenever this tab is the active one.
  // A webview that keeps focus on its own document swallows every keystroke,
  // and the CPC never sees a key.
  const frame = document.querySelector('iframe');
  const focusEmulator = () => { try { frame.contentWindow.focus(); } catch (_) {} };
  window.addEventListener('focus', focusEmulator);
  document.addEventListener('pointerdown', focusEmulator);
  frame.addEventListener('load', focusEmulator);
  setTimeout(focusEmulator, 300);
</script>
</body>
</html>`;
}

/**
 * Close VS Code's built-in Disassembly view, wherever it is.
 *
 * Identified by having no recognised editor input - it is not a file, a
 * notebook, a diff or a webview - together with a label naming it. Both halves
 * matter: the label alone would risk closing someone's file called
 * "disassembly.asm", and the input alone would catch every exotic editor.
 */
function looksLikeDisassembly(tab: vscode.Tab): boolean {
    const known =
        tab.input instanceof vscode.TabInputText ||
        tab.input instanceof vscode.TabInputTextDiff ||
        tab.input instanceof vscode.TabInputCustom ||
        tab.input instanceof vscode.TabInputWebview ||
        tab.input instanceof vscode.TabInputNotebook ||
        tab.input instanceof vscode.TabInputTerminal;
    return !known && /disassembl|d\u00e9sassembl/i.test(tab.label);
}

async function closeBuiltInDisassemblyView(): Promise<void> {
    const doomed = vscode.window.tabGroups.all
        .flatMap(group => group.tabs)
        .filter(looksLikeDisassembly);
    if (doomed.length > 0) {
        await vscode.window.tabGroups.close(doomed, false);
    }
}

/**
 * Type a debug-console command for the user.
 *
 * `-dv` and `-mv` answer with a `cpclib/*View` event, which opens the panel -
 * so the palette entries are the console entries, not a second way of doing
 * the same thing that could drift from it.
 */
async function consoleCommand(expression: string): Promise<void> {
    const session = vscode.debug.activeDebugSession;
    if (!session) {
        void vscode.window.showWarningMessage(
            'No debug session is running. Start one first.',
        );
        return;
    }
    try {
        await session.customRequest('evaluate', { expression, context: 'repl' });
    } catch (error) {
        void vscode.window.showErrorMessage(`${expression} failed: ${error}`);
    }
}

/** The last stop, so it can be returned to on demand. */
let lastStop: StopLocation | undefined;

/** Where the program stopped, as the adapter reports it. */
interface StopLocation {
    path?: string;
    line?: number;
    column?: number;
    endColumn?: number;
    /**
     * The instruction the machine really holds there, when it is not what the
     * line says - `ld a,0x01` for a line reading `ld a,ANIMATION_STATE_FINISHED`.
     * Absent when the source already spells it out, and there is nothing to
     * disambiguate.
     *
     * Decoded from the *emulator's* memory, so it is also the answer for an
     * instruction that has modified itself, for one written instruction that
     * became several real ones, and for a line reading `defs` whose code was
     * generated at run time. It therefore arrives a round trip after the stop,
     * in `cpclib/stoppedInstruction`, and is null on the `cpclib/stoppedAt`
     * that revealed the line - except where there was no emulator to ask, when
     * the assembled image answers immediately instead.
     */
    instruction?: string | null;
}

/**
 * The dimmed text after the stopped line, made on first use.
 *
 * One type for the whole extension: a decoration type is how VS Code addresses
 * a set of decorations, so making a second one would leave the first one's text
 * on screen with no way left to remove it.
 */
let instructionHint: vscode.TextEditorDecorationType | undefined;

/**
 * What the hint says and where it belongs.
 *
 * Kept rather than only written into an editor because the same file can be
 * open in more than one group, and a group can be split *after* the stop: an
 * editor that appears later has to be given the hint it never saw, and one
 * showing anything else can be known to be stale.
 */
interface InstructionHint {
    /** The hinted document, as `Uri.toString()` - what editors are matched on. */
    uri: string;
    line: number;
    text: string;
    endColumn?: number;
}

let currentHint: InstructionHint | undefined;

function instructionHintType(): vscode.TextEditorDecorationType {
    instructionHint ??= vscode.window.createTextEditorDecorationType({
        after: {
            margin: '0 0 0 2em',
            // Ghost text: the colour VS Code already uses for code-shaped text
            // that is not in the file. Exactly this hint's status, and lighter
            // than real code in every theme.
            //
            // One colour for the whole hint, not one per token as the `-dv`
            // panel gives it. An `after` decoration is a single text
            // attachment and takes a single `color`; the only way to colour
            // its words separately is several decoration types at the same
            // position, whose rendering order VS Code does not define - and a
            // hint that comes out as `a, ld 0x01)(` is worse than a plain one.
            color: new vscode.ThemeColor('editorGhostText.foreground'),
        },
        // The hint describes one address, so it must not be dragged along by an
        // edit above it.
        rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
    });
    return instructionHint;
}

/**
 * Show, after the line, what the bytes at `PC` decode to.
 *
 * Written as an instruction rather than as a comment - it *is* one, in the
 * assembler's own spelling - and parenthesised so it cannot be mistaken for
 * code that is in the file.
 */
function showInstructionHint(
    document: vscode.TextDocument,
    line: number,
    text: string,
    endColumn?: number,
): void {
    clearInstructionHint();
    if (line >= document.lineCount) { return; }

    currentHint = { uri: document.uri.toString(), line, text, endColumn };
    for (const editor of editorsShowing(currentHint.uri)) {
        drawInstructionHint(editor, currentHint);
    }
}

/**
 * Every visible editor showing one document.
 *
 * Not `find`: a decoration belongs to an *editor*, not to a document, so the
 * same file open in two groups is two editors and decorating the first one
 * leaves the second showing the stopped line with nothing beside it.
 */
function editorsShowing(uri: string): vscode.TextEditor[] {
    return vscode.window.visibleTextEditors.filter(
        editor => editor.document.uri.toString() === uri,
    );
}

/** Put the hint into one editor. */
function drawInstructionHint(editor: vscode.TextEditor, hint: InstructionHint): void {
    if (hint.line >= editor.document.lineCount) { return; }
    // Just after the instruction, not at the end of the line. A line can hold
    // several instructions (`ld a,l : inc a : ld (hl),a`), and a hint parked at
    // the far right would sit beside whichever came last rather than beside the
    // one being executed.
    const lineRange = editor.document.lineAt(hint.line).range;
    const at = hint.endColumn && hint.endColumn > 1
        ? lineRange.start.translate(0, Math.min(hint.endColumn - 1, lineRange.end.character))
        : lineRange.end;
    editor.setDecorations(instructionHintType(), [{
        range: new vscode.Range(at, at),
        renderOptions: { after: { contentText: `(${hint.text})` } },
    }]);
}

/**
 * Put the hint on the editor already showing the stop, without disturbing it.
 *
 * Separate from `revealStop` because it arrives separately: the adapter reads
 * the bytes at `PC` from the emulator, which costs a round trip it refuses to
 * make the reveal wait for.
 */
function applyInstructionHint(where: StopLocation | undefined): void {
    if (!where?.path || !where.line) { return; }
    const line = Math.max(0, where.line - 1);
    // Any editor showing the file will do to name the document; the hint then
    // goes to all of them.
    const document = vscode.window.visibleTextEditors.find(
        candidate => candidate.document.uri.fsPath === where.path,
    )?.document;
    if (!document) { return; }
    if (where.instruction) {
        showInstructionHint(document, line, where.instruction, where.endColumn);
    } else {
        clearInstructionHint();
    }
}

/**
 * Take the hint away, everywhere.
 *
 * A hint left over from the previous stop is worse than no hint at all: it
 * describes an address the program has already left, in a spelling that looks
 * authoritative. So it goes on continue, on the next stop, and when the session
 * ends - and from every editor, since the stop may since have moved file.
 */
function clearInstructionHint(): void {
    currentHint = undefined;
    if (!instructionHint) { return; }
    for (const editor of vscode.window.visibleTextEditors) {
        editor.setDecorations(instructionHint, []);
    }
}

/**
 * Open the line the program stopped on, and put the cursor on the instruction.
 *
 * The stack trace already carries this and the editor is meant to act on it,
 * but whether it *reveals* the file turned out to depend on what happened to
 * hold the editor area - the emulator's own webview, a panel, a view restored
 * from a previous session - so the answer was "sometimes". Doing it here does
 * not depend on any of that.
 *
 * `preserveFocus` is deliberately false: stopping at a breakpoint is exactly
 * the moment you want the keyboard in the source.
 */
async function revealStop(where: StopLocation | undefined): Promise<void> {
    if (!where?.path || !where.line) { return; }
    let document: vscode.TextDocument;
    try {
        document = await vscode.workspace.openTextDocument(vscode.Uri.file(where.path));
    } catch {
        return; // a file we cannot open is not worth an error popup on every stop
    }

    const line = Math.max(0, where.line - 1);
    // Columns are 1-based in the protocol and 0-based here. The range is the
    // *instruction*, not the whole line: `ld e,(hl) : inc hl` is two of them,
    // and landing on the first when the second is running is a wrong answer.
    const from = Math.max(0, (where.column ?? 1) - 1);
    const to = where.endColumn && where.endColumn > (where.column ?? 1)
        ? where.endColumn - 1
        : from;
    const selection = new vscode.Range(line, from, line, to);

    // No `viewColumn` here defaults to the *active* column, not wherever the
    // file already happens to be open - and the active column is whatever the
    // user last clicked into, which between stops is routinely the emulator's
    // own window, not this file. Every stop was opening a fresh tab in
    // whichever column that left active, rather than reusing the one already
    // showing this file - repeatedly stealing focus back from the emulator
    // (1984js needs it to keep running: see cpclib-bridge.js) far more often
    // than a debugger stop actually needs to.
    const already = vscode.window.visibleTextEditors.find(
        editor => editor.document.uri.toString() === document.uri.toString()
    );

    const editor = await vscode.window.showTextDocument(document, {
        viewColumn: already?.viewColumn,
        preserveFocus: false,
        preview: false,
        selection,
    });
    // Selected backwards, so the caret lands on the *first* character of the
    // instruction rather than just past its last one. `showTextDocument` leaves
    // the caret at the end of the selection, which is exactly where the hint's
    // `after` decoration sits; the editor measures a caret across the whole DOM
    // box at that position, and that box carries the hint's text, so the caret
    // came out as wide as the hint. Nothing about the selection changes but
    // which of its ends the caret is on.
    editor.selection = new vscode.Selection(selection.end, selection.start);
    editor.revealRange(selection, vscode.TextEditorRevealType.InCenterIfOutsideViewport);

    if (where.instruction) {
        showInstructionHint(document, line, where.instruction, where.endColumn);
    } else {
        clearInstructionHint();
    }
}
