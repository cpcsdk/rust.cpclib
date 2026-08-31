import * as vscode from 'vscode';

/**
 * Consolidates what used to be three structurally identical pickers
 * (`pickSnapshotFile`/`pickDiskFile` in `debug.ts`, `pickMusicFile` in
 * `extension.ts`): every matching file in the open workspace, fuzzy-
 * filterable as you type (`showQuickPick`'s own built-in matching - there is
 * no separate "autocomplete" API for a bare file path in VS Code), plus a
 * "Browse..." entry for one outside it.
 */
export interface PickWorkspaceFileOptions {
    /** Glob passed to `vscode.workspace.findFiles`. */
    glob: string;
    /** Exclude glob, also passed to `findFiles`. Defaults to the same
     * `node_modules`/`.git`/`out` exclusion all three original pickers used. */
    excludeGlob?: string;
    /** The sentinel QuickPick item that opens the native file dialog instead,
     * e.g. `'$(folder-opened) Browse for a .sna file...'`. */
    browseLabel: string;
    /** `showQuickPick`'s own placeholder text. */
    placeHolder: string;
    /** `showOpenDialog`'s own filters. */
    dialogFilters: Record<string, string[]>;
    /** `showOpenDialog`'s own `openLabel`. */
    dialogOpenLabel: string;
}

export async function pickWorkspaceFile(opts: PickWorkspaceFileOptions): Promise<string | undefined> {
    const found = await vscode.workspace.findFiles(
        opts.glob,
        opts.excludeGlob ?? '**/{node_modules,.git,out}/**',
        200,
    );
    const picked = await vscode.window.showQuickPick(
        [
            opts.browseLabel,
            ...found.map(uri => vscode.workspace.asRelativePath(uri)),
        ],
        { placeHolder: opts.placeHolder },
    );
    if (picked === undefined) { return undefined; }
    if (picked === opts.browseLabel) {
        const chosen = await vscode.window.showOpenDialog({
            canSelectMany: false,
            filters: opts.dialogFilters,
            openLabel: opts.dialogOpenLabel,
        });
        return chosen?.[0]?.fsPath;
    }
    const match = found.find(uri => vscode.workspace.asRelativePath(uri) === picked);
    return match?.fsPath;
}
