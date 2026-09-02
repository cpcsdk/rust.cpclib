import * as vscode from 'vscode';
import * as path from 'path';
import { LanguageClient } from 'vscode-languageclient/node';
import { resolvedServerPath } from '../lsp/client';
import { buildEmbeddedRuleTask } from './codeLensRunners';

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
export const BUILD_FILE_GLOB =
    '{**/*.bnd,**/*.BND,**/*.build,**/*.BUILD,**/bndbuild.yml,**/BNDBUILD.YML}';

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
export function bndbuildCommandPrefix(config: vscode.WorkspaceConfiguration): string {
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
export function buildBndbuildTask(
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

export class BndbuildTaskProvider implements vscode.TaskProvider {
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

        // One `getTargets` round-trip per build file, all in flight at
        // once instead of one at a time - with N build files, awaiting
        // them sequentially means N * round-trip-time instead of
        // max(round-trip-time), on every Ctrl+Shift+B press. Each file's
        // own try/catch and logging stay exactly as before, so one file's
        // failure still doesn't affect the others.
        const perFileTargets = await Promise.all(buildFiles.map(async fileUri => {
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
            return { fileUri, targets };
        }));

        for (const { fileUri, targets } of perFileTargets) {
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
