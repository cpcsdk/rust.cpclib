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
        }),
        vscode.debug.onDidTerminateDebugSession(session => {
            if (session.type === DEBUG_TYPE) {
                disposeEmulator(session.id);
                memoryPanels.get(session.id)?.dispose();
                memoryPanels.delete(session.id);
                disassemblyPanels.get(session.id)?.dispose();
                disassemblyPanels.delete(session.id);
                emulatorUrls.delete(session.id);
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
    address: number;
    label?: string | null;
    bytes: number[];
    marks?: { offset: number; name: string }[];
    changed?: number[];
}

const memoryPanels = new Map<string, vscode.WebviewPanel>();

/**
 * A memory dump, in a tab of its own.
 *
 * The *command* is typed in the debug console (`-mv 0xC000 0x20`) because that
 * is where your hands already are, but the dump belongs in a panel: it is
 * something you keep open and glance at while stepping, and console output
 * scrolls away the moment anything else is printed. One panel per session,
 * reused, so repeating the command refreshes what you are already looking at
 * rather than burying it under a new tab.
 */
function showMemory(session: vscode.DebugSession, dump: MemoryDump | undefined): void {
    if (!dump || !Array.isArray(dump.bytes)) { return; }

    let panel = memoryPanels.get(session.id);
    if (!panel) {
        panel = vscode.window.createWebviewPanel(
            'cpclib.memory',
            `CPC memory — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: false, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (memoryPanels.get(session.id) === owned) { memoryPanels.delete(session.id); }
        });
        memoryPanels.set(session.id, panel);
    }

    panel.webview.html = memoryHtml(dump);
    // `preserveFocus` on every reveal: the panel refreshes itself on every
    // stop, and stealing focus from the editor on each step would make
    // stepping unusable.
    panel.reveal(vscode.ViewColumn.Beside, true);
}

const hex = (value: number, width: number) =>
    value.toString(16).toUpperCase().padStart(width, '0');

const escapeHtml = (text: string) =>
    text.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]!));

/**
 * Sixteen bytes to a row, hex and ASCII, with the program's own labels marked
 * where they start - which is what turns a wall of digits into "this is
 * `animation_state`, and this is the four bytes after it".
 */
function memoryHtml(dump: MemoryDump): string {
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

    const title = dump.label
        ? `${escapeHtml(dump.label)} &mdash; &amp;${hex(dump.address, 4)}`
        : `&amp;${hex(dump.address, 4)}`;

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<style>
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 8px 12px; }
  h2 { font-size: 1em; font-weight: 600; margin: 0 0 8px; }
  table { border-collapse: collapse; font-variant-numeric: tabular-nums; }
  td { padding: 1px 10px 1px 0; white-space: pre; }
  .addr { color: var(--vscode-descriptionForeground); }
  .ascii { color: var(--vscode-descriptionForeground); }
  .label { color: var(--vscode-symbolIcon-variableForeground, inherit); }
  .mark { text-decoration: underline; font-weight: 700; }
  /* What moved since the last stop - the reason to keep this open at all. */
  .changed { background: var(--vscode-diffEditor-insertedTextBackground, #2a4);
             color: var(--vscode-editor-foreground); border-radius: 2px; }
  footer { margin-top: 10px; color: var(--vscode-descriptionForeground); font-size: 0.9em; }
</style>
</head>
<body>
<h2>${title} &nbsp;<span class="addr">${dump.bytes.length} bytes</span></h2>
<table>${rows.join('')}</table>
<footer>Refreshed on every stop; highlighted bytes changed since the last one.
Point it elsewhere with <code>-mv</code> in the debug console; <code>-help</code> lists the commands.</footer>
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
                await vscode.window.showTextDocument(document, {
                    viewColumn: vscode.ViewColumn.One,
                    selection: new vscode.Range(line, from, line, to),
                });
            } catch {
                vscode.window.showWarningMessage(`Cannot open ${message.path}`);
            }
        });
        disassemblyPanels.set(session.id, panel);
    }

    panel.webview.html = disassemblyHtml(dump);
    panel.reveal(vscode.ViewColumn.Beside, true);
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
        const heading = entry.symbol
            ? `<tr><td colspan="4" class="symbol">${escapeHtml(entry.symbol)}:</td></tr>`
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
