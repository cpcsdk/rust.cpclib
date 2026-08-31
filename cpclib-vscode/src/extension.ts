import * as vscode from 'vscode';
import { ExtensionContext, window, workspace } from 'vscode';
import { createLanguageClient, client, resolvedServerPath } from './lsp/client';
import { installLogMessageMirror } from './lsp/logMirror';
import { BndbuildTaskProvider } from './tasks/bndbuildTaskProvider';
import { registerCodeLensRunners } from './tasks/codeLensRunners';
import { registerInkColorPicker } from './commands/inkColorPicker';
import { registerPeephole } from './commands/peephole';
import { registerAssemble } from './commands/assemble';
import { registerBuildActiveFile } from './commands/buildActiveFile';
import { registerEditConfig } from './commands/editConfig';
import { registerMusic } from './commands/music';
import { registerBreakpointSync } from './commands/breakpointSync';
import { registerCycleCountStatusBar } from './statusBar/cycleCount';
import { registerRegistersStatusBar } from './statusBar/registers';
import { registerDebugging } from './debug/register';
import { setDebugClient } from './debug/launch';

export function activate(context: ExtensionContext): void {
    const config = workspace.getConfiguration('cpclib-lsp');
    const languageClient = createLanguageClient(context, config);

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
        new BndbuildTaskProvider(languageClient, config),
    );
    context.subscriptions.push(taskProvider);

    languageClient.start().then(() => {
        setDebugClient(languageClient);
        window.showInformationMessage('CPClib LSP server started.');
        installLogMessageMirror(languageClient);
    }).catch((err: Error) => {
        window.showErrorMessage(`CPClib LSP failed to start: ${err.message}. Check cpclib-lsp.serverPath setting.`);
    });

    registerInkColorPicker(context);
    registerBuildActiveFile(context);
    registerCodeLensRunners(context);
    registerPeephole(context);
    registerAssemble(context);
    registerMusic(context);
    registerEditConfig(context);

    registerDebugging(context, () => resolvedServerPath);

    registerBreakpointSync(context);

    registerCycleCountStatusBar(context);
    registerRegistersStatusBar(context);
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
