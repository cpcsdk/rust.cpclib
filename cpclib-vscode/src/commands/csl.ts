import * as vscode from 'vscode';
import { ExtensionContext } from 'vscode';
import { client } from '../lsp/client';
import { pickWorkspaceFile } from '../shared/pickWorkspaceFile';

/**
 * Prompts for a `.csl` file: every one found in the open workspace, plus a
 * "Browse..." entry - for the Command Palette form of {@link runCsl}, where
 * there is no file-explorer target and (unlike `.asm`/`.bas`) a `.csl` file
 * is rarely the active editor tab.
 */
function pickCslFile(): Promise<string | undefined> {
    return pickWorkspaceFile({
        glob: '**/*.{csl,CSL}',
        browseLabel: '$(folder-opened) Browse for a CSL script...',
        placeHolder: 'Which CSL script?',
        dialogFilters: { 'CSL script': ['csl', 'CSL'] },
        dialogOpenLabel: 'Select',
    });
}

/**
 * "▶ Run CSL script in emulator" - a file-browser context-menu / command-
 * palette entry. Unlike `cpclib.runBasic`/`cpclib.runAssembly`/`cpclib.runCsl`
 * itself (CodeLens-only, invoked with an explicit `arguments: [path]` VS Code
 * never touches, and bridged automatically since they're `executeCommandProvider`-
 * advertised), this is registered client-side because VS Code hands a context-
 * menu invocation a `vscode.Uri`, not a string - same reason `playMusic`/
 * `buildMusicDsk` are separate client commands from the server's own
 * `cpclib.musicPlay`/`cpclib.musicBuildDsk`. Forwards to the server's
 * `cpclib.runCsl` (the same command `csl::command::code_lens` already
 * triggers from inside an open document).
 *
 * The server reports the outcome itself (`show_message`/`log_message`), so
 * there is nothing to do here with the response.
 */
async function runCsl(target: string | vscode.Uri | undefined): Promise<void> {
    let fileName = target instanceof vscode.Uri ? target.fsPath : target;
    if (!fileName) {
        fileName = await pickCslFile();
        if (!fileName) { return; }
    }
    await client.sendRequest('workspace/executeCommand', {
        command: 'cpclib.runCsl',
        arguments: [fileName],
    });
}

export function registerCsl(context: ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.runCslFile', runCsl),
    );
}
