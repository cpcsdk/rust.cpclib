import * as vscode from 'vscode';
import { workspace, ExtensionContext, window, commands, Terminal } from 'vscode';
import * as path from 'path';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;
let buildTerminal: Terminal | undefined;

function getOrCreateTerminal(): Terminal {
    if (!buildTerminal || buildTerminal.exitStatus !== undefined) {
        buildTerminal = window.createTerminal('CPClib Build');
    }
    return buildTerminal;
}

export function activate(context: ExtensionContext) {
    const config = workspace.getConfiguration('cpclib-lsp');
    const serverPath = config.get<string>('serverPath', 'cpclib-lsp');

    const serverOptions: ServerOptions = {
        run: { command: serverPath, transport: TransportKind.stdio },
        debug: { command: serverPath, transport: TransportKind.stdio }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'basm' },
            { scheme: 'file', language: 'bndbuild' }
        ],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/*.{asm,z80,build,bnd}')
        }
    };

    client = new LanguageClient(
        'cpclib-lsp',
        'CPClib LSP',
        serverOptions,
        clientOptions
    );

    // Register the "Run Rule" command invoked by CodeLens buttons in bndbuild files.
    // Arguments: target (string), buildFilePath (string, absolute path to the .bnd/.build file).
    const runRuleCmd = commands.registerCommand('cpclib.runRule', (target: string, buildFilePath?: string) => {
        const terminal = getOrCreateTerminal();
        terminal.show(true); // preserve focus

        const bndbuildPath = config.get<string>('bndbuildPath', 'bndbuild');

        // Prefer the file path passed by the code lens; fall back to the active editor.
        const filePath = buildFilePath
            ?? (window.activeTextEditor?.document.uri.scheme === 'file'
                ? window.activeTextEditor.document.uri.fsPath
                : undefined);

        if (filePath) {
            const workDir  = path.dirname(filePath);
            const fileName = path.basename(filePath);
            terminal.sendText(`cd "${workDir}" && ${bndbuildPath} -f "${fileName}" ${target}`);
        } else {
            terminal.sendText(`${bndbuildPath} ${target}`);
        }
    });

    context.subscriptions.push(runRuleCmd);

    // Clean up terminal on extension deactivation
    context.subscriptions.push(
        window.onDidCloseTerminal(t => {
            if (t === buildTerminal) {
                buildTerminal = undefined;
            }
        })
    );

    client.start().then(() => {
        window.showInformationMessage('CPClib LSP server started.');

        // Register the task provider once the LSP is ready so it can query targets.
        const taskProvider = vscode.tasks.registerTaskProvider(
            BndbuildTaskProvider.taskType,
            new BndbuildTaskProvider(client, config),
        );
        context.subscriptions.push(taskProvider);
    }).catch((err: Error) => {
        window.showErrorMessage(`CPClib LSP failed to start: ${err.message}. Check cpclib-lsp.serverPath setting.`);
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

// ── Task provider ─────────────────────────────────────────────────────────────

class BndbuildTaskProvider implements vscode.TaskProvider {
    static readonly taskType = 'bndbuild';

    constructor(
        private readonly lspClient: LanguageClient,
        private readonly config: vscode.WorkspaceConfiguration,
    ) {}

    async provideTasks(_token: vscode.CancellationToken): Promise<vscode.Task[]> {
        const buildFiles = await vscode.workspace.findFiles(
            '**/*.{bnd,build}',
            '{**/node_modules/**,**/.git/**,**/target/**}',
        );

        const bndbuildPath = this.config.get<string>('bndbuildPath', 'bndbuild');
        const tasks: vscode.Task[] = [];

        for (const fileUri of buildFiles) {
            let targets: string[] = [];
            try {
                targets = await this.lspClient.sendRequest<string[]>(
                    'workspace/executeCommand',
                    { command: 'cpclib.getTargets', arguments: [fileUri.toString()] },
                ) ?? [];
            } catch {
                // LSP not ready or file unreadable — skip
            }

            const filePath  = fileUri.fsPath;
            const workDir   = path.dirname(filePath);
            const fileName  = path.basename(filePath);

            for (const target of targets) {
                const def: vscode.TaskDefinition = {
                    type: BndbuildTaskProvider.taskType,
                    target,
                    file: filePath,
                };
                const task = new vscode.Task(
                    def,
                    vscode.TaskScope.Workspace,
                    buildFiles.length > 1 ? `${target} (${fileName})` : target,
                    'bndbuild',
                    new vscode.ShellExecution(
                        `${bndbuildPath} -f "${fileName}" ${target}`,
                        { cwd: workDir },
                    ),
                );
                task.group = vscode.TaskGroup.Build;
                tasks.push(task);
            }
        }

        return tasks;
    }

    resolveTask(task: vscode.Task): vscode.Task | undefined {
        const def = task.definition;
        if (def.type !== BndbuildTaskProvider.taskType || !def.target) {
            return undefined;
        }
        const bndbuildPath = this.config.get<string>('bndbuildPath', 'bndbuild');
        const filePath  = def.file as string | undefined;
        if (!filePath) {
            return undefined;
        }
        return new vscode.Task(
            def,
            task.scope ?? vscode.TaskScope.Workspace,
            task.name,
            'bndbuild',
            new vscode.ShellExecution(
                `${bndbuildPath} -f "${path.basename(filePath)}" ${def.target}`,
                { cwd: path.dirname(filePath) },
            ),
        );
    }
}
