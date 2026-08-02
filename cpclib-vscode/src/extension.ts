import * as vscode from 'vscode';
import { workspace, ExtensionContext, window } from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

// The resolved `cpclib-lsp` binary path, set once in `activate()` - reused
// by `bndbuildCommandPrefix` so bndbuild execution (Tasks, the "▶ Run"
// CodeLens) invokes the *same* binary as the language server itself,
// running it as `cpclib-lsp bndbuild ...` instead of requiring a second
// `bndbuild` binary on PATH (`cpclib-lsp` already links `cpclib-bndbuild`
// in full for its own `cpclib.runRule`/`cpclib.runTask` LSP commands - this
// reuses that same code as a CLI entry point too, see
// `cpclib-lsp/src/main.rs`'s `run_as_bndbuild`).
let resolvedServerPath: string;

// Selection cycle-count status bar item - created in `activate()`, updated
// by `updateCycleCountStatusBar` (see "Cycle count for selection" section
// below).
let cycleCountStatusBarItem: vscode.StatusBarItem;

// "Registers at cursor" status bar item - created in `activate()`, updated
// by `updateRegistersStatusBar` (see "Registers at cursor" section below).
let registersStatusBarItem: vscode.StatusBarItem;

/// Resolves the `cpclib-lsp` binary when `cpclib-lsp.serverPath` is left at
/// its default. Search order:
/// 1. Bundled platform-specific binary (bin/<platform>/cpclib-lsp[.exe])
/// 2. User's PATH (multiple directories)
/// 3. ~/.cargo/bin (cargo install location)
/// 4. Return the bare name so a normal ENOENT error surfaces if not found
///
/// GUI-launched editors on macOS/Linux commonly don't inherit the PATH a
/// login shell would have (`cargo install`'s `~/.cargo/bin` is added there,
/// not system-wide), so a bare PATH lookup alone silently fails with no
/// indication of *why*.
function resolveServerPath(configured: string, extensionPath: string): string {
    if (configured !== 'cpclib-lsp') {
        return configured; // explicit user override - respect as-is
    }

    const exeName = process.platform === 'win32' ? 'cpclib-lsp.exe' : 'cpclib-lsp';
    
    // Determine platform-specific subdirectory
    let platformDir: string;
    switch (process.platform) {
        case 'win32':
            platformDir = 'windows';
            break;
        case 'darwin':
            platformDir = 'macos';
            break;
        case 'linux':
            platformDir = 'linux';
            break;
        default:
            platformDir = 'linux'; // fallback
    }

    // 1. Check for bundled binary first (highest priority)
    const bundledBinary = path.join(extensionPath, 'bin', platformDir, exeName);
    if (fs.existsSync(bundledBinary)) {
        return bundledBinary;
    }

    // 2. Check PATH and ~/.cargo/bin
    const candidateDirs = [
        ...(process.env.PATH ?? '').split(path.delimiter),
        path.join(os.homedir(), '.cargo', 'bin')
    ];
    for (const dir of candidateDirs) {
        const candidate = path.join(dir, exeName);
        if (fs.existsSync(candidate)) {
            return candidate;
        }
    }

    // 3. Return bare name so ENOENT error surfaces
    return configured;
}

export function activate(context: ExtensionContext) {
    const config = workspace.getConfiguration('cpclib-lsp');
    const serverPath = resolveServerPath(
        config.get<string>('serverPath', 'cpclib-lsp'),
        context.extensionPath
    );
    resolvedServerPath = serverPath;

    const serverOptions: ServerOptions = {
        run: { command: serverPath, transport: TransportKind.stdio },
        debug: { command: serverPath, transport: TransportKind.stdio }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'basm' },
            { scheme: 'file', language: 'bndbuild' },
            { scheme: 'file', language: 'locomotive-basic' },
            { scheme: 'file', language: 'catart-basic' },
            // Unsaved buffers (language mode set manually, not yet written to
            // disk) get no `file:` URI - without these, the client silently
            // drops didOpen/didChange for them and the server never sees the
            // document at all.
            { scheme: 'untitled', language: 'basm' },
            { scheme: 'untitled', language: 'bndbuild' },
            { scheme: 'untitled', language: 'locomotive-basic' },
            { scheme: 'untitled', language: 'catart-basic' }
        ],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('{**/*.{asm,z80,build,bnd,bas,BAS,CAT,cat,ASC,asc},**/bndbuild.yml}')
        },
        middleware: {
            // The server streams build output (stdout/stderr as the rule
            // or task runs) via `window/logMessage`, which
            // vscode-languageclient always writes to its own "CPClib LSP"
            // output channel - but never *shows* that channel on its own.
            // Reveal it right when a build starts, the same way the old
            // terminal-based runner used to pop into view; `true` preserves
            // editor focus.
            //
            // `cpclib.runRuleInTerminal` (the rule-level "▶ Run" CodeLens on
            // a real on-disk .bnd file) deliberately isn't listed here - it
            // runs as a real VS Code Task/terminal instead, which shows
            // itself. This middleware only covers the two commands that
            // still stream through the LSP's own output channel:
            // `cpclib.runRule` (embedded-bndbuild-in-.asm blocks, which have
            // no on-disk file for a terminal Task to invoke) and
            // `cpclib.runTask` (the per-command "▶ Run this command"
            // CodeLens, which has no CLI equivalent for "run just task N of
            // rule R"). Missing `cpclib.runTask` here was a real bug: its
            // output *was* being logged, just never shown, so it looked
            // like nothing happened at all.
            executeCommand: (command, args, next) => {
                if (command === 'cpclib.runRule' || command === 'cpclib.runTask') {
                    client.outputChannel.show(true);
                }
                return next(command, args);
            }
        }
    };

    client = new LanguageClient(
        'cpclib-lsp',
        'CPClib LSP',
        serverOptions,
        clientOptions
    );

    // NOTE: do NOT register `cpclib.runRule` here. The server advertises it in
    // its `executeCommandProvider` capability and vscode-languageclient
    // auto-registers a bridge for every advertised command; registering it a
    // second time throws "command already exists" and aborts the whole client
    // start (no code lenses, no completion, nothing). Clicking the "▶ Run"
    // code lens therefore goes through the bridge to the server, which runs
    // the rule and publishes a diagnostic on the failing line when it fails.
    // The `executeCommand` middleware above reveals the output channel; the
    // bridge (registered by vscode-languageclient itself) forwards the
    // request to the server unchanged.

    // Registered immediately, synchronously - not nested inside
    // `client.start().then(...)`. `Ctrl+Shift+B` ("Run Build Task") asks
    // every *registered* task provider for its tasks the moment it's
    // invoked, with no retry if none are registered yet; registering only
    // after the LSP finishes starting meant any `Ctrl+Shift+B` pressed
    // before that point (a real, easy-to-hit race - e.g. right after
    // opening the workspace) saw no bndbuild tasks at all, permanently for
    // that invocation. `BndbuildTaskProvider.provideTasks` is already
    // async and already tolerates the LSP not being ready yet (a
    // `try`/`catch` around each `sendRequest`, falling back to no targets
    // for that file) - safe to register before `client.start()` resolves.
    const taskProvider = vscode.tasks.registerTaskProvider(
        BndbuildTaskProvider.taskType,
        new BndbuildTaskProvider(client, config),
    );
    context.subscriptions.push(taskProvider);

    client.start().then(() => {
        window.showInformationMessage('CPClib LSP server started.');
    }).catch((err: Error) => {
        window.showErrorMessage(`CPClib LSP failed to start: ${err.message}. Check cpclib-lsp.serverPath setting.`);
    });

    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.pickInkColor', pickInkColor),
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.buildActiveFile', buildActiveFile),
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.runRuleInTerminal', runRuleInTerminal),
    );

    // Keep the `cpclib.cursorOnInkColor` context key (used by the
    // "Pick CPC Ink Color" context-menu entry's `when` clause) in sync with
    // the cursor, so the menu entry only shows up where it'd actually do
    // something.
    context.subscriptions.push(
        window.onDidChangeActiveTextEditor(editor => { void updateCursorContext(editor); }),
        window.onDidChangeTextEditorSelection(e => { void updateCursorContext(e.textEditor); }),
    );
    let colorRefreshTimer: ReturnType<typeof setTimeout> | undefined;
    context.subscriptions.push(
        workspace.onDidChangeTextDocument(e => {
            if (e.document !== window.activeTextEditor?.document) {
                return;
            }
            if (colorRefreshTimer) {
                clearTimeout(colorRefreshTimer);
            }
            // Debounced: re-running documentColor on every keystroke would
            // be wasteful, and the context key only needs to be eventually
            // consistent, not instantaneous.
            colorRefreshTimer = setTimeout(() => { void updateCursorContext(window.activeTextEditor); }, 300);
        }),
    );
    void updateCursorContext(window.activeTextEditor);

    // Live cycle-count status bar item for the current selection - see
    // "Cycle count for selection" section below.
    cycleCountStatusBarItem = window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    context.subscriptions.push(cycleCountStatusBarItem);
    let cycleCountTimer: ReturnType<typeof setTimeout> | undefined;
    context.subscriptions.push(
        window.onDidChangeActiveTextEditor(editor => { void updateCycleCountStatusBar(editor); }),
        window.onDidChangeTextEditorSelection(e => {
            if (cycleCountTimer) {
                clearTimeout(cycleCountTimer);
            }
            // Debounced: dragging out a selection fires this repeatedly:
            // querying the server on every tick would be wasteful, and the
            // status bar only needs to catch up shortly after the drag
            // settles, not track it live keystroke-by-keystroke.
            cycleCountTimer = setTimeout(() => { void updateCycleCountStatusBar(e.textEditor); }, 250);
        }),
    );
    void updateCycleCountStatusBar(window.activeTextEditor);

    // Live "registers at cursor" status bar item - see "Registers at
    // cursor" section below. Same debounced-selection-listener shape as
    // the cycle-count item just above, kept as its own status bar item and
    // its own debounce timer so dragging a selection doesn't cause the two
    // to interfere with each other's timing.
    registersStatusBarItem = window.createStatusBarItem(vscode.StatusBarAlignment.Right, 99);
    context.subscriptions.push(registersStatusBarItem);
    let registersTimer: ReturnType<typeof setTimeout> | undefined;
    context.subscriptions.push(
        window.onDidChangeActiveTextEditor(editor => { void updateRegistersStatusBar(editor); }),
        window.onDidChangeTextEditorSelection(e => {
            if (registersTimer) {
                clearTimeout(registersTimer);
            }
            registersTimer = setTimeout(() => { void updateRegistersStatusBar(e.textEditor); }, 250);
        }),
    );
    void updateRegistersStatusBar(window.activeTextEditor);
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

// ── CPC ink color picker ────────────────────────────────────────────────────
//
// VS Code always opens its own continuous RGB/HSV picker for a language's
// `documentColor` swatches - there is no extension point to replace that
// with something else. This command is a deliberate second path: it reuses
// the LSP's own `documentColor`/`colorPresentation` results (so the ink
// list, ordering, and edits stay a single source of truth with the server)
// but presents them as a `QuickPick` restricted to the real 27/32-entry CPC
// palette, with an accurate color swatch icon per entry. Other editors keep
// using the standard LSP color flow unchanged; this is VS Code-only, purely
// additive on top of it.

// Mirrors `cpclib-lsp/src/common/colors.rs`'s `INK_RGB` table exactly (ink
// index 0-31, the 27-31 range being GA-byte-distinct firmware duplicates of
// an earlier index's RGB) - kept here only to render each `QuickPick`
// item's color swatch icon; the actual ink list/edits still come from the
// server via `colorPresentation`.
const INK_RGB: readonly [number, number, number][] = [
    [0x00, 0x00, 0x00], [0x00, 0x00, 0x80], [0x00, 0x00, 0xFF], [0x80, 0x00, 0x00],
    [0x80, 0x00, 0x80], [0x80, 0x00, 0xFF], [0xFF, 0x00, 0x00], [0xFF, 0x00, 0x80],
    [0xFF, 0x00, 0xFF], [0x00, 0x80, 0x00], [0x00, 0x80, 0x80], [0x00, 0x80, 0xFF],
    [0x80, 0x80, 0x00], [0x80, 0x80, 0x80], [0x80, 0x80, 0xFF], [0xFF, 0x80, 0x00],
    [0xFF, 0x80, 0x80], [0xFF, 0x80, 0xFF], [0x00, 0xFF, 0x00], [0x00, 0xFF, 0x80],
    [0x00, 0xFF, 0xFF], [0x80, 0xFF, 0x00], [0x80, 0xFF, 0x80], [0x80, 0xFF, 0xFF],
    [0xFF, 0xFF, 0x00], [0xFF, 0xFF, 0x80], [0xFF, 0xFF, 0xFF],
    [0x80, 0x80, 0x80], [0xFF, 0x00, 0x80], [0xFF, 0xFF, 0x80], [0x00, 0x00, 0x80], [0x00, 0xFF, 0x80],
];

// The 27 official Amstrad CPC ink names, in index order - mirrors
// `cpclib-image/src/ink.rs`'s `impl Display for Ink` (and its inverse,
// `impl From<String> for Ink`, which also accepts these with spaces
// removed/underscored). Indices 27-31 are firmware duplicates of an
// earlier index's *color* (and so, its name too) - see `INK_RGB` above.
const INK_NAMES: readonly string[] = [
    'Black', 'Blue', 'Bright Blue', 'Red', 'Magenta', 'Mauve', 'Bright Red',
    'Purple', 'Bright Magenta', 'Green', 'Cyan', 'Sky Blue', 'Yellow', 'White',
    'Pastel Blue', 'Orange', 'Pink', 'Pastel Magenta', 'Bright Green', 'Sea Green',
    'Bright Cyan', 'Lime', 'Pastel Green', 'Pastel Cyan', 'Bright Yellow',
    'Pastel Yellow', 'Bright White',
    'White', 'Purple', 'Pastel Yellow', 'Blue', 'Sea Green',
];

/// A small solid-color square as a data-URI SVG, for a `QuickPickItem.iconPath`.
function inkSwatchIcon(rgb: readonly [number, number, number]): vscode.Uri {
    const hex = rgb.map(c => c.toString(16).padStart(2, '0')).join('');
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">` +
        `<rect x="1" y="1" width="14" height="14" fill="#${hex}" stroke="#888888" stroke-width="1"/></svg>`;
    return vscode.Uri.parse(`data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`);
}

interface InkQuickPickItem extends vscode.QuickPickItem {
    presentation: vscode.ColorPresentation;
    idx: number;
}

// Cache of the active document's color swatches (from textDocument/
// documentColor), invalidated whenever the document's version changes -
// shared between the "is the cursor on a color" context-key check (which
// runs on every cursor move) and the picker command itself, so moving the
// cursor around doesn't re-hit the LSP each time.
let cachedColors: vscode.ColorInformation[] = [];
let cachedColorsKey: string | undefined;

async function colorsFor(document: vscode.TextDocument): Promise<vscode.ColorInformation[]> {
    if (
        document.languageId !== 'basm'
        && document.languageId !== 'locomotive-basic'
        && document.languageId !== 'catart-basic'
    ) {
        return [];
    }
    const key = `${document.uri.toString()}@${document.version}`;
    if (key === cachedColorsKey) {
        return cachedColors;
    }
    try {
        const colors = await vscode.commands.executeCommand<vscode.ColorInformation[]>(
            'vscode.executeDocumentColorProvider',
            document.uri,
        );
        cachedColors = colors ?? [];
        cachedColorsKey = key;
    } catch {
        // LSP not ready yet, or the request failed - treat as "no colors"
        // rather than throwing, since this also drives a background
        // context-key update that shouldn't surface an error to the user.
        cachedColors = [];
        cachedColorsKey = key;
    }
    return cachedColors;
}

async function updateCursorContext(editor: vscode.TextEditor | undefined): Promise<void> {
    if (!editor) {
        await vscode.commands.executeCommand('setContext', 'cpclib.cursorOnInkColor', false);
        return;
    }
    const colors = await colorsFor(editor.document);
    const onColor = colors.some(c => c.range.contains(editor.selection.active));
    await vscode.commands.executeCommand('setContext', 'cpclib.cursorOnInkColor', onColor);
}

async function pickInkColor(): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor) {
        return;
    }
    const document = editor.document;
    const position = editor.selection.active;

    const colors = await colorsFor(document);
    const swatch = colors.find(c => c.range.contains(position));
    if (!swatch) {
        window.showInformationMessage('No CPC ink color at the cursor.');
        return;
    }

    // Reuses the server's own textDocument/colorPresentation for the exact
    // edit text/style - only the display order (by number, below) and the
    // added name are computed client-side.
    const presentations = await vscode.commands.executeCommand<vscode.ColorPresentation[]>(
        'vscode.executeColorPresentationProvider',
        swatch.color,
        { uri: document.uri, range: swatch.range },
    );
    if (!presentations || presentations.length === 0) {
        window.showInformationMessage(
            'This is a reference to a named constant and can\'t be edited directly - change its definition instead.'
        );
        return;
    }

    const items: InkQuickPickItem[] = presentations
        .map(p => {
            // Server label is "Ink {idx}" or "Ink {idx} (0x{byte})".
            const match = /^Ink (\d+)(?:\s*\((.+)\))?$/.exec(p.label);
            const idx = match ? parseInt(match[1], 10) : -1;
            const byteDetail = match?.[2];
            const name = INK_NAMES[idx] ?? `Ink ${idx}`;
            const rgb: readonly [number, number, number] =
                INK_RGB[idx] ?? [swatch.color.red * 255, swatch.color.green * 255, swatch.color.blue * 255];
            return {
                label: `${idx}: ${name}`,
                description: byteDetail,
                iconPath: inkSwatchIcon(rgb),
                presentation: p,
                idx,
            };
        })
        .sort((a, b) => a.idx - b.idx);

    // showQuickPick's built-in fuzzy filter matches typed text against the
    // label, so typing a CPC name (e.g. "sky blue") narrows the list too.
    const picked = await window.showQuickPick(items, {
        placeHolder: 'Pick a CPC ink color by number or name',
    });
    if (!picked) {
        return;
    }

    const edits = [picked.presentation.textEdit, ...(picked.presentation.additionalTextEdits ?? [])]
        .filter((e): e is vscode.TextEdit => !!e);
    if (edits.length === 0) {
        return;
    }
    const workspaceEdit = new vscode.WorkspaceEdit();
    for (const edit of edits) {
        workspaceEdit.replace(document.uri, edit.range, edit.newText);
    }
    await vscode.workspace.applyEdit(workspaceEdit);
}

// ── Task provider ─────────────────────────────────────────────────────────────

/// The shell command prefix used to actually run bndbuild (everything
/// before ` -f "file" target`). Defaults to the *already-resolved*
/// `cpclib-lsp` binary itself, run as its `bndbuild` subcommand -
/// `cpclib-lsp` links `cpclib-bndbuild` in full already (for its own
/// `cpclib.runRule`/`cpclib.runTask` LSP commands), so this needs no second
/// binary installed or found on PATH. `cpclib-lsp.bndbuildPath`, if a user
/// explicitly sets it, is an escape hatch to use a real standalone
/// `bndbuild` binary instead (e.g. a different/newer version than the one
/// bundled with this extension's LSP server) - left unset (the default),
/// it's ignored.
function bndbuildCommandPrefix(config: vscode.WorkspaceConfiguration): string {
    const explicit = config.get<string>('bndbuildPath', '');
    if (explicit) {
        return explicit;
    }
    return `"${resolvedServerPath}" bndbuild`;
}

/// Shared by `BndbuildTaskProvider` (Ctrl+Shift+B / `tasks.json`) and the
/// "▶ Run" CodeLens's `cpclib.runRuleInTerminal` command: builds the one
/// `vscode.Task` shape that actually invokes `bndbuild`, with the `$basm`
/// problemMatcher wired in so a build failure lands as a clickable
/// Problems-panel entry "for free" via VS Code's own terminal+task
/// machinery, instead of the LSP's custom `log_message`-streaming path.
function buildBndbuildTask(
    target: string,
    filePath: string,
    bndbuildCommand: string,
    taskName: string,
): vscode.Task {
    const workDir  = path.dirname(filePath);
    const fileName = path.basename(filePath);
    const def: vscode.TaskDefinition = {
        type: BndbuildTaskProvider.taskType,
        target,
        file: filePath,
    };
    const task = new vscode.Task(
        def,
        vscode.TaskScope.Workspace,
        taskName,
        'bndbuild',
        new vscode.ShellExecution(
            `${bndbuildCommand} -f "${fileName}" ${target}`,
            { cwd: workDir },
        ),
        // Parses basm's own codespan-reporting locus format (`error: ...`
        // followed by `┌─ file:line:col`) so a build failure surfaces as a
        // clickable Problems-panel entry.
        '$basm',
    );
    task.group = vscode.TaskGroup.Build;
    return task;
}

class BndbuildTaskProvider implements vscode.TaskProvider {
    static readonly taskType = 'bndbuild';

    constructor(
        private readonly lspClient: LanguageClient,
        private readonly config: vscode.WorkspaceConfiguration,
    ) {}

    async provideTasks(_token: vscode.CancellationToken): Promise<vscode.Task[]> {
        const buildFiles = await vscode.workspace.findFiles(
            '{**/*.{bnd,build},**/bndbuild.yml}',
            '{**/node_modules/**,**/.git/**,**/target/**}',
        );

        const bndbuildCommand = bndbuildCommandPrefix(this.config);
        const tasks: vscode.Task[] = [];

        for (const fileUri of buildFiles) {
            let targets: string[] = [];
            try {
                targets = await this.lspClient.sendRequest<string[]>(
                    'workspace/executeCommand',
                    { command: 'cpclib.getTargets', arguments: [fileUri.toString()] },
                ) ?? [];
            } catch (err) {
                // LSP not ready or file unreadable — skip, but surface it:
                // this used to fail silently, which made "Ctrl+Shift+B finds
                // no tasks" indistinguishable from "this file legitimately
                // has none".
                this.lspClient.outputChannel.appendLine(
                    `[bndbuild task provider] cpclib.getTargets failed for ${fileUri.fsPath}: ${err}`,
                );
            }

            const filePath = fileUri.fsPath;
            const fileName = path.basename(filePath);

            for (const target of targets) {
                const taskName = buildFiles.length > 1 ? `${target} (${fileName})` : target;
                tasks.push(buildBndbuildTask(target, filePath, bndbuildCommand, taskName));
            }
        }

        return tasks;
    }

    resolveTask(task: vscode.Task): vscode.Task | undefined {
        const def = task.definition;
        if (def.type !== BndbuildTaskProvider.taskType || !def.target) {
            return undefined;
        }
        const bndbuildCommand = bndbuildCommandPrefix(this.config);
        const filePath = def.file as string | undefined;
        if (!filePath) {
            return undefined;
        }
        return buildBndbuildTask(def.target as string, filePath, bndbuildCommand, task.name);
    }
}

// ── "▶ Run" CodeLens execution for a real on-disk .bnd file's rule ─────────
//
// `cpclib.runRuleInTerminal(target, filePath)`: a client-only command (never
// sent to the server, deliberately absent from the server's
// `executeCommandProvider.commands`) invoked by the bndbuild file's
// rule-level "▶ Run" CodeLens. Runs the same `bndbuild` CLI invocation as
// `BndbuildTaskProvider`, via a real VS Code Task/terminal, so build errors
// get clickable Problems-panel entries through the already-working `$basm`
// problemMatcher - the LSP's own `cpclib.runRule` streaming path proved
// unreliable at making its own diagnostics clickable. The per-command
// "▶ Run this command" CodeLens keeps using `cpclib.runRule`/`cpclib.runTask`
// (the LSP path), since there is no CLI equivalent for "run just task N of
// rule R" - only a real rule name maps to a real `bndbuild` invocation.
async function runRuleInTerminal(target: string, filePath: string): Promise<void> {
    const config = workspace.getConfiguration('cpclib-lsp');
    const bndbuildCommand = bndbuildCommandPrefix(config);
    const task = buildBndbuildTask(target, filePath, bndbuildCommand, target);
    await vscode.tasks.executeTask(task);
}

// ── Build the active .asm file's own bndbuild target(s) ────────────────────
//
// "cpclib.buildActiveFile": finds every bndbuild file in the workspace that
// references the currently active .asm file (as a dependency or as one of
// its own targets, via the server-side `cpclib.getTargetsForFile`
// reverse-lookup) and runs the one match directly, or offers a QuickPick
// when several build files/targets reference the same source file.

async function buildActiveFile(): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor) {
        return;
    }
    const sourcePath = editor.document.uri.fsPath;
    if (!/\.(asm|z80)$/i.test(sourcePath)) {
        window.showInformationMessage('The active file is not an assembly file.');
        return;
    }

    const buildFiles = await vscode.workspace.findFiles(
        '{**/*.{bnd,build},**/bndbuild.yml}',
        '{**/node_modules/**,**/.git/**,**/target/**}',
    );

    type Match = { buildFileUri: vscode.Uri; target: string };
    const matches: Match[] = [];
    for (const buildFileUri of buildFiles) {
        try {
            const targets = await client.sendRequest<string[]>(
                'workspace/executeCommand',
                { command: 'cpclib.getTargetsForFile', arguments: [buildFileUri.toString(), sourcePath] },
            ) ?? [];
            for (const target of targets) {
                matches.push({ buildFileUri, target });
            }
        } catch {
            // LSP not ready or file unreadable — skip this build file.
        }
    }

    if (matches.length === 0) {
        window.showInformationMessage('No bndbuild file in this workspace references the active file.');
        return;
    }

    let chosen = matches[0];
    if (matches.length > 1) {
        const items = matches.map(m => ({
            label: m.target,
            description: path.basename(m.buildFileUri.fsPath),
            match: m,
        }));
        const picked = await window.showQuickPick(items, { placeHolder: 'Select a build target' });
        if (!picked) {
            return;
        }
        chosen = picked.match;
    }

    const allTasks = await vscode.tasks.fetchTasks({ type: BndbuildTaskProvider.taskType });
    const task = allTasks.find(t =>
        t.definition.target === chosen.target
        && t.definition.file === chosen.buildFileUri.fsPath,
    );
    if (task) {
        await vscode.tasks.executeTask(task);
    } else {
        window.showErrorMessage(`Could not find the '${chosen.target}' build task.`);
    }
}

// ── Cycle count for selection ───────────────────────────────────────────────
//
// The server-side `cpclib.cycleCountForSelection` command (backed by
// `cpclib-lsp/src/basm/cycles.rs`) is also directly usable from the Quick
// Fix menu (a purely informational code action showing the same numbers in
// its title - works in any LSP client, no extension code needed). This is
// the VS Code-only, richer counterpart: a status bar item that stays in
// sync with the current selection live, mirroring `updateCursorContext`'s
// debounced-selection-listener shape above.

interface CycleCountResult {
    min_nops: number;
    max_nops: number;
    // True when the selection contains a block-repeat instruction (LDIR/
    // LDDR/CPIR/CPDR/INIR/INDR/OTIR/OTDR) whose iteration count (BC) isn't
    // statically known - `max_nops` is then a meaningless partial sum, not
    // a real upper bound (see `cpclib-lsp/src/basm/cycles.rs`'s
    // `SelectionCycleCount::max_unbounded` doc comment).
    max_unbounded: boolean;
    instruction_count: number;
    unrecognized_count: number;
}

async function updateCycleCountStatusBar(editor: vscode.TextEditor | undefined): Promise<void> {
    // A bare cursor position (no selection) is sent too, not just a real
    // drag-selection - the server now shows the cost of the single
    // instruction/line the cursor is on in that case (see
    // `cycle_count_for_selection`'s own doc comment in `command.rs`),
    // rather than nothing at all.
    if (!editor || editor.document.languageId !== 'basm') {
        cycleCountStatusBarItem.hide();
        return;
    }

    let result: CycleCountResult | null | undefined;
    try {
        result = await client.sendRequest<CycleCountResult | null>('workspace/executeCommand', {
            command: 'cpclib.cycleCountForSelection',
            arguments: [{
                uri: editor.document.uri.toString(),
                range: client.code2ProtocolConverter.asRange(editor.selection),
            }],
        });
    } catch {
        // LSP not ready yet, or the request failed - same "treat as
        // nothing to show" handling as `colorsFor`'s own try/catch.
        cycleCountStatusBarItem.hide();
        return;
    }
    if (!result) {
        cycleCountStatusBarItem.hide();
        return;
    }

    const { min_nops, max_nops, max_unbounded, unrecognized_count } = result;
    const range = max_unbounded
        ? `${min_nops}-?`
        : min_nops === max_nops ? `${min_nops}` : `${min_nops}-${max_nops}`;
    const warning = unrecognized_count > 0 ? ' ⚠' : '';
    cycleCountStatusBarItem.text = `$(watch) ${range} NOPs${warning}`;

    // Wording only - a bare cursor position (no drag-selection) still
    // shows a real count (the cursor's own line), just not literally a
    // "selection" the user made.
    const label = editor.selection.isEmpty ? 'Cycle count' : 'Selection cycle count';
    let tooltip = max_unbounded
        ? `${label}: ${min_nops} NOPs (best case) - unbounded (a repeat-block instruction's loop count isn't statically known)`
        : min_nops === max_nops
            ? `${label}: ${min_nops} NOPs`
            : `${label}: ${min_nops} NOPs (best case) - ${max_nops} NOPs (worst case, branch taken)`;
    if (unrecognized_count > 0) {
        tooltip += `\n${unrecognized_count} line(s) not counted (macro call or unrecognized instruction) - actual total may be higher.`;
    }
    cycleCountStatusBarItem.tooltip = tooltip;
    cycleCountStatusBarItem.show();
}

// ── Registers at cursor ─────────────────────────────────────────────────────
//
// The server-side `cpclib.registersAtPosition` command (backed by
// `cpclib-lsp/src/basm/registers.rs`'s `all_tracked_registers_at`, the
// all-at-once counterpart to the per-register value already shown in
// instruction hover) drives a compact status bar item: just an icon plus
// "Registers" text, with every tracked register's value (or "?" when not
// statically known at this point) listed in the tooltip - 13 registers is
// too much to usefully cram into the status bar text itself.

interface RegistersResult {
    a: string | null;
    b: string | null;
    c: string | null;
    d: string | null;
    e: string | null;
    h: string | null;
    l: string | null;
    bc: string | null;
    de: string | null;
    hl: string | null;
    ix: string | null;
    iy: string | null;
    sp: string | null;
}

async function updateRegistersStatusBar(editor: vscode.TextEditor | undefined): Promise<void> {
    if (!editor || editor.document.languageId !== 'basm') {
        registersStatusBarItem.hide();
        return;
    }

    let result: RegistersResult | null | undefined;
    try {
        result = await client.sendRequest<RegistersResult | null>('workspace/executeCommand', {
            command: 'cpclib.registersAtPosition',
            arguments: [{
                uri: editor.document.uri.toString(),
                position: client.code2ProtocolConverter.asPosition(editor.selection.active),
            }],
        });
    } catch {
        registersStatusBarItem.hide();
        return;
    }
    if (!result) {
        registersStatusBarItem.hide();
        return;
    }

    registersStatusBarItem.text = '$(list-unordered) Registers';

    const rows: [string, string | null][] = [
        ['A', result.a], ['B', result.b], ['C', result.c],
        ['D', result.d], ['E', result.e], ['H', result.h], ['L', result.l],
        ['BC', result.bc], ['DE', result.de], ['HL', result.hl],
        ['IX', result.ix], ['IY', result.iy], ['SP', result.sp],
    ];
    const lines = rows.map(([name, value]) => `${name.padEnd(2)} = ${value ?? '?'}`);
    registersStatusBarItem.tooltip = `Registers at cursor:\n${lines.join('\n')}`;
    registersStatusBarItem.show();
}
