import * as vscode from 'vscode';
import { ExtensionContext, window } from 'vscode';
import { client } from '../lsp/client';

// ── Registers at cursor ─────────────────────────────────────────────────────
//
// The server-side `cpclib.registersAtPosition` command (backed by
// `cpclib-lsp/src/basm/registers.rs`'s `all_tracked_registers_at`, the
// all-at-once counterpart to the per-register value already shown in
// instruction hover) drives a compact status bar item: just an icon plus
// "Registers" text, with every tracked register's value (or "?" when not
// statically known at this point) listed in the tooltip - 13 registers is
// too much to usefully cram into the status bar text itself.

// "Registers at cursor" status bar item - created in `registerRegistersStatusBar()`.
let registersStatusBarItem: vscode.StatusBarItem;

interface RegistersResult {
    a: string | null;
    b: string | null;
    c: string | null;
    d: string | null;
    e: string | null;
    h: string | null;
    l: string | null;
    bc: string | null;
    de: string | null;
    hl: string | null;
    ix: string | null;
    iy: string | null;
    sp: string | null;
}

async function updateRegistersStatusBar(editor: vscode.TextEditor | undefined): Promise<void> {
    if (!editor || editor.document.languageId !== 'basm') {
        registersStatusBarItem.hide();
        return;
    }

    let result: RegistersResult | null | undefined;
    try {
        result = await client.sendRequest<RegistersResult | null>('workspace/executeCommand', {
            command: 'cpclib.registersAtPosition',
            arguments: [{
                uri: editor.document.uri.toString(),
                position: client.code2ProtocolConverter.asPosition(editor.selection.active),
            }],
        });
    } catch {
        registersStatusBarItem.hide();
        return;
    }
    if (!result) {
        registersStatusBarItem.hide();
        return;
    }

    registersStatusBarItem.text = '$(list-unordered) Registers';

    const rows: [string, string | null][] = [
        ['A', result.a], ['B', result.b], ['C', result.c],
        ['D', result.d], ['E', result.e], ['H', result.h], ['L', result.l],
        ['BC', result.bc], ['DE', result.de], ['HL', result.hl],
        ['IX', result.ix], ['IY', result.iy], ['SP', result.sp],
    ];
    const lines = rows.map(([name, value]) => `${name.padEnd(2)} = ${value ?? '?'}`);
    registersStatusBarItem.tooltip = `Registers at cursor:\n${lines.join('\n')}`;
    registersStatusBarItem.show();
}

export function registerRegistersStatusBar(context: ExtensionContext): void {
    registersStatusBarItem = window.createStatusBarItem(vscode.StatusBarAlignment.Right, 99);
    context.subscriptions.push(registersStatusBarItem);
    let registersTimer: ReturnType<typeof setTimeout> | undefined;
    context.subscriptions.push(
        window.onDidChangeActiveTextEditor(editor => { void updateRegistersStatusBar(editor); }),
        window.onDidChangeTextEditorSelection(e => {
            if (registersTimer) {
                clearTimeout(registersTimer);
            }
            registersTimer = setTimeout(() => { void updateRegistersStatusBar(e.textEditor); }, 250);
        }),
    );
    void updateRegistersStatusBar(window.activeTextEditor);
}
