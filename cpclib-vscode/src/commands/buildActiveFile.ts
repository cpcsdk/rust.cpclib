import * as vscode from 'vscode';
import { ExtensionContext, window } from 'vscode';
import { client } from '../lsp/client';
import { BUILD_FILE_GLOB, BndbuildTaskProvider } from '../tasks/bndbuildTaskProvider';

// ── Build the active .asm file's own bndbuild target(s) ────────────────────
//
// "cpclib.buildActiveFile": finds every bndbuild file in the workspace that
// references the currently active .asm file (as a dependency or as one of
// its own targets, via the server-side `cpclib.getTargetsForFile`
// reverse-lookup) and runs the one match directly, or offers a QuickPick
// when several build files/targets reference the same source file.

async function buildActiveFile(): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor) {
        return;
    }
    const sourcePath = editor.document.uri.fsPath;
    if (!/\.(asm|z80)$/i.test(sourcePath)) {
        window.showInformationMessage('The active file is not an assembly file.');
        return;
    }

    const buildFiles = await vscode.workspace.findFiles(
        BUILD_FILE_GLOB,
        '{**/node_modules/**,**/.git/**,**/target/**}',
    );

    type Match = { buildFileUri: vscode.Uri; target: string };
    // One `getTargetsForFile` round-trip per build file, all in flight at
    // once instead of one at a time - same reasoning (and same per-file
    // try/catch preserved) as `BndbuildTaskProvider.provideTasks`.
    const perFileMatches = await Promise.all(buildFiles.map(async (buildFileUri): Promise<Match[]> => {
        try {
            const targets = await client.sendRequest<string[]>(
                'workspace/executeCommand',
                { command: 'cpclib.getTargetsForFile', arguments: [buildFileUri.toString(), sourcePath] },
            ) ?? [];
            return targets.map(target => ({ buildFileUri, target }));
        } catch {
            // LSP not ready or file unreadable — skip this build file.
            return [];
        }
    }));
    const matches: Match[] = perFileMatches.flat();

    if (matches.length === 0) {
        window.showInformationMessage('No bndbuild file in this workspace references the active file.');
        return;
    }

    let chosen = matches[0];
    if (matches.length > 1) {
        const items = matches.map(m => ({
            label: m.target,
            // Workspace-relative path, not just the bare filename - same
            // ambiguity risk as `BndbuildTaskProvider.provideTasks`' own
            // task naming when several build files share a target name.
            description: vscode.workspace.asRelativePath(m.buildFileUri),
            match: m,
        }));
        const picked = await window.showQuickPick(items, { placeHolder: 'Select a build target' });
        if (!picked) {
            return;
        }
        chosen = picked.match;
    }

    const allTasks = await vscode.tasks.fetchTasks({ type: BndbuildTaskProvider.taskType });
    const task = allTasks.find(t =>
        t.definition.target === chosen.target
        && t.definition.file === chosen.buildFileUri.fsPath,
    );
    if (task) {
        await vscode.tasks.executeTask(task);
    } else {
        window.showErrorMessage(`Could not find the '${chosen.target}' build task.`);
    }
}

export function registerBuildActiveFile(context: ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.buildActiveFile', buildActiveFile),
    );
}
