import { workspace, ExtensionContext, window, commands, Terminal, Uri } from 'vscode';
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
