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
/** Start a debug session for one .asm file. */
export async function debugActiveFile(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (editor?.document.languageId !== 'basm') {
        void vscode.window.showWarningMessage('Open a .asm file to debug it.');
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
                    if (editor?.document.languageId === 'basm') {
                        return {
                            type: DEBUG_TYPE,
                            request: 'launch',
                            name: 'Debug this .asm file',
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
                // No `viewColumn`, and `preview: false` - same as `revealStop`
                // below, and for the same reason: forcing a column here
                // duplicated a tab the file already had open elsewhere
                // instead of reusing it.
                const opened = await vscode.window.showTextDocument(document, {
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

    const editor = await vscode.window.showTextDocument(document, {
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
