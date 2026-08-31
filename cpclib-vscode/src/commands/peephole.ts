import * as vscode from 'vscode';
import { workspace, ExtensionContext, window } from 'vscode';
import { client } from '../lsp/client';

/// Ask the server for peephole-optimization suggestions.
///
/// The server's automatic pass is off by default - deciding whether a `jp`
/// reaches as a `jr` needs the addresses a real build produces, which means
/// assembling the whole project, and that is not a keystroke-time cost. These
/// commands are the explicit request that turns it on for the file (or the
/// project) the user actually cares about. Results arrive as ordinary
/// diagnostics in the Problems panel, and stay live as the file is edited
/// until `cpclib.clearPeepholeResults`.
///
/// The project sweep is driven from here, one server request per file, rather
/// than being a single server-side command. Analysing a file is *seconds* of
/// real assembling, so a project of any size is minutes and a workspace
/// holding several demos is much worse - which is exactly what the first
/// version did, with no count, no progress and no way to stop it. Owning the
/// loop here is what makes those three possible.
async function analyzePeephole(scope: 'file' | 'selection' | 'project'): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor) {
        window.showInformationMessage('Open an assembly file first.');
        return;
    }
    if (!/\.(asm|z80)$/i.test(editor.document.uri.fsPath)) {
        window.showInformationMessage('The active file is not an assembly file.');
        return;
    }

    let selection: unknown | undefined;
    if (scope === 'selection') {
        if (editor.selection.isEmpty) {
            window.showInformationMessage('Select the code to analyze first.');
            return;
        }
        // The server narrows only what it *reports*: the analysis itself
        // always covers the whole file, because whether a rewrite is safe
        // depends on the code surrounding it.
        selection = {
            start: { line: editor.selection.start.line, character: editor.selection.start.character },
            end: { line: editor.selection.end.line, character: editor.selection.end.character },
        };
    }

    let targets: vscode.Uri[] = [editor.document.uri];
    if (scope === 'project') {
        // Scoped to the folder the active file belongs to, not to every
        // workspace folder: a checkout that holds several demos would
        // otherwise sweep all of them.
        const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri);
        if (!folder) {
            window.showInformationMessage('This file is not inside a workspace folder.');
            return;
        }
        targets = await vscode.workspace.findFiles(
            new vscode.RelativePattern(folder, '**/*.{asm,z80}'),
            '{**/node_modules/**,**/.git/**,**/target/**}',
        );
        if (targets.length === 0) {
            window.showInformationMessage('No assembly files found in this folder.');
            return;
        }
        // Each file costs a real assemble. Say how many before spending them.
        const go = await window.showWarningMessage(
            `Analyze ${targets.length} file${targets.length === 1 ? '' : 's'} in ${folder.name}? `
            + 'Each one is assembled, so this can take several minutes.',
            { modal: true },
            'Analyze',
        );
        if (go !== 'Analyze') {
            return;
        }
    }

    await window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: 'Looking for peephole optimizations',
            cancellable: true,
        },
        async (progress, token) => {
            // A handful in flight at once: the server answers requests
            // concurrently, so this is where the parallelism comes from, while
            // the bound keeps a large project from queueing hundreds of
            // assembles at once.
            const CONCURRENCY = 8;
            let done = 0;
            let cancelled = false;
            const findings: PeepholeFinding[] = [];

            for (let i = 0; i < targets.length; i += CONCURRENCY) {
                if (token.isCancellationRequested) {
                    cancelled = true;
                    break;
                }
                const batch = targets.slice(i, i + CONCURRENCY);
                const perFile = await Promise.all(batch.map(async uri => {
                    try {
                        return await client.sendRequest<PeepholeFinding[] | null>(
                            'workspace/executeCommand',
                            {
                                command: 'cpclib.analyzePeephole',
                                arguments: selection ? [uri.toString(), selection] : [uri.toString()],
                            },
                        ) ?? [];
                    } catch {
                        // One unreadable or unparseable file must not abandon
                        // the sweep.
                        return [];
                    }
                }));
                for (const list of perFile) {
                    findings.push(...list);
                }
                done += batch.length;
                progress.report({
                    increment: (batch.length / targets.length) * 100,
                    message: `${done}/${targets.length} files, ${findings.length} found`,
                });
            }

            const where = targets.length === 1 ? 'this file' : `${done} files`;
            await reportFindings(findings, where, cancelled);
        },
    );
}

/// One suggestion, as the server reports it.
type PeepholeFinding = {
    uri: string;
    line: number;
    character: number;
    message: string;
};

/// `file.asm:42`, the form that is worth reading in a notification.
function findingLabel(finding: PeepholeFinding): string {
    const name = vscode.Uri.parse(finding.uri).path.split('/').pop() ?? finding.uri;
    return `${name}:${finding.line + 1}`;
}

/// Tell the user what was found, and let them get there.
///
/// A bare count is not actionable - it leaves the reader to go hunting for
/// something the analysis already knows the exact position of. So: the
/// locations go in the message itself while they still fit, the full list is
/// always one click away as a jumpable quick pick, and every one of them is
/// written to the output channel where it can be read back later.
async function reportFindings(
    findings: PeepholeFinding[],
    where: string,
    cancelled: boolean,
): Promise<void> {
    if (findings.length > 0) {
        client.outputChannel.appendLine(`Peephole optimizations in ${where}:`);
        for (const finding of findings) {
            client.outputChannel.appendLine(`  ${findingLabel(finding)}  ${finding.message}`);
        }
    }

    if (findings.length === 0) {
        window.showInformationMessage(
            cancelled
                ? `Peephole analysis stopped after ${where}: nothing found so far.`
                : `No peephole optimizations found in ${where}.`);
        return;
    }

    const count = `${findings.length} peephole optimization${findings.length === 1 ? '' : 's'}`;
    // Short lists read better inline than behind a click; long ones would turn
    // the notification into a wall.
    const inline = findings.length <= 3
        ? ` (${findings.map(findingLabel).join(', ')})`
        : '';
    const summary = cancelled
        ? `Peephole analysis stopped after ${where}: ${count} found so far${inline}.`
        : `${count} found in ${where}${inline}.`;

    const choice = await window.showInformationMessage(summary, 'Go to…');
    if (choice !== 'Go to…') {
        return;
    }
    await pickFinding(findings);
}

/// A jumpable list of findings, previewing each one as it is highlighted.
async function pickFinding(findings: PeepholeFinding[]): Promise<void> {
    type Item = vscode.QuickPickItem & { finding: PeepholeFinding };
    const items: Item[] = findings.map(finding => ({
        label: findingLabel(finding),
        description: finding.message,
        finding,
    }));

    const reveal = async (finding: PeepholeFinding) => {
        const document = await workspace.openTextDocument(vscode.Uri.parse(finding.uri));
        const position = new vscode.Position(finding.line, finding.character);
        await window.showTextDocument(document, {
            selection: new vscode.Range(position, position),
            preview: true,
        });
    };

    const picker = window.createQuickPick<Item>();
    picker.items = items;
    picker.placeholder = 'Peephole optimizations';
    picker.matchOnDescription = true;
    picker.onDidChangeActive(active => {
        if (active[0]) {
            void reveal(active[0].finding);
        }
    });
    picker.onDidAccept(() => {
        picker.hide();
    });
    picker.onDidHide(() => picker.dispose());
    picker.show();
}

/// Stop reporting peephole optimizations - for the active file, or everywhere
/// when no assembly file is active.
async function clearPeephole(): Promise<void> {
    const editor = window.activeTextEditor;
    const uri = editor && /\.(asm|z80)$/i.test(editor.document.uri.fsPath)
        ? editor.document.uri.toString()
        : undefined;
    try {
        await client.sendRequest('workspace/executeCommand', {
            command: 'cpclib.clearPeephole',
            arguments: uri ? [uri] : [],
        });
    } catch (err) {
        window.showErrorMessage(`Could not clear the peephole results: ${(err as Error).message}`);
    }
}

// These four IDs are deliberately *not* the server-side command names
// (`cpclib.analyzePeephole`, `cpclib.analyzePeepholeWorkspace`,
// `cpclib.clearPeephole`) they forward to. vscode-languageclient
// auto-registers a bridge command for every name the server advertises in
// `executeCommandProvider`, so registering the same name here throws
// "command already exists" and aborts the whole client start - which
// surfaces to the user as "Client is not running" the moment they invoke
// anything.
export function registerPeephole(context: ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.findPeepholeInFile', () => analyzePeephole('file')),
        vscode.commands.registerCommand('cpclib.findPeepholeInSelection', () => analyzePeephole('selection')),
        vscode.commands.registerCommand('cpclib.findPeepholeInProject', () => analyzePeephole('project')),
        vscode.commands.registerCommand('cpclib.clearPeepholeResults', clearPeephole),
    );
}
