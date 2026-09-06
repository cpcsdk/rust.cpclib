// A structural, read-only hex viewer for `.sna`/`.cpr` - opens the file as
// bytes and overlays the regions `cpclib.fileRegions` (Rust side,
// `cpclib-lsp/src/fileformat.rs`) already knows the format has (header,
// memory pages, chunks/banks), instead of a plain undifferentiated hex dump.
//
// Deliberately read-only (`CustomReadonlyEditorProvider`, not
// `CustomEditorProvider`): editing a binary snapshot/cartridge file wrongly
// is much easier to get wrong than inspecting one, and VS Code's own
// edit-tracking/undo-redo machinery is real additional complexity this
// first version does not take on. `.dsk`/`.cdt` are not registered here yet -
// `fileformat.rs` returns no regions for them (see its own doc comment for
// why), so a viewer would show an undifferentiated dump indistinguishable
// from just using VS Code's built-in binary preview.

import * as vscode from 'vscode';
import { client } from '../lsp/client';
import { escapeHtml } from '../shared/html';

interface FileRegion {
    offset: number;
    length: number;
    label: string;
    kind: string;
}

const KIND_COLORS: Record<string, string> = {
    header: '#7aa2f7',
    memory: '#9ece6a',
    chunk: '#e0af68',
};

class HexDocument implements vscode.CustomDocument {
    constructor(
        public readonly uri: vscode.Uri,
        public readonly bytes: Uint8Array,
        public readonly regions: FileRegion[],
    ) {}

    dispose(): void {}
}

export class HexViewProvider implements vscode.CustomReadonlyEditorProvider<HexDocument> {
    static readonly viewType = 'cpclib.hexView';

    async openCustomDocument(uri: vscode.Uri): Promise<HexDocument> {
        const bytes = await vscode.workspace.fs.readFile(uri);
        let regions: FileRegion[] = [];
        try {
            const result = await client.sendRequest<{ regions: FileRegion[] } | null>(
                'workspace/executeCommand',
                { command: 'cpclib.fileRegions', arguments: [{ uri: uri.toString() }] },
            );
            regions = result?.regions ?? [];
        } catch {
            // No LSP, or the request failed - still show the bytes, just
            // without the structural overlay.
        }
        return new HexDocument(uri, bytes, regions);
    }

    resolveCustomEditor(document: HexDocument, panel: vscode.WebviewPanel): void {
        panel.webview.options = { enableScripts: false };
        panel.webview.html = hexViewHtml(document);
    }
}

export function registerHexView(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.window.registerCustomEditorProvider(HexViewProvider.viewType, new HexViewProvider(), {
            webviewOptions: { retainContextWhenHidden: true },
            supportsMultipleEditorsPerDocument: true,
        }),
    );
}

const BYTES_PER_ROW = 16;

function regionAt(regions: FileRegion[], offset: number): FileRegion | undefined {
    return regions.find(r => offset >= r.offset && offset < r.offset + r.length);
}

// A plain hex dump has no cheap way to virtualize/lazily render in a
// scripts-disabled webview, and one DOM node per byte stops being practical
// well before a large `.cpr` (32 banks x 16K = 512K)'s full size - so the
// view is capped rather than left to render 30k+ rows and stall the
// webview. 256K covers every `.sna` this workspace produces (max 128K
// machine + a handful of small chunks) with room to spare.
const MAX_RENDERED_BYTES = 256 * 1024;

function hexViewHtml(document: HexDocument): string {
    const { regions } = document;
    const truncated = document.bytes.length > MAX_RENDERED_BYTES;
    const bytes = truncated ? document.bytes.subarray(0, MAX_RENDERED_BYTES) : document.bytes;
    const rows: string[] = [];
    for (let base = 0; base < bytes.length; base += BYTES_PER_ROW) {
        const addr = base.toString(16).padStart(6, '0').toUpperCase();
        const cells: string[] = [];
        const chars: string[] = [];
        for (let i = 0; i < BYTES_PER_ROW; i++) {
            const offset = base + i;
            if (offset >= bytes.length) {
                cells.push('<span class="pad">  </span>');
                continue;
            }
            const byte = bytes[offset];
            const region = regionAt(regions, offset);
            const style = region ? ` style="background:${KIND_COLORS[region.kind] ?? '#888'}22"` : '';
            const title = region ? ` title="${escapeHtml(region.label)}"` : '';
            cells.push(`<span class="byte"${style}${title}>${byte.toString(16).padStart(2, '0').toUpperCase()}</span>`);
            const printable = byte >= 0x20 && byte < 0x7f ? String.fromCharCode(byte) : '.';
            chars.push(`<span class="char"${style}${title}>${escapeHtml(printable)}</span>`);
        }
        rows.push(
            `<div class="row"><span class="addr">${addr}</span>` +
            `<span class="hex">${cells.join(' ')}</span>` +
            `<span class="ascii">${chars.join('')}</span></div>`,
        );
    }

    const legendKinds = [...new Set(regions.map(r => r.kind))];
    const legend = legendKinds.length
        ? `<div class="legend">${legendKinds
              .map(kind => `<span class="swatch" style="background:${KIND_COLORS[kind] ?? '#888'}22">${escapeHtml(kind)}</span>`)
              .join('')}</div>`
        : '<p class="ok">No known structure for this file type - showing a plain hex dump.</p>';

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<style>
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 8px 12px; font-size: 0.9em; }
  .row { display: flex; gap: 8px; white-space: pre; }
  .addr { color: var(--vscode-descriptionForeground); width: 6em; }
  .hex { width: 34em; }
  .byte, .char, .pad { display: inline-block; }
  .byte { width: 1.4em; text-align: center; }
  .ascii { color: var(--vscode-editor-foreground); }
  .legend { margin-top: 10px; display: flex; gap: 10px; flex-wrap: wrap; }
  .swatch { padding: 2px 6px; border-radius: 3px; font-size: 0.85em; }
  .ok { color: var(--vscode-descriptionForeground); }
</style>
</head>
<body>
${legend}
${truncated ? `<p class="ok">Showing the first ${MAX_RENDERED_BYTES / 1024}K of ${(document.bytes.length / 1024).toFixed(0)}K.</p>` : ''}
<div class="rows">${rows.join('')}</div>
</body>
</html>`;
}
