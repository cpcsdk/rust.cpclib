import * as vscode from 'vscode';
import { ExtensionContext, window } from 'vscode';
import { client } from '../lsp/client';

// ── Cycle count for selection ───────────────────────────────────────────────
//
// The server-side `cpclib.cycleCountForSelection` command (backed by
// `cpclib-lsp/src/basm/cycles.rs`) is also directly usable from the Quick
// Fix menu (a purely informational code action showing the same numbers in
// its title - works in any LSP client, no extension code needed). This is
// the VS Code-only, richer counterpart: a status bar item that stays in
// sync with the current selection live, mirroring `updateCursorContext`'s
// debounced-selection-listener shape in `commands/inkColorPicker.ts`.

// Selection cycle-count status bar item - created in `registerCycleCountStatusBar()`.
let cycleCountStatusBarItem: vscode.StatusBarItem;

interface CycleCountResult {
    min_nops: number;
    max_nops: number;
    // True when the selection contains a block-repeat instruction (LDIR/
    // LDDR/CPIR/CPDR/INIR/INDR/OTIR/OTDR) whose iteration count (BC) isn't
    // statically known - `max_nops` is then a meaningless partial sum, not
    // a real upper bound (see `cpclib-lsp/src/basm/cycles.rs`'s
    // `SelectionCycleCount::max_unbounded` doc comment).
    max_unbounded: boolean;
    instruction_count: number;
    unrecognized_count: number;
}

async function updateCycleCountStatusBar(editor: vscode.TextEditor | undefined): Promise<void> {
    // A bare cursor position (no selection) is sent too, not just a real
    // drag-selection - the server now shows the cost of the single
    // instruction/line the cursor is on in that case (see
    // `cycle_count_for_selection`'s own doc comment in `command.rs`),
    // rather than nothing at all.
    if (!editor || editor.document.languageId !== 'basm') {
        cycleCountStatusBarItem.hide();
        return;
    }

    let result: CycleCountResult | null | undefined;
    try {
        result = await client.sendRequest<CycleCountResult | null>('workspace/executeCommand', {
            command: 'cpclib.cycleCountForSelection',
            arguments: [{
                uri: editor.document.uri.toString(),
                range: client.code2ProtocolConverter.asRange(editor.selection),
            }],
        });
    } catch {
        // LSP not ready yet, or the request failed - same "treat as
        // nothing to show" handling as `colorsFor`'s own try/catch.
        cycleCountStatusBarItem.hide();
        return;
    }
    if (!result) {
        cycleCountStatusBarItem.hide();
        return;
    }

    const { min_nops, max_nops, max_unbounded, unrecognized_count } = result;
    const range = max_unbounded
        ? `${min_nops}-?`
        : min_nops === max_nops ? `${min_nops}` : `${min_nops}-${max_nops}`;
    const warning = unrecognized_count > 0 ? ' ⚠' : '';
    cycleCountStatusBarItem.text = `$(watch) ${range} NOPs${warning}`;

    // Wording only - a bare cursor position (no drag-selection) still
    // shows a real count (the cursor's own line), just not literally a
    // "selection" the user made.
    const label = editor.selection.isEmpty ? 'Cycle count' : 'Selection cycle count';
    let tooltip = max_unbounded
        ? `${label}: ${min_nops} NOPs (best case) - unbounded (a repeat-block instruction's loop count isn't statically known)`
        : min_nops === max_nops
            ? `${label}: ${min_nops} NOPs`
            : `${label}: ${min_nops} NOPs (best case) - ${max_nops} NOPs (worst case, branch taken)`;
    if (unrecognized_count > 0) {
        tooltip += `\n${unrecognized_count} line(s) not counted (macro call or unrecognized instruction) - actual total may be higher.`;
    }
    cycleCountStatusBarItem.tooltip = tooltip;
    cycleCountStatusBarItem.show();
}

export function registerCycleCountStatusBar(context: ExtensionContext): void {
    cycleCountStatusBarItem = window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    context.subscriptions.push(cycleCountStatusBarItem);
    let cycleCountTimer: ReturnType<typeof setTimeout> | undefined;
    context.subscriptions.push(
        window.onDidChangeActiveTextEditor(editor => { void updateCycleCountStatusBar(editor); }),
        window.onDidChangeTextEditorSelection(e => {
            if (cycleCountTimer) {
                clearTimeout(cycleCountTimer);
            }
            // Debounced: dragging out a selection fires this repeatedly:
            // querying the server on every tick would be wasteful, and the
            // status bar only needs to catch up shortly after the drag
            // settles, not track it live keystroke-by-keystroke.
            cycleCountTimer = setTimeout(() => { void updateCycleCountStatusBar(e.textEditor); }, 250);
        }),
    );
    void updateCycleCountStatusBar(window.activeTextEditor);
}
