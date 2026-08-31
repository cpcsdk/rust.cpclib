// Small string helpers shared by the webviews that still render raw HTML
// (memory, grouped-memory, screen, CRTC, emulator). The disassembly and
// BASIC-listing views no longer need these - they render real basm/BASIC
// text through `TextDocumentContentProvider`s instead.

export const hex = (value: number, width: number): string =>
    value.toString(16).toUpperCase().padStart(width, '0');

export const escapeHtml = (text: string): string =>
    text.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]!));
