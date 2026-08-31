// Ghost-text decoration machinery: shows, after the stopped line, what the
// bytes at `PC` actually decode to (self-modifying code, macros, a `defs`
// filled at run time). Reused as-is by the disassembly virtual document's
// own at-PC row highlight (`debug/disassemblyDoc.ts`).

import * as vscode from 'vscode';
import { StopLocation } from './types';

/**
 * The dimmed text after the stopped line, made on first use.
 *
 * One type for the whole extension: a decoration type is how VS Code addresses
 * a set of decorations, so making a second one would leave the first one's text
 * on screen with no way left to remove it.
 */
let instructionHint: vscode.TextEditorDecorationType | undefined;

/**
 * What the hint says and where it belongs.
 *
 * Kept rather than only written into an editor because the same file can be
 * open in more than one group, and a group can be split *after* the stop: an
 * editor that appears later has to be given the hint it never saw, and one
 * showing anything else can be known to be stale.
 */
interface InstructionHint {
    /** The hinted document, as `Uri.toString()` - what editors are matched on. */
    uri: string;
    line: number;
    text: string;
    endColumn?: number;
}

let currentHint: InstructionHint | undefined;

export function instructionHintType(): vscode.TextEditorDecorationType {
    instructionHint ??= vscode.window.createTextEditorDecorationType({
        after: {
            margin: '0 0 0 2em',
            // Ghost text: the colour VS Code already uses for code-shaped text
            // that is not in the file. Exactly this hint's status, and lighter
            // than real code in every theme.
            //
            // One colour for the whole hint, not one per token as the `-dv`
            // panel gives it. An `after` decoration is a single text
            // attachment and takes a single `color`; the only way to colour
            // its words separately is several decoration types at the same
            // position, whose rendering order VS Code does not define - and a
            // hint that comes out as `a, ld 0x01)(` is worse than a plain one.
            color: new vscode.ThemeColor('editorGhostText.foreground'),
        },
        // The hint describes one address, so it must not be dragged along by an
        // edit above it.
        rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
    });
    return instructionHint;
}

/**
 * Every visible editor showing one document.
 *
 * Not `find`: a decoration belongs to an *editor*, not to a document, so the
 * same file open in two groups is two editors and decorating the first one
 * leaves the second showing the stopped line with nothing beside it.
 */
export function editorsShowing(uri: string): vscode.TextEditor[] {
    return vscode.window.visibleTextEditors.filter(
        editor => editor.document.uri.toString() === uri,
    );
}

/** Put the hint into one editor. */
export function drawInstructionHint(editor: vscode.TextEditor, hint: { line: number; endColumn?: number; text: string }): void {
    if (hint.line >= editor.document.lineCount) { return; }
    // Just after the instruction, not at the end of the line. A line can hold
    // several instructions (`ld a,l : inc a : ld (hl),a`), and a hint parked at
    // the far right would sit beside whichever came last rather than beside the
    // one being executed.
    const lineRange = editor.document.lineAt(hint.line).range;
    const at = hint.endColumn && hint.endColumn > 1
        ? lineRange.start.translate(0, Math.min(hint.endColumn - 1, lineRange.end.character))
        : lineRange.end;
    editor.setDecorations(instructionHintType(), [{
        range: new vscode.Range(at, at),
        renderOptions: { after: { contentText: `(${hint.text})` } },
    }]);
}

/**
 * Show, after the line, what the bytes at `PC` decode to.
 *
 * Written as an instruction rather than as a comment - it *is* one, in the
 * assembler's own spelling - and parenthesised so it cannot be mistaken for
 * code that is in the file.
 */
export function showInstructionHint(
    document: vscode.TextDocument,
    line: number,
    text: string,
    endColumn?: number,
): void {
    clearInstructionHint();
    if (line >= document.lineCount) { return; }

    currentHint = { uri: document.uri.toString(), line, text, endColumn };
    for (const editor of editorsShowing(currentHint.uri)) {
        drawInstructionHint(editor, currentHint);
    }
}

/**
 * Put the hint on the editor already showing the stop, without disturbing it.
 *
 * Separate from `revealStop` because it arrives separately: the adapter reads
 * the bytes at `PC` from the emulator, which costs a round trip it refuses to
 * make the reveal wait for.
 */
export function applyInstructionHint(where: StopLocation | undefined): void {
    if (!where?.path || !where.line) { return; }
    const line = Math.max(0, where.line - 1);
    // Any editor showing the file will do to name the document; the hint then
    // goes to all of them.
    const document = vscode.window.visibleTextEditors.find(
        candidate => candidate.document.uri.fsPath === where.path,
    )?.document;
    if (!document) { return; }
    if (where.instruction) {
        showInstructionHint(document, line, where.instruction, where.endColumn);
    } else {
        clearInstructionHint();
    }
}

/**
 * Take the hint away, everywhere.
 *
 * A hint left over from the previous stop is worse than no hint at all: it
 * describes an address the program has already left, in a spelling that looks
 * authoritative. So it goes on continue, on the next stop, and when the session
 * ends - and from every editor, since the stop may since have moved file.
 */
export function clearInstructionHint(): void {
    currentHint = undefined;
    if (!instructionHint) { return; }
    for (const editor of vscode.window.visibleTextEditors) {
        editor.setDecorations(instructionHint, []);
    }
}

/** Disposes the shared decoration type - the extension itself owns it since
 * it outlives any one debug session. */
export function disposeInstructionHintType(): void {
    instructionHint?.dispose();
}

/**
 * A hint set in a file that is then hidden cannot be taken out of an editor
 * nobody can see, so it is caught on the way back: every visible editor
 * showing something other than the hinted file is cleared. The other half is
 * the split made *after* the stop - an editor that was never there to be
 * decorated - which is given the hint here instead.
 */
export function reconcileVisibleEditors(editors: readonly vscode.TextEditor[]): void {
    if (!instructionHint) { return; }
    for (const editor of editors) {
        if (currentHint && editor.document.uri.toString() === currentHint.uri) {
            drawInstructionHint(editor, currentHint);
        } else {
            editor.setDecorations(instructionHint, []);
        }
    }
}
