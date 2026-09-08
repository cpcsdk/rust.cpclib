import * as vscode from 'vscode';
import { LanguageClient, WorkDoneProgress } from 'vscode-languageclient/node';

/**
 * Runs `work` wrapped in a prominent, visible `vscode.window.withProgress`
 * notification, fed by `$/progress` reports the *server* sends against a
 * token generated right here - not vscode-languageclient's own automatic
 * handling of a server-initiated `window/workDoneProgress/create`, which
 * (per that library's own source comment, `common/progressPart.ts`) renders
 * as "a silent window progress with a hidden notification"
 * (`ProgressLocation.Window`, a barely-visible status-bar sliver) with no
 * way to configure it otherwise.
 *
 * `work` receives the token as its own argument - passing it through as a
 * command argument (e.g. `vscode.commands.executeCommand('cpclib.runX',
 * ...args, token)`) is how the server-side handler learns to report against
 * *this* token instead of minting and `workDoneProgress/create`-ing its own.
 * The server must skip that create handshake for a client-supplied token
 * (see `cpclib-lsp`'s `start_build_progress_with_token`) - asking the
 * client to "create" a token it already generated and is already listening
 * on would be redundant, and the LSP spec reserves `workDoneProgress/create`
 * for a token the *server* mints.
 */
export async function withServerProgress<T>(
    client: LanguageClient,
    title: string,
    work: (token: string) => Promise<T>,
): Promise<T> {
    const token = `cpclib-${Date.now()}-${Math.random().toString(36).slice(2)}`;

    return vscode.window.withProgress(
        { location: vscode.ProgressLocation.Notification, title, cancellable: false },
        async progress => {
            // Mirrors `ProgressPart.report`'s own delta bookkeeping in
            // vscode-languageclient (`withProgress` wants an *increment*
            // per report, not an absolute percentage).
            let reported = 0;
            const disposable = client.onProgress(WorkDoneProgress.type, token, value => {
                if (value.kind === 'report' || value.kind === 'begin') {
                    if (typeof value.percentage === 'number') {
                        const percentage = Math.max(0, Math.min(value.percentage, 100));
                        const increment = Math.max(0, percentage - reported);
                        reported += increment;
                        progress.report({ message: value.message, increment });
                    } else {
                        progress.report({ message: value.message });
                    }
                }
            });
            try {
                return await work(token);
            } finally {
                disposable.dispose();
            }
        },
    );
}
