import * as vscode from 'vscode';
import { workspace, ExtensionContext, window } from 'vscode';

// ── CPC ink color picker ────────────────────────────────────────────────────
//
// VS Code always opens its own continuous RGB/HSV picker for a language's
// `documentColor` swatches - there is no extension point to replace that
// with something else. This command is a deliberate second path: it reuses
// the LSP's own `documentColor`/`colorPresentation` results (so the ink
// list, ordering, and edits stay a single source of truth with the server)
// but presents them as a `QuickPick` restricted to the real 27/32-entry CPC
// palette, with an accurate color swatch icon per entry. Other editors keep
// using the standard LSP color flow unchanged; this is VS Code-only, purely
// additive on top of it.

// Mirrors `cpclib-lsp/src/common/colors.rs`'s `INK_RGB` table exactly (ink
// index 0-31, the 27-31 range being GA-byte-distinct firmware duplicates of
// an earlier index's RGB) - kept here only to render each `QuickPick`
// item's color swatch icon; the actual ink list/edits still come from the
// server via `colorPresentation`.
const INK_RGB: readonly [number, number, number][] = [
    [0x00, 0x00, 0x00], [0x00, 0x00, 0x80], [0x00, 0x00, 0xFF], [0x80, 0x00, 0x00],
    [0x80, 0x00, 0x80], [0x80, 0x00, 0xFF], [0xFF, 0x00, 0x00], [0xFF, 0x00, 0x80],
    [0xFF, 0x00, 0xFF], [0x00, 0x80, 0x00], [0x00, 0x80, 0x80], [0x00, 0x80, 0xFF],
    [0x80, 0x80, 0x00], [0x80, 0x80, 0x80], [0x80, 0x80, 0xFF], [0xFF, 0x80, 0x00],
    [0xFF, 0x80, 0x80], [0xFF, 0x80, 0xFF], [0x00, 0xFF, 0x00], [0x00, 0xFF, 0x80],
    [0x00, 0xFF, 0xFF], [0x80, 0xFF, 0x00], [0x80, 0xFF, 0x80], [0x80, 0xFF, 0xFF],
    [0xFF, 0xFF, 0x00], [0xFF, 0xFF, 0x80], [0xFF, 0xFF, 0xFF],
    [0x80, 0x80, 0x80], [0xFF, 0x00, 0x80], [0xFF, 0xFF, 0x80], [0x00, 0x00, 0x80], [0x00, 0xFF, 0x80],
];

// The 27 official Amstrad CPC ink names, in index order - mirrors
// `cpclib-image/src/ink.rs`'s `impl Display for Ink` (and its inverse,
// `impl From<String> for Ink`, which also accepts these with spaces
// removed/underscored). Indices 27-31 are firmware duplicates of an
// earlier index's *color* (and so, its name too) - see `INK_RGB` above.
const INK_NAMES: readonly string[] = [
    'Black', 'Blue', 'Bright Blue', 'Red', 'Magenta', 'Mauve', 'Bright Red',
    'Purple', 'Bright Magenta', 'Green', 'Cyan', 'Sky Blue', 'Yellow', 'White',
    'Pastel Blue', 'Orange', 'Pink', 'Pastel Magenta', 'Bright Green', 'Sea Green',
    'Bright Cyan', 'Lime', 'Pastel Green', 'Pastel Cyan', 'Bright Yellow',
    'Pastel Yellow', 'Bright White',
    'White', 'Purple', 'Pastel Yellow', 'Blue', 'Sea Green',
];

/// A small solid-color square as a data-URI SVG, for a `QuickPickItem.iconPath`.
function inkSwatchIcon(rgb: readonly [number, number, number]): vscode.Uri {
    const hex = rgb.map(c => c.toString(16).padStart(2, '0')).join('');
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">` +
        `<rect x="1" y="1" width="14" height="14" fill="#${hex}" stroke="#888888" stroke-width="1"/></svg>`;
    return vscode.Uri.parse(`data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`);
}

interface InkQuickPickItem extends vscode.QuickPickItem {
    presentation: vscode.ColorPresentation;
    idx: number;
}

// Cache of the active document's color swatches (from textDocument/
// documentColor), invalidated whenever the document's version changes -
// shared between the "is the cursor on a color" context-key check (which
// runs on every cursor move) and the picker command itself, so moving the
// cursor around doesn't re-hit the LSP each time.
let cachedColors: vscode.ColorInformation[] = [];
let cachedColorsKey: string | undefined;

async function colorsFor(document: vscode.TextDocument): Promise<vscode.ColorInformation[]> {
    if (
        document.languageId !== 'basm'
        && document.languageId !== 'locomotive-basic'
        && document.languageId !== 'catart-basic'
    ) {
        return [];
    }
    const key = `${document.uri.toString()}@${document.version}`;
    if (key === cachedColorsKey) {
        return cachedColors;
    }
    try {
        const colors = await vscode.commands.executeCommand<vscode.ColorInformation[]>(
            'vscode.executeDocumentColorProvider',
            document.uri,
        );
        cachedColors = colors ?? [];
        cachedColorsKey = key;
    } catch {
        // LSP not ready yet, or the request failed - treat as "no colors"
        // rather than throwing, since this also drives a background
        // context-key update that shouldn't surface an error to the user.
        cachedColors = [];
        cachedColorsKey = key;
    }
    return cachedColors;
}

async function updateCursorContext(editor: vscode.TextEditor | undefined): Promise<void> {
    if (!editor) {
        await vscode.commands.executeCommand('setContext', 'cpclib.cursorOnInkColor', false);
        return;
    }
    const colors = await colorsFor(editor.document);
    const onColor = colors.some(c => c.range.contains(editor.selection.active));
    await vscode.commands.executeCommand('setContext', 'cpclib.cursorOnInkColor', onColor);
}

async function pickInkColor(): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor) {
        return;
    }
    const document = editor.document;
    const position = editor.selection.active;

    const colors = await colorsFor(document);
    const swatch = colors.find(c => c.range.contains(position));
    if (!swatch) {
        window.showInformationMessage('No CPC ink color at the cursor.');
        return;
    }

    // Reuses the server's own textDocument/colorPresentation for the exact
    // edit text/style - only the display order (by number, below) and the
    // added name are computed client-side.
    const presentations = await vscode.commands.executeCommand<vscode.ColorPresentation[]>(
        'vscode.executeColorPresentationProvider',
        swatch.color,
        { uri: document.uri, range: swatch.range },
    );
    if (!presentations || presentations.length === 0) {
        window.showInformationMessage(
            'This is a reference to a named constant and can\'t be edited directly - change its definition instead.'
        );
        return;
    }

    const items: InkQuickPickItem[] = presentations
        .map(p => {
            // Server label is "Ink {idx}" or "Ink {idx} (0x{byte})".
            const match = /^Ink (\d+)(?:\s*\((.+)\))?$/.exec(p.label);
            const idx = match ? parseInt(match[1], 10) : -1;
            const byteDetail = match?.[2];
            const name = INK_NAMES[idx] ?? `Ink ${idx}`;
            const rgb: readonly [number, number, number] =
                INK_RGB[idx] ?? [swatch.color.red * 255, swatch.color.green * 255, swatch.color.blue * 255];
            return {
                label: `${idx}: ${name}`,
                description: byteDetail,
                iconPath: inkSwatchIcon(rgb),
                presentation: p,
                idx,
            };
        })
        .sort((a, b) => a.idx - b.idx);

    // showQuickPick's built-in fuzzy filter matches typed text against the
    // label, so typing a CPC name (e.g. "sky blue") narrows the list too.
    const picked = await window.showQuickPick(items, {
        placeHolder: 'Pick a CPC ink color by number or name',
    });
    if (!picked) {
        return;
    }

    const edits = [picked.presentation.textEdit, ...(picked.presentation.additionalTextEdits ?? [])]
        .filter((e): e is vscode.TextEdit => !!e);
    if (edits.length === 0) {
        return;
    }
    const workspaceEdit = new vscode.WorkspaceEdit();
    for (const edit of edits) {
        workspaceEdit.replace(document.uri, edit.range, edit.newText);
    }
    await vscode.workspace.applyEdit(workspaceEdit);
}

/** Registers `cpclib.pickInkColor` and keeps the `cpclib.cursorOnInkColor`
 * context key (its `editor/context` entry's own `when` clause) in sync with
 * the cursor, so the menu entry only shows up where it'd actually do
 * something. */
export function registerInkColorPicker(context: ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.pickInkColor', pickInkColor),
    );

    context.subscriptions.push(
        window.onDidChangeActiveTextEditor(editor => { void updateCursorContext(editor); }),
        window.onDidChangeTextEditorSelection(e => { void updateCursorContext(e.textEditor); }),
    );
    let colorRefreshTimer: ReturnType<typeof setTimeout> | undefined;
    context.subscriptions.push(
        workspace.onDidChangeTextDocument(e => {
            if (e.document !== window.activeTextEditor?.document) {
                return;
            }
            if (colorRefreshTimer) {
                clearTimeout(colorRefreshTimer);
            }
            // Debounced: re-running documentColor on every keystroke would
            // be wasteful, and the context key only needs to be eventually
            // consistent, not instantaneous.
            colorRefreshTimer = setTimeout(() => { void updateCursorContext(window.activeTextEditor); }, 300);
        }),
    );
    void updateCursorContext(window.activeTextEditor);
}
