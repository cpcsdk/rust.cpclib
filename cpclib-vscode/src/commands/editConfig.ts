import * as vscode from 'vscode';
import { ExtensionContext, window } from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { execFile } from 'child_process';
import { resolvedServerPath } from '../lsp/client';

/**
 * Opens the project's `cpclib-lsp.toml`, creating it at the workspace
 * root with the server's own defaults first if it doesn't exist yet.
 *
 * Reuses `cpclib-lsp --init-config` (cpclib-lsp/src/main.rs) rather than
 * duplicating `EXAMPLE_CONFIG_TOML`'s own content here - the same reason
 * `bndbuildCommandPrefix` reuses the bundled binary's own `bndbuild`
 * subcommand instead of shipping a second one. `--init-config` itself
 * refuses to overwrite an existing file, so the existence check happens
 * here first and the CLI is only invoked when there is genuinely nothing
 * to open yet.
 */
async function editConfig(): Promise<void> {
    const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    if (!root) {
        void window.showWarningMessage('Open a workspace folder first.');
        return;
    }
    const configPath = path.join(root, 'cpclib-lsp.toml');
    if (!fs.existsSync(configPath)) {
        const created = await new Promise<boolean>(resolve => {
            execFile(resolvedServerPath, ['--init-config', root], error => {
                if (error) {
                    void window.showErrorMessage(
                        `Could not create cpclib-lsp.toml: ${error.message}`,
                    );
                }
                resolve(!error);
            });
        });
        if (!created || !fs.existsSync(configPath)) { return; }
    }
    const document = await vscode.workspace.openTextDocument(configPath);
    await window.showTextDocument(document);
}

export function registerEditConfig(context: ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.editConfig', editConfig),
    );
}
