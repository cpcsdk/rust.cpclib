import * as vscode from 'vscode';
import { isDebugSessionActive, registerDebugging } from './debug';
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

// Glob for discovering bndbuild files in the workspace - mirrors
// `cpclib_bndbuild::builder::EXPECTED_FILENAMES` exactly (`bndbuild.yml`,
// `build.bnd`, `bnd.build`, and their all-caps variants - the crate's own
// comment calls the latter out as a real, known toolchain quirk: "ACE fuck
// up by uppercasing files"), plus *any* other file simply ending in
// `.bnd`/`.build` (regular or `.BND`/`.BUILD`), matching real project
// layouts that don't use one of the 3 canonical stems (e.g. `linking.bnd`).
// VS Code's `findFiles` glob matching is case-sensitive on a case-sensitive
// filesystem and has no case-insensitive flag, so each case needs its own
// explicit alternative - `**/*.{bnd,build}` alone silently misses every
// all-caps file.
const BUILD_FILE_GLOB =
    '{**/*.bnd,**/*.BND,**/*.build,**/*.BUILD,**/bndbuild.yml,**/BNDBUILD.YML}';

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
            // The bndbuild half mirrors `BUILD_FILE_GLOB` (also covering the
            // all-caps `.BND`/`.BUILD`/`BNDBUILD.YML` variants
            // `cpclib_bndbuild::builder::EXPECTED_FILENAMES` explicitly
            // handles), so the LSP re-syncs on a build file's changes
            // regardless of which casing it was named with.
            fileEvents: workspace.createFileSystemWatcher(
                '{**/*.{asm,z80,bas,BAS,CAT,cat,ASC,asc},' +
                '**/*.bnd,**/*.BND,**/*.build,**/*.BUILD,**/bndbuild.yml,**/BNDBUILD.YML}'
            )
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
            // `cpclib.runRuleInTerminal`/`cpclib.runTaskInTerminal` (the
            // rule-level and per-task CodeLenses on a real on-disk .bnd
            // file) deliberately aren't listed here - they run as real VS
            // Code Tasks/terminals instead, which show themselves. This
            // middleware only covers the two commands that still stream
            // through the LSP's own output channel because there's no
            // on-disk file for a terminal Task to invoke:
            // `cpclib.runRule`/`cpclib.runTask`, both scoped to
            // embedded-bndbuild-in-.asm blocks. Missing `cpclib.runTask`
            // here used to be a real bug: its output *was* being logged,
            // just never shown, so it looked like nothing happened at all.
            // An embedded rule run via `Ctrl+Shift+B`/`EmbeddedRulePseudoterminal`
            // also goes through `cpclib.runRule` under the hood, so this
            // still fires then too (`show(true)` preserves focus, so it
            // doesn't steal it away from the task terminal) - the same
            // output is now visible in *both* places at once
            // (`installLogMessageMirror` mirrors it into the terminal live),
            // which is intentional redundancy, not a bug.
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
        installLogMessageMirror(client);
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

    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.runTaskInTerminal', runTaskInTerminal),
    );

    // These four IDs are deliberately *not* the server-side command names
    // (`cpclib.analyzePeephole`, `cpclib.analyzePeepholeWorkspace`,
    // `cpclib.clearPeephole`) they forward to. See the NOTE above about
    // `cpclib.runRule`: vscode-languageclient auto-registers a bridge command
    // for every name the server advertises in `executeCommandProvider`, so
    // registering the same name here throws "command already exists" and
    // aborts the whole client start - which surfaces to the user as
    // "Client is not running" the moment they invoke anything.
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.findPeepholeInFile', () => analyzePeephole('file')),
        vscode.commands.registerCommand('cpclib.findPeepholeInSelection', () => analyzePeephole('selection')),
        vscode.commands.registerCommand('cpclib.findPeepholeInProject', () => analyzePeephole('project')),
        vscode.commands.registerCommand('cpclib.clearPeepholeResults', clearPeephole),
        vscode.commands.registerCommand('cpclib.assembleThisFile', assembleActiveFile),
    );

    registerDebugging(context, () => resolvedServerPath);

    context.subscriptions.push(
        vscode.debug.onDidChangeBreakpoints(e => { void syncBreakpointDirectives(e); }),
        // The other direction: a file that already contains directives shows
        // its dots as soon as it is opened.
        vscode.workspace.onDidOpenTextDocument(doc => { void showExistingBreakpoints(doc); }),
    );
    for (const doc of vscode.workspace.textDocuments) {
        void showExistingBreakpoints(doc);
    }

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

// Every currently-open `EmbeddedRulePseudoterminal`'s writer, so
// `installLogMessageMirror`'s single `window/logMessage` handler can push
// each line into whichever embedded-rule task terminal(s) are running right
// now, live, as `cpclib.runRule` streams them - not just a "check the
// output channel instead" pointer message. A `Set` rather than a single
// slot: nothing stops two embedded-rule tasks from running concurrently
// (each click starts its own), and a message arriving while more than one
// is active has no server-side tag saying which task it belongs to, so it
// goes to all of them - a rare, cosmetic-only interleaving, not a
// correctness issue (the real, authoritative output is still also always in
// the "CPClib LSP" output channel).
const activeEmbeddedTaskWriters = new Set<(text: string) => void>();

/// Registers the *one* `window/logMessage` handler this extension ever
/// installs beyond vscode-languageclient's own built-in one - and
/// necessarily *replaces* it: `LanguageClient.onNotification`/the
/// underlying `vscode-jsonrpc` connection only support one handler per
/// notification method (a plain `Map.set` keyed by method name in both
/// layers - confirmed directly in `vscode-languageclient/lib/common/client.js`
/// and `vscode-jsonrpc/lib/common/connection.js`), so registering a second,
/// *additional* listener isn't possible - it would just silently steal the
/// slot from whichever registers first.
///
/// This handler therefore reimplements vscode-languageclient's own default
/// `window/logMessage` handling *exactly* (dispatching to `client.error`/
/// `warn`/`info`/`debug` for those message types, `outputChannel.appendLine`
/// directly for anything else - which is the path `MessageType.Log` (4)
/// takes, what the server actually uses for streamed build stdout/stderr;
/// see `vscode-languageclient/lib/common/client.js`'s own `doStart` for the
/// original this must stay byte-for-byte in sync with) so nothing regresses
/// for any other feature that relies on server log messages reaching the
/// output channel - then additionally mirrors the same line into every
/// currently-active embedded-rule task terminal.
///
/// Called from `client.start().then(...)`, not synchronously in `activate()`:
/// vscode-languageclient installs its *own* built-in handler directly on the
/// connection during `doStart()`, so registering ours only after `start()`
/// resolves guarantees the connection already exists and our call is the
/// one that ends up owning the method's single handler slot.
function installLogMessageMirror(client: LanguageClient): void {
    client.onNotification('window/logMessage', (message: { type: number; message: string }) => {
        switch (message.type) {
            case 1: client.error(message.message, undefined, false); break;   // MessageType.Error
            case 2: client.warn(message.message, undefined, false); break;    // MessageType.Warning
            case 3: client.info(message.message, undefined, false); break;    // MessageType.Info
            case 5: client.debug(message.message, undefined, false); break;   // MessageType.Debug
            default: client.outputChannel.appendLine(message.message);        // MessageType.Log (4), and anything else
        }
        for (const write of activeEmbeddedTaskWriters) {
            write(message.message + '\r\n');
        }
    });
}

/// A `vscode.Pseudoterminal` wrapping the LSP's own `cpclib.runRule` command
/// - the execution mechanism for a `#!bndbuild`-embedded rule in a `.asm`
/// file, which (unlike a real `.bnd` file) has no on-disk YAML file a
/// `ShellExecution` could target, so it can't become a real terminal Task
/// the way `buildBndbuildTask`'s tasks are. While `cpclib.runRule` runs,
/// this terminal receives a live mirror of the same build output the
/// "CPClib LSP" output channel gets (via `installLogMessageMirror`), so an
/// embedded rule's task terminal behaves like a real one rather than a bare
/// "see elsewhere" pointer.
class EmbeddedRulePseudoterminal implements vscode.Pseudoterminal {
    private readonly writeEmitter = new vscode.EventEmitter<string>();
    private readonly closeEmitter = new vscode.EventEmitter<number>();
    onDidWrite: vscode.Event<string> = this.writeEmitter.event;
    onDidClose: vscode.Event<number> = this.closeEmitter.event;

    constructor(
        private readonly rule: string,
        private readonly hostFilePath: string,
    ) {}

    async open(): Promise<void> {
        const write = (text: string) => this.writeEmitter.fire(text);
        activeEmbeddedTaskWriters.add(write);
        this.writeEmitter.fire(`Running embedded bndbuild rule '${this.rule}' from ${this.hostFilePath}\r\n`);
        try {
            await vscode.commands.executeCommand('cpclib.runRule', this.rule, this.hostFilePath);
            this.closeEmitter.fire(0);
        } catch (err) {
            this.writeEmitter.fire(`Failed to run: ${err}\r\n`);
            this.closeEmitter.fire(1);
        } finally {
            activeEmbeddedTaskWriters.delete(write);
        }
    }

    close(): void {}
}

/// Builds the `vscode.Task` for a `#!bndbuild`-embedded rule - see
/// `EmbeddedRulePseudoterminal`'s own doc comment for why this needs a
/// `CustomExecution` instead of `buildBndbuildTask`'s `ShellExecution`.
function buildEmbeddedRuleTask(rule: string, hostFilePath: string, taskName: string): vscode.Task {
    const def: vscode.TaskDefinition = {
        type: BndbuildTaskProvider.taskType,
        target: rule,
        file: hostFilePath,
        embedded: true,
    };
    const execution = new vscode.CustomExecution(
        async () => new EmbeddedRulePseudoterminal(rule, hostFilePath),
    );
    const task = new vscode.Task(def, vscode.TaskScope.Workspace, taskName, 'bndbuild', execution);
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
        // Logged unconditionally (not just on error) - "Ctrl+Shift+B finds
        // no bndbuild tasks" has too many possible causes (provider never
        // invoked at all, glob matched nothing, every getTargets call
        // failed, every file legitimately has zero targets) to tell apart
        // from silence alone. Check the "CPClib LSP" output channel after
        // pressing Ctrl+Shift+B - if this line isn't even there, the
        // provider itself was never asked (an activation problem, not this
        // function); if it's there with 0 build files, the glob or workspace
        // folder is the problem; if files but 0 targets each, `getTargets`
        // or the LSP connection is the problem.
        this.lspClient.outputChannel.appendLine(
            `[bndbuild task provider] provideTasks() invoked, workspace folders: ${
                vscode.workspace.workspaceFolders?.map(f => f.uri.fsPath).join(', ') ?? '(none)'
            }`,
        );

        const buildFiles = await vscode.workspace.findFiles(
            BUILD_FILE_GLOB,
            '{**/node_modules/**,**/.git/**,**/target/**}',
        );
        this.lspClient.outputChannel.appendLine(
            `[bndbuild task provider] found ${buildFiles.length} build file(s): ${
                buildFiles.map(f => f.fsPath).join(', ')
            }`,
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
            this.lspClient.outputChannel.appendLine(
                `[bndbuild task provider] ${fileUri.fsPath}: ${targets.length} target(s): ${targets.join(', ')}`,
            );

            const filePath = fileUri.fsPath;
            // Workspace-relative path, not just the bare filename - two
            // build files in different directories can both declare a
            // target of the same name (e.g. two `build.bnd`s each with a
            // `test` rule), and the bare filename alone doesn't disambiguate
            // them in the Ctrl+Shift+B picker. Default `includeWorkspaceFolder`
            // (omitted, not forced false) also prefixes the root folder name
            // in a multi-root workspace, where even the relative path alone
            // could still collide across roots.
            const relativePath = vscode.workspace.asRelativePath(fileUri);

            for (const target of targets) {
                const taskName = buildFiles.length > 1 ? `${target} (${relativePath})` : target;
                tasks.push(buildBndbuildTask(target, filePath, bndbuildCommand, taskName));
            }
        }

        // Rules embedded in `.asm` files (`#!bndbuild` comment blocks) -
        // these have no on-disk YAML file to `findFiles`/`getTargets` scan
        // for, so they're discovered separately: one fast, single request
        // to the already-maintained server-side index
        // (`cpclib.getEmbeddedBndbuildFiles`, see its own Rust-side doc
        // comment for why this is instant rather than a fresh workspace
        // scan - critical here, since scanning every `.asm` file's content
        // client-side to look for the marker would make every
        // `Ctrl+Shift+B` press noticeably slow on a real project).
        try {
            const embeddedFiles = await this.lspClient.sendRequest<{ uri: string; targets: string[] }[]>(
                'workspace/executeCommand',
                { command: 'cpclib.getEmbeddedBndbuildFiles', arguments: [] },
            ) ?? [];
            this.lspClient.outputChannel.appendLine(
                `[bndbuild task provider] ${embeddedFiles.length} .asm file(s) known to have embedded rules`,
            );
            for (const { uri, targets } of embeddedFiles) {
                const fileUri = vscode.Uri.parse(uri);
                const filePath = fileUri.fsPath;
                const relativePath = vscode.workspace.asRelativePath(fileUri);
                for (const target of targets) {
                    tasks.push(
                        buildEmbeddedRuleTask(target, filePath, `${target} (${relativePath}, embedded)`),
                    );
                }
            }
        } catch (err) {
            this.lspClient.outputChannel.appendLine(
                `[bndbuild task provider] cpclib.getEmbeddedBndbuildFiles failed: ${err}`,
            );
        }

        this.lspClient.outputChannel.appendLine(
            `[bndbuild task provider] returning ${tasks.length} task(s)`,
        );
        return tasks;
    }

    resolveTask(task: vscode.Task): vscode.Task | undefined {
        const def = task.definition;
        if (def.type !== BndbuildTaskProvider.taskType || !def.target) {
            return undefined;
        }
        const filePath = def.file as string | undefined;
        if (!filePath) {
            return undefined;
        }
        if (def.embedded) {
            return buildEmbeddedRuleTask(def.target as string, filePath, task.name);
        }
        const bndbuildCommand = bndbuildCommandPrefix(this.config);
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
// unreliable at making its own diagnostics clickable.
async function runRuleInTerminal(target: string, filePath: string): Promise<void> {
    const config = workspace.getConfiguration('cpclib-lsp');
    const bndbuildCommand = bndbuildCommandPrefix(config);
    const task = buildBndbuildTask(target, filePath, bndbuildCommand, target);
    await vscode.tasks.executeTask(task);
}

/// Builds the `vscode.Task` for `cpclib.runTaskInTerminal` - `--only-task
/// RULE:INDEX` (`cpclib-bndbuild`'s `BndBuilder::execute_task`) runs just
/// that one task, with the *same* Jinja/automatic-variable context a normal
/// rule build gets (unlike `--direct`, which runs a raw, unexpanded command
/// string - see that method's own doc comment for why that distinction
/// matters), and bypasses dependency resolution/up-to-date checks entirely,
/// so it runs even when the rule's target already exists.
function buildBndbuildOnlyTaskTask(
    rule: string,
    filePath: string,
    taskIndex: number,
    bndbuildCommand: string,
    taskName: string,
): vscode.Task {
    const workDir  = path.dirname(filePath);
    const fileName = path.basename(filePath);
    const def: vscode.TaskDefinition = {
        type: BndbuildTaskProvider.taskType,
        target: rule,
        file: filePath,
    };
    const task = new vscode.Task(
        def,
        vscode.TaskScope.Workspace,
        taskName,
        'bndbuild',
        new vscode.ShellExecution(
            `${bndbuildCommand} -f "${fileName}" --only-task "${rule}:${taskIndex}"`,
            { cwd: workDir },
        ),
        '$basm',
    );
    task.group = vscode.TaskGroup.Build;
    return task;
}

// `cpclib.runTaskInTerminal(rule, filePath, taskIndex)`: the per-command
// "▶ Run this command" CodeLens's terminal-based counterpart to
// `cpclib.runRuleInTerminal`, for a rule in a real on-disk .bnd file - uses
// the very same mechanism (a real Task/terminal, `$basm` problemMatcher) as
// the rule-level runner, per the same reasoning: the LSP's own
// `cpclib.runTask` streaming path doesn't surface clickable errors as
// reliably as a real terminal does. The embedded-bndbuild-in-.asm-block
// CodeLens (no on-disk .bnd file for a CLI invocation to target) still uses
// the LSP path - there's no terminal equivalent for that case.
async function runTaskInTerminal(rule: string, filePath: string, taskIndex: number): Promise<void> {
    const config = workspace.getConfiguration('cpclib-lsp');
    const bndbuildCommand = bndbuildCommandPrefix(config);
    const task = buildBndbuildOnlyTaskTask(rule, filePath, taskIndex, bndbuildCommand, `${rule} #${taskIndex + 1}`);
    await vscode.tasks.executeTask(task);
}

// ── Build the active .asm file's own bndbuild target(s) ────────────────────
//
// "cpclib.buildActiveFile": finds every bndbuild file in the workspace that
// references the currently active .asm file (as a dependency or as one of
// its own targets, via the server-side `cpclib.getTargetsForFile`
// reverse-lookup) and runs the one match directly, or offers a QuickPick
// when several build files/targets reference the same source file.

/// Ask the server for peephole-optimization suggestions.
///
/// The server's automatic pass is off by default - deciding whether a `jp`
/// reaches as a `jr` needs the addresses a real build produces, which means
/// assembling the whole project, and that is not a keystroke-time cost. These
/// commands are the explicit request that turns it on for the file (or the
/// project) the user actually cares about. Results arrive as ordinary
/// diagnostics in the Problems panel, and stay live as the file is edited
/// until `cpclib.clearPeepholeResults`.
///
/// The project sweep is driven from here, one server request per file, rather
/// than being a single server-side command. Analysing a file is *seconds* of
/// real assembling, so a project of any size is minutes and a workspace
/// holding several demos is much worse - which is exactly what the first
/// version did, with no count, no progress and no way to stop it. Owning the
/// loop here is what makes those three possible.
async function analyzePeephole(scope: 'file' | 'selection' | 'project'): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor) {
        window.showInformationMessage('Open an assembly file first.');
        return;
    }
    if (!/\.(asm|z80)$/i.test(editor.document.uri.fsPath)) {
        window.showInformationMessage('The active file is not an assembly file.');
        return;
    }

    let selection: unknown | undefined;
    if (scope === 'selection') {
        if (editor.selection.isEmpty) {
            window.showInformationMessage('Select the code to analyze first.');
            return;
        }
        // The server narrows only what it *reports*: the analysis itself
        // always covers the whole file, because whether a rewrite is safe
        // depends on the code surrounding it.
        selection = {
            start: { line: editor.selection.start.line, character: editor.selection.start.character },
            end: { line: editor.selection.end.line, character: editor.selection.end.character },
        };
    }

    let targets: vscode.Uri[] = [editor.document.uri];
    if (scope === 'project') {
        // Scoped to the folder the active file belongs to, not to every
        // workspace folder: a checkout that holds several demos would
        // otherwise sweep all of them.
        const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri);
        if (!folder) {
            window.showInformationMessage('This file is not inside a workspace folder.');
            return;
        }
        targets = await vscode.workspace.findFiles(
            new vscode.RelativePattern(folder, '**/*.{asm,z80}'),
            '{**/node_modules/**,**/.git/**,**/target/**}',
        );
        if (targets.length === 0) {
            window.showInformationMessage('No assembly files found in this folder.');
            return;
        }
        // Each file costs a real assemble. Say how many before spending them.
        const go = await window.showWarningMessage(
            `Analyze ${targets.length} file${targets.length === 1 ? '' : 's'} in ${folder.name}? `
            + 'Each one is assembled, so this can take several minutes.',
            { modal: true },
            'Analyze',
        );
        if (go !== 'Analyze') {
            return;
        }
    }

    await window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: 'Looking for peephole optimizations',
            cancellable: true,
        },
        async (progress, token) => {
            // A handful in flight at once: the server answers requests
            // concurrently, so this is where the parallelism comes from, while
            // the bound keeps a large project from queueing hundreds of
            // assembles at once.
            const CONCURRENCY = 8;
            let done = 0;
            let cancelled = false;
            const findings: PeepholeFinding[] = [];

            for (let i = 0; i < targets.length; i += CONCURRENCY) {
                if (token.isCancellationRequested) {
                    cancelled = true;
                    break;
                }
                const batch = targets.slice(i, i + CONCURRENCY);
                const perFile = await Promise.all(batch.map(async uri => {
                    try {
                        return await client.sendRequest<PeepholeFinding[] | null>(
                            'workspace/executeCommand',
                            {
                                command: 'cpclib.analyzePeephole',
                                arguments: selection ? [uri.toString(), selection] : [uri.toString()],
                            },
                        ) ?? [];
                    } catch {
                        // One unreadable or unparseable file must not abandon
                        // the sweep.
                        return [];
                    }
                }));
                for (const list of perFile) {
                    findings.push(...list);
                }
                done += batch.length;
                progress.report({
                    increment: (batch.length / targets.length) * 100,
                    message: `${done}/${targets.length} files, ${findings.length} found`,
                });
            }

            const where = targets.length === 1 ? 'this file' : `${done} files`;
            await reportFindings(findings, where, cancelled);
        },
    );
}

/// One suggestion, as the server reports it.
type PeepholeFinding = {
    uri: string;
    line: number;
    character: number;
    message: string;
};

/// `file.asm:42`, the form that is worth reading in a notification.
function findingLabel(finding: PeepholeFinding): string {
    const name = vscode.Uri.parse(finding.uri).path.split('/').pop() ?? finding.uri;
    return `${name}:${finding.line + 1}`;
}

/// Tell the user what was found, and let them get there.
///
/// A bare count is not actionable - it leaves the reader to go hunting for
/// something the analysis already knows the exact position of. So: the
/// locations go in the message itself while they still fit, the full list is
/// always one click away as a jumpable quick pick, and every one of them is
/// written to the output channel where it can be read back later.
async function reportFindings(
    findings: PeepholeFinding[],
    where: string,
    cancelled: boolean,
): Promise<void> {
    if (findings.length > 0) {
        client.outputChannel.appendLine(`Peephole optimizations in ${where}:`);
        for (const finding of findings) {
            client.outputChannel.appendLine(`  ${findingLabel(finding)}  ${finding.message}`);
        }
    }

    if (findings.length === 0) {
        window.showInformationMessage(
            cancelled
                ? `Peephole analysis stopped after ${where}: nothing found so far.`
                : `No peephole optimizations found in ${where}.`);
        return;
    }

    const count = `${findings.length} peephole optimization${findings.length === 1 ? '' : 's'}`;
    // Short lists read better inline than behind a click; long ones would turn
    // the notification into a wall.
    const inline = findings.length <= 3
        ? ` (${findings.map(findingLabel).join(', ')})`
        : '';
    const summary = cancelled
        ? `Peephole analysis stopped after ${where}: ${count} found so far${inline}.`
        : `${count} found in ${where}${inline}.`;

    const choice = await window.showInformationMessage(summary, 'Go to…');
    if (choice !== 'Go to…') {
        return;
    }
    await pickFinding(findings);
}

/// A jumpable list of findings, previewing each one as it is highlighted.
async function pickFinding(findings: PeepholeFinding[]): Promise<void> {
    type Item = vscode.QuickPickItem & { finding: PeepholeFinding };
    const items: Item[] = findings.map(finding => ({
        label: findingLabel(finding),
        description: finding.message,
        finding,
    }));

    const reveal = async (finding: PeepholeFinding) => {
        const document = await workspace.openTextDocument(vscode.Uri.parse(finding.uri));
        const position = new vscode.Position(finding.line, finding.character);
        await window.showTextDocument(document, {
            selection: new vscode.Range(position, position),
            preview: true,
        });
    };

    const picker = window.createQuickPick<Item>();
    picker.items = items;
    picker.placeholder = 'Peephole optimizations';
    picker.matchOnDescription = true;
    picker.onDidChangeActive(active => {
        if (active[0]) {
            void reveal(active[0].finding);
        }
    });
    picker.onDidAccept(() => {
        picker.hide();
    });
    picker.onDidHide(() => picker.dispose());
    picker.show();
}

/// Assemble the active `.asm` file with `basm` and show its errors.
///
/// The build-driven diagnostics only cover what a bndbuild *rule* names, and
/// most sources in a demo are not a rule's target - so nothing ever reports an
/// error in them until the whole project is built. This is the explicit "try
/// assembling this one" answer.
///
/// Extra arguments are asked for rather than guessed: a file that is part of a
/// larger program usually needs the same `-D`/`-I` flags the real build passes,
/// and only the user knows which. The last answer is offered again next time,
/// since it rarely changes.
let lastAssembleArguments = '';

async function assembleActiveFile(): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor) {
        window.showInformationMessage('Open an assembly file first.');
        return;
    }
    if (!/\.(asm|z80)$/i.test(editor.document.uri.fsPath)) {
        window.showInformationMessage('The active file is not an assembly file.');
        return;
    }
    if (editor.document.isDirty) {
        // basm reads the file from disk, so unsaved edits would be assembled
        // as they were, not as they look.
        await editor.document.save();
    }

    const args = await window.showInputBox({
        title: 'Assemble with basm',
        prompt: 'Extra basm arguments (optional) - e.g. -DMUSIC=1 --snapshot',
        placeHolder: 'leave empty for none',
        value: lastAssembleArguments,
        ignoreFocusOut: true,
    });
    // Escape (undefined) cancels; an empty string is a real answer.
    if (args === undefined) {
        return;
    }
    lastAssembleArguments = args;

    await window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: `Assembling ${editor.document.uri.path.split('/').pop()}…`,
            cancellable: false,
        },
        async () => {
            try {
                await client.sendRequest('workspace/executeCommand', {
                    command: 'cpclib.assembleFile',
                    arguments: [editor.document.uri.toString(), args],
                });
            } catch (err) {
                window.showErrorMessage(`Could not assemble: ${(err as Error).message}`);
            }
        },
    );
}

/// Stop reporting peephole optimizations - for the active file, or everywhere
/// when no assembly file is active.
async function clearPeephole(): Promise<void> {
    const editor = window.activeTextEditor;
    const uri = editor && /\.(asm|z80)$/i.test(editor.document.uri.fsPath)
        ? editor.document.uri.toString()
        : undefined;
    try {
        await client.sendRequest('workspace/executeCommand', {
            command: 'cpclib.clearPeephole',
            arguments: uri ? [uri] : [],
        });
    } catch (err) {
        window.showErrorMessage(`Could not clear the peephole results: ${(err as Error).message}`);
    }
}

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
        BUILD_FILE_GLOB,
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
            // Workspace-relative path, not just the bare filename - same
            // ambiguity risk as `BndbuildTaskProvider.provideTasks`' own
            // task naming when several build files share a target name.
            description: vscode.workspace.asRelativePath(m.buildFileUri),
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


// ---------------------------------------------------------------------------
// Breakpoints
// ---------------------------------------------------------------------------

/**
 * Mirror VS Code's breakpoints into `breakpoint` directives in the source.
 *
 * There is no debug adapter here, and writing one would not help: the red dot
 * has to become an *address* for an emulator to stop on, and every emulator
 * takes that list differently - when it takes one at all. basm already has a
 * `breakpoint` directive that travels into the snapshot, so the shortest path
 * from the dot to a stopped emulator is to put the directive in the file.
 *
 * Which means toggling a breakpoint edits the source. That is a real
 * side-effect and not everyone will want it, hence
 * `cpclib.breakpointDirective` - but it is on by default, since a red dot that
 * does nothing at all is worse.
 *
 * The server decides *where* on the line the directive goes: in basm, telling a
 * label from a mnemonic needs a parse, not a regex.
 */
async function syncBreakpointDirectives(event: vscode.BreakpointsChangeEvent): Promise<void> {
    // During a debug session a red dot is a *live* breakpoint: VS Code sends it
    // to the adapter, which sets it in the emulator. Writing the directive as
    // well would edit the user's source behind their back and desynchronise the
    // two - so the writer stands down for the duration.
    if (isDebugSessionActive()) {
        return;
    }
    if (!vscode.workspace.getConfiguration('cpclib').get<boolean>('breakpointDirective', true)) {
        return;
    }
    if (!client) {
        return;
    }
    // `showExistingBreakpoints` adds breakpoints, which fires this event right
    // back. Nothing breaks if it runs - the server declines to insert a second
    // directive on a line that already has one - but the round trip is pure
    // waste, so the flag skips it outright.
    if (adoptingExistingBreakpoints) {
        return;
    }

    const wanted: { uri: vscode.Uri; line: number; enable: boolean }[] = [];
    const collect = (breakpoints: readonly vscode.Breakpoint[], enable: boolean) => {
        for (const breakpoint of breakpoints) {
            if (!(breakpoint instanceof vscode.SourceBreakpoint)) { continue; }
            const uri = breakpoint.location.uri;
            if (!uri.fsPath.toLowerCase().endsWith('.asm')) { continue; }
            wanted.push({ uri, line: breakpoint.location.range.start.line, enable });
        }
    };
    collect(event.added, true);
    collect(event.removed, false);
    // A changed breakpoint may have moved: VS Code reports the new location
    // only, so the directive is (re)placed there and any stale one elsewhere
    // is left for the user - guessing at the old line would be worse than
    // leaving a directive they can see.
    collect(event.changed, true);

    if (wanted.length === 0) {
        return;
    }

    const edit = new vscode.WorkspaceEdit();
    for (const { uri, line, enable } of wanted) {
        try {
            // The document must be open for the server to have parsed it.
            await vscode.workspace.openTextDocument(uri);
            const textEdit = await client.sendRequest<{
                range: { start: { line: number; character: number }; end: { line: number; character: number } };
                newText: string;
            } | null>(
                'workspace/executeCommand',
                { command: 'cpclib.breakpointEdit', arguments: [uri.toString(), line, enable] },
            );
            if (!textEdit) { continue; }
            edit.replace(
                uri,
                new vscode.Range(
                    textEdit.range.start.line, textEdit.range.start.character,
                    textEdit.range.end.line, textEdit.range.end.character,
                ),
                textEdit.newText,
            );
        } catch (err) {
            client.outputChannel.appendLine(
                `[breakpoints] cpclib.breakpointEdit failed for ${uri.fsPath}:${line + 1}: ${err}`,
            );
        }
    }

    if (edit.size > 0) {
        await vscode.workspace.applyEdit(edit);
    }
}


/**
 * Show a red dot for every `breakpoint` directive already written in a file.
 *
 * Without this the mapping only runs one way: a directive committed last week,
 * or typed by hand, would sit in the source with nothing in the gutter to say
 * so - and clicking that line would then try to add a *second* one.
 *
 * The server finds them, because a directive is not always the first thing on
 * its line (`ld a,0 : BREAKPOINT` is as valid as the other order) and knowing
 * that means parsing, not scanning for a word.
 */
let adoptingExistingBreakpoints = false;

async function showExistingBreakpoints(document: vscode.TextDocument): Promise<void> {
    if (!vscode.workspace.getConfiguration('cpclib').get<boolean>('breakpointDirective', true)) {
        return;
    }
    if (!client || document.languageId !== 'basm') {
        return;
    }

    let lines: number[] = [];
    try {
        lines = await client.sendRequest<number[]>(
            'workspace/executeCommand',
            { command: 'cpclib.breakpointLines', arguments: [document.uri.toString()] },
        ) ?? [];
    } catch (err) {
        client.outputChannel.appendLine(
            `[breakpoints] cpclib.breakpointLines failed for ${document.uri.fsPath}: ${err}`,
        );
        return;
    }
    if (lines.length === 0) {
        return;
    }

    // Only the ones the editor does not already know about, so reopening a
    // file does not pile duplicates onto the same line.
    const known = new Set(
        vscode.debug.breakpoints
            .filter((b): b is vscode.SourceBreakpoint => b instanceof vscode.SourceBreakpoint)
            .filter(b => b.location.uri.toString() === document.uri.toString())
            .map(b => b.location.range.start.line),
    );
    const missing = lines.filter(line => !known.has(line));
    if (missing.length === 0) {
        return;
    }

    adoptingExistingBreakpoints = true;
    try {
        vscode.debug.addBreakpoints(missing.map(line => new vscode.SourceBreakpoint(
            new vscode.Location(document.uri, new vscode.Position(line, 0)),
        )));
    } finally {
        adoptingExistingBreakpoints = false;
    }
}
