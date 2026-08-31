import * as vscode from 'vscode';

/**
 * Type a debug-console command for the user.
 *
 * `-dv` and `-mv` answer with a `cpclib/*View` event, which opens the panel -
 * so the palette entries are the console entries, not a second way of doing
 * the same thing that could drift from it.
 */
export async function consoleCommand(expression: string): Promise<void> {
    const session = vscode.debug.activeDebugSession;
    if (!session) {
        void vscode.window.showWarningMessage(
            'No debug session is running. Start one first.',
        );
        return;
    }
    try {
        await session.customRequest('evaluate', { expression, context: 'repl' });
    } catch (error) {
        void vscode.window.showErrorMessage(`${expression} failed: ${error}`);
    }
}
