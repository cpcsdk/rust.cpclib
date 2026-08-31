import * as vscode from 'vscode';
import { ExtensionContext, window } from 'vscode';
import { client } from '../lsp/client';

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

export function registerAssemble(context: ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.assembleThisFile', assembleActiveFile),
    );
}
