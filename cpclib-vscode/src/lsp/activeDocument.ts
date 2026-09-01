import * as vscode from 'vscode';

/**
 * Tells the server which document VS Code's active editor currently shows,
 * via `cpclib.setActiveDocument` - the server uses this to skip its
 * expensive, full multi-pass assemble for background tabs nobody's looking
 * at (see the server's own `CpcLspBackend::should_fully_assemble` doc
 * comment). Reported live: a workspace restore reopening dozens of
 * previously-open tabs used to full-assemble every single one, including
 * ones the user had no intention of looking at right then.
 *
 * Registered once `languageClient.start()` has resolved (called from
 * `extension.ts`'s own `activate()`), since the command it calls is
 * advertised/bridged by the server itself - calling it any earlier would be
 * a no-op at best.
 */
export function registerActiveDocumentTracking(context: vscode.ExtensionContext): void {
    const report = (editor: vscode.TextEditor | undefined): void => {
        void vscode.commands.executeCommand(
            'cpclib.setActiveDocument',
            editor?.document.uri.toString() ?? null,
        );
    };

    // `onDidChangeActiveTextEditor` only fires on a *change* from here on -
    // never for whatever's already active when this runs, which restoring a
    // workspace's previous session already made active before the extension
    // finished activating.
    report(vscode.window.activeTextEditor);
    context.subscriptions.push(vscode.window.onDidChangeActiveTextEditor(report));
}
