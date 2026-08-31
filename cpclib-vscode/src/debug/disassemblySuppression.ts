// VS Code's own Disassembly view, shut before it can take the stop.
//
// This adapter deliberately does not advertise `supportsDisassembleRequest`
// - an editor told it can disassemble keeps stepping at instruction
// granularity and shows that view instead of your source, which is the
// opposite of what a source-level debugger is for. But the view is an
// *editor tab*: once it has been opened it is restored with the window,
// and it then sits there empty (this session will never fill it), takes
// the stop, and quietly turns stepping into instruction stepping. Closing
// it when a session starts is the only way to be rid of it; `-dv` opens a
// disassembly that actually has contents.

import * as vscode from 'vscode';

/**
 * Close VS Code's built-in Disassembly view, wherever it is.
 *
 * Identified by having no recognised editor input - it is not a file, a
 * notebook, a diff or a webview - together with a label naming it. Both halves
 * matter: the label alone would risk closing someone's file called
 * "disassembly.asm", and the input alone would catch every exotic editor.
 */
export function looksLikeDisassembly(tab: vscode.Tab): boolean {
    const known =
        tab.input instanceof vscode.TabInputText ||
        tab.input instanceof vscode.TabInputTextDiff ||
        tab.input instanceof vscode.TabInputCustom ||
        tab.input instanceof vscode.TabInputWebview ||
        tab.input instanceof vscode.TabInputNotebook ||
        tab.input instanceof vscode.TabInputTerminal;
    return !known && /disassembl|désassembl/i.test(tab.label);
}

export async function closeBuiltInDisassemblyView(): Promise<void> {
    const doomed = vscode.window.tabGroups.all
        .flatMap(group => group.tabs)
        .filter(looksLikeDisassembly);
    if (doomed.length > 0) {
        await vscode.window.tabGroups.close(doomed, false);
    }
}
