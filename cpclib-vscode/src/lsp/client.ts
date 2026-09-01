import * as vscode from 'vscode';
import { workspace, ExtensionContext } from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

/**
 * The single `LanguageClient` instance, assigned synchronously by
 * {@link createLanguageClient} before `activate()` returns - every command
 * module that needs to talk to the server imports this live binding directly
 * rather than having the client threaded through as a parameter.
 */
export let client: LanguageClient;

/**
 * Timestamped activation-timing log, shared by `extension.ts` and this file -
 * created once here (before the `LanguageClient` itself exists, so
 * `activate()`'s very first line can log to it) and handed to
 * `LanguageClientOptions.outputChannel` below so the client reuses this
 * channel instead of creating a second "CPClib LSP" one.
 *
 * Exists to answer a real, otherwise-unanswerable question: a report of
 * "cpclib-lsp itself responds instantly when driven directly over stdio,
 * but a real VS Code session still has a ~40s stall before the workspace's
 * previously-open tabs get re-opened" - i.e. something client-side, not
 * server-side. `createFileSystemWatcher`'s broad, recursive,
 * workspace-wide pattern (below) is the leading suspect - broad recursive
 * watchers are a well-documented category of VS Code startup slowness -
 * but this is the only way to actually confirm or rule that out on a
 * remote machine nobody can attach an interactive debugger to.
 */
export const startupLog = vscode.window.createOutputChannel('CPClib LSP');

export function logStartupTiming(label: string): void {
    startupLog.appendLine(`[startup ${new Date().toISOString()}] ${label}`);
}

/**
 * The resolved `cpclib-lsp` binary path, set once by
 * {@link createLanguageClient} - reused by `bndbuildCommandPrefix` so
 * bndbuild execution (Tasks, the "▶ Run" CodeLens) invokes the *same* binary
 * as the language server itself, running it as `cpclib-lsp bndbuild ...`
 * instead of requiring a second `bndbuild` binary on PATH (`cpclib-lsp`
 * already links `cpclib-bndbuild` in full for its own
 * `cpclib.runRule`/`cpclib.runTask` LSP commands - this reuses that same
 * code as a CLI entry point too, see `cpclib-lsp/src/main.rs`'s
 * `run_as_bndbuild`).
 */
export let resolvedServerPath: string;

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
export function resolveServerPath(configured: string, extensionPath: string): string {
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

/**
 * Resolves the server binary, builds the `LanguageClient` (server + client
 * options, including the `window/logMessage`-revealing middleware) and
 * assigns it to the module-level {@link client}/{@link resolvedServerPath}
 * bindings. Does not call `client.start()` - that, plus the cross-cutting
 * side effects that only make sense once it resolves (the "server started"
 * notification, wiring the debug module's own client reference, installing
 * the log-message mirror), stays in `extension.ts`'s `activate()`.
 */
export function createLanguageClient(
    context: ExtensionContext,
    config: vscode.WorkspaceConfiguration,
): LanguageClient {
    logStartupTiming('createLanguageClient: start');
    const serverPath = resolveServerPath(
        config.get<string>('serverPath', 'cpclib-lsp'),
        context.extensionPath
    );
    resolvedServerPath = serverPath;
    logStartupTiming(`createLanguageClient: server path resolved to ${serverPath}`);

    // `cwd` matters beyond convention: the server looks for `cpclib-lsp.toml`
    // relative to it at startup (before `initialize` gives it a workspace
    // root over the protocol) to decide where to write its own trace log -
    // see `LspConfig::log`. Mirrors the same `cwd` already set for the DAP in
    // `debug/debugAdapterFactory.ts`.
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    const serverOptions: ServerOptions = {
        run: { command: serverPath, transport: TransportKind.stdio, options: { cwd: workspaceRoot } },
        debug: { command: serverPath, transport: TransportKind.stdio, options: { cwd: workspaceRoot } }
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
            //
            // Timed on both sides: this is a broad, recursive, workspace-wide
            // watcher - VS Code has to set up a native recursive watch over
            // the whole directory tree to support it, not just the matched
            // file types, which is a well-documented category of VS Code
            // startup slowness on a large/asset-heavy tree. `createFileSystemWatcher`
            // itself returns synchronously (the native watcher setup happens
            // in the background), so this pair of log lines won't show a long
            // gap even if that background setup is what's actually slow -
            // but it does at least prove or disprove that *this call itself*
            // isn't a synchronous stall, and marks the one place in this
            // file's own startup path most likely to matter if it is slow.
            fileEvents: (() => {
                logStartupTiming('createFileSystemWatcher: start (broad, recursive, workspace-wide)');
                const watcher = workspace.createFileSystemWatcher(
                    '{**/*.{asm,z80,bas,BAS,CAT,cat,ASC,asc},' +
                    '**/*.bnd,**/*.BND,**/*.build,**/*.BUILD,**/bndbuild.yml,**/BNDBUILD.YML}'
                );
                logStartupTiming('createFileSystemWatcher: call returned');
                return watcher;
            })()
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
            },
            // Timed for the same reason as `createFileSystemWatcher` above:
            // this fires the moment VS Code itself hands this document to
            // the client to forward as `textDocument/didOpen` - correlating
            // this timestamp against the server's own "Document opened" log
            // line for the same URI is what actually tells apart "the client
            // was slow to forward a document VS Code already had open" from
            // "VS Code itself hadn't opened the document yet" - the two
            // explanations this instrumentation exists to distinguish.
            didOpen: (document, next) => {
                logStartupTiming(`didOpen middleware: ${document.uri.toString()}`);
                return next(document);
            }
        },
        // Reuses `startupLog` rather than letting `LanguageClient` create its
        // own "CPClib LSP" channel - same name, same channel, so the startup
        // timing lines above and the client's own request/response tracing
        // land together instead of in two separately-named channels.
        outputChannel: startupLog
    };

    client = new LanguageClient(
        'cpclib-lsp',
        'CPClib LSP',
        serverOptions,
        clientOptions
    );
    logStartupTiming('createLanguageClient: LanguageClient constructed, about to return (not started yet)');

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

    return client;
}
