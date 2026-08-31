// Disassembled memory, with every row a link back to the source it came
// from - `-dv` in the debug console.
//
// Rendered as a real `basm` document via a `TextDocumentContentProvider`,
// not a webview: mnemonics/operands are genuine basm syntax (real token
// coloring, identical to source), a `symbol` heading becomes a real basm
// label line, and address/opcode-bytes/source-location metadata rides in a
// trailing `;`-comment. Interactive bits carried over from the old webview:
// a RAM-configuration picker (now an `editor/title` toolbar button instead
// of an in-page `<select>`) and click-to-navigate (now a
// `DocumentLinkProvider` targeting `file:` URIs with a `line,col` fragment,
// rather than a custom postMessage handler).
//
// Memory and source are not the same thing in a demo, in two ways that
// matter while debugging: a macro or a `REPEAT` turns one source line into a
// screenful of opcodes, and self-modifying code means the bytes running are
// not the bytes that were assembled. So this is worth having open *beside*
// the source rather than instead of it.

import * as vscode from 'vscode';
import { hex } from '../shared/html';
import { Disassembly } from './types';
import { consoleCommand } from './consoleCommand';
import { editorsShowing } from './instructionHint';

export const DISASSEMBLY_SCHEME = 'cpclib-disasm';

/** Keeps a debug session's own URI stable across repeated `-dv` refreshes -
 * mirrors `basicListingDoc.ts`'s own `sessionUris`. */
const sessionUris = new Map<string, vscode.Uri>();

interface DisassemblyDocState {
    dump: Disassembly;
    text: string;
    links: vscode.DocumentLink[];
    /** 0-based line in `text` holding the at-PC row, if the program is
     * sitting on one of these instructions right now. */
    atPcLine: number | undefined;
}

/** Per-open-document state (keyed by `uri.toString()`). */
const state = new Map<string, DisassemblyDocState>();

function sanitizeForUriSegment(name: string): string {
    return name.replace(/[^A-Za-z0-9 ._-]/g, '_');
}

function uriForSession(session: vscode.DebugSession): vscode.Uri {
    let uri = sessionUris.get(session.id);
    if (!uri) {
        uri = vscode.Uri.from({
            scheme: DISASSEMBLY_SCHEME,
            path: `/${session.id}/${sanitizeForUriSegment(session.name)}.disasm`,
        });
        sessionUris.set(session.id, uri);
    }
    return uri;
}

/** `config` (`0`-`7`, or `null`/`undefined` for the live CPU view) as the
 * `cpclib.disasmConfig` context-key value the `editor/title` toolbar
 * button's 9 `when`-gated command variants are keyed on. */
function configKey(config: number | null | undefined): string {
    return typeof config === 'number' ? `c${config}` : 'live';
}

function configLabel(config: number | null | undefined): string {
    return typeof config === 'number' ? `C${config}` : 'Live (CPU)';
}

function padColumn(text: string, width: number): string {
    return text.length + 2 <= width ? text.padEnd(width) : `${text}  `;
}

const INSTRUCTION_COL_WIDTH = 36;
const INDENT = '        ';

/** Builds the document text, its click-to-navigate links, and the at-PC
 * line, from one `-dv` dump - the plain-text/basm-syntax counterpart to the
 * old `disassemblyHtml`. */
function renderDisassemblyDocument(dump: Disassembly): {
    text: string;
    links: vscode.DocumentLink[];
    atPcLine: number | undefined;
} {
    const lines: string[] = [];
    const links: vscode.DocumentLink[] = [];
    let atPcLine: number | undefined;

    const title = dump.label
        ? `${dump.label} — &${hex(dump.address, 4)}`
        : `&${hex(dump.address, 4)}`;
    lines.push(`; ${title} — ${dump.instructions.length} instructions`);
    lines.push('');

    for (const entry of dump.instructions) {
        if (entry.symbol) {
            const alternatives = (entry.symbolAlternatives ?? []).length
                ? `  ; also ${entry.symbolAlternatives!.join(', ')}`
                : '';
            lines.push(`${entry.symbol}:${alternatives}`);
        }

        const addressNum = parseInt(entry.address.replace(/^0x/i, ''), 16);
        // The emulator writes addresses lowercase; compare on the number.
        const atPc = typeof dump.pc === 'number' && addressNum === dump.pc;

        const path = entry.location?.path;
        const hasLocation = !!(path && entry.line);
        const locationName = entry.location?.name ?? '';
        const locationLabel = hasLocation
            ? `${locationName}:${entry.line}${entry.column ? `:${entry.column}` : ''}`
            : '';

        const bytesPart = entry.instructionBytes ? ` ${entry.instructionBytes}` : '';
        const symbolsPart = (entry.symbols ?? []).length
            ? ` (${(entry.symbols ?? []).join(', ')})`
            : '';
        const commentBody = hasLocation
            ? `&${hex(addressNum, 4)}:${bytesPart} -> ${locationLabel}${symbolsPart}`
            : `&${hex(addressNum, 4)}:${bytesPart}${symbolsPart}`;

        const insnText = `${INDENT}${entry.instruction ?? ''}`;
        const line = `${padColumn(insnText, INSTRUCTION_COL_WIDTH)}; ${commentBody}`;
        lines.push(line);
        const lineIndex = lines.length - 1;

        if (atPc) { atPcLine = lineIndex; }

        if (hasLocation) {
            // The link spans exactly the "name:line[:col]" substring within
            // the trailing comment - `line`'s own text, not the whole line,
            // so hovering/clicking only lights up the location itself.
            const locationStart = line.lastIndexOf(locationLabel);
            if (locationStart >= 0) {
                // `file:` URI fragments take a single `line,col` position,
                // not a range - `endColumn` has no equivalent here. This is
                // "jump to column," not "select the exact instruction span"
                // (a deliberate, accepted simplification from the old
                // webview's precise-selection click behavior; see the
                // refactor plan's Phase 3c notes).
                const target = vscode.Uri.file(path!).with({
                    fragment: `${entry.line},${entry.column ?? 1}`,
                });
                links.push(new vscode.DocumentLink(
                    new vscode.Range(lineIndex, locationStart, lineIndex, locationStart + locationLabel.length),
                    target,
                ));
            }
        }
    }

    lines.push('');
    lines.push('; Decoded by basm\'s own tables, not by the emulator - so this reads the same whichever emulator is underneath.');
    lines.push(
        dump.followsPc
            ? '; Following PC: this re-reads itself on every step.'
            : '; Anchored here; the RAM-configuration button above follows PC again when set back to "Live (CPU)".',
    );
    lines.push('; Click a linked location to open the line it came from. This is what is in memory: after self-modifying');
    lines.push('; code, or a macro, it will not match your source one-for-one.');
    lines.push('; Only AMSpiriT Lite can honour an explicit RAM configuration.');

    return { text: lines.join('\n'), links, atPcLine };
}

class DisassemblyContentProvider implements vscode.TextDocumentContentProvider {
    private readonly changeEmitter = new vscode.EventEmitter<vscode.Uri>();
    readonly onDidChange = this.changeEmitter.event;

    provideTextDocumentContent(uri: vscode.Uri): string {
        // Cheap synchronous read only - VS Code may call this speculatively
        // more than once per logical change, and it must never itself
        // trigger a new `-dv` round trip. Defensive fallback for a tab VS
        // Code still holds open after its session's state was dropped (a
        // tab moved to another editor group might dodge the best-effort
        // close on terminate).
        return state.get(uri.toString())?.text ?? '; This debug session has ended.';
    }

    fireChange(uri: vscode.Uri): void {
        this.changeEmitter.fire(uri);
    }
}

const provider = new DisassemblyContentProvider();

class DisassemblyLinkProvider implements vscode.DocumentLinkProvider {
    provideDocumentLinks(document: vscode.TextDocument): vscode.DocumentLink[] {
        return state.get(document.uri.toString())?.links ?? [];
    }
}

const atPcDecorationType = vscode.window.createTextEditorDecorationType({
    isWholeLine: true,
    backgroundColor: new vscode.ThemeColor('editor.selectionBackground'),
});

function applyAtPcDecoration(uri: vscode.Uri, atPcLine: number | undefined): void {
    const ranges = atPcLine !== undefined ? [new vscode.Range(atPcLine, 0, atPcLine, 0)] : [];
    for (const editor of editorsShowing(uri.toString())) {
        editor.setDecorations(atPcDecorationType, ranges);
    }
}

/** Keeps the `cpclib.disasmConfig` context key (the `editor/title` toolbar
 * button's 9 `when`-gated command variants are keyed on it) in sync with
 * whichever disassembly document is currently active - a static menu
 * contribution cannot compute its own label, so this is what makes the
 * *right* one of the 9 variants show. */
function refreshActiveDisasmConfigContext(): void {
    const editor = vscode.window.activeTextEditor;
    const uri = editor?.document.uri;
    const key = uri?.scheme === DISASSEMBLY_SCHEME
        ? configKey(state.get(uri.toString())?.dump.config)
        : undefined;
    void vscode.commands.executeCommand('setContext', 'cpclib.disasmConfig', key);
}

export async function showDisassembly(
    session: vscode.DebugSession,
    dump: Disassembly | undefined,
): Promise<void> {
    if (!dump || !Array.isArray(dump.instructions)) { return; }

    const uri = uriForSession(session);
    const isNew = !state.has(uri.toString());
    const { text, links, atPcLine } = renderDisassemblyDocument(dump);
    state.set(uri.toString(), { dump, text, links, atPcLine });

    if (isNew) {
        const document = await vscode.workspace.openTextDocument(uri);
        // Custom-scheme documents are not associated with a language by file
        // extension the way `file:`-scheme ones are - skipping this call
        // silently renders the tab as plain text with zero coloring.
        await vscode.languages.setTextDocumentLanguage(document, 'basm');
        await vscode.window.showTextDocument(document, {
            viewColumn: vscode.ViewColumn.Beside,
            preview: false,
            preserveFocus: true,
        });
    } else {
        // Only reveal on first open - a PC-following view refreshes on
        // *every* stop, and revealing it each time would pull it over
        // whatever shares its column, exactly like the old webview panel's
        // own `isNew`-only reveal.
        provider.fireChange(uri);
    }

    applyAtPcDecoration(uri, atPcLine);
    refreshActiveDisasmConfigContext();
}

/** Best-effort close of a session's own tab, and drops its cached state -
 * called from `onDidTerminateDebugSession`. */
export async function disposeDisassemblyDoc(sessionId: string): Promise<void> {
    const uri = sessionUris.get(sessionId);
    if (!uri) { return; }
    state.delete(uri.toString());
    sessionUris.delete(sessionId);

    const tab = vscode.window.tabGroups.all
        .flatMap(group => group.tabs)
        .find(t => t.input instanceof vscode.TabInputText && t.input.uri.toString() === uri.toString());
    if (tab) {
        await vscode.window.tabGroups.close(tab, false);
    }
    refreshActiveDisasmConfigContext();
}

/** All 9 RAM-configuration values the `editor/title` toolbar button's
 * `when`-gated command variants cover - "Live (CPU)" plus C0-C7, mirroring
 * the old in-page `<select>`'s own `<option>` list. */
const CONFIG_CHOICES: (number | null)[] = [null, 0, 1, 2, 3, 4, 5, 6, 7];

/**
 * Opens the RAM-configuration picker for the *active* disassembly document -
 * the `editor/title` toolbar button's command handler (all 9 `when`-gated
 * command ids share this one implementation). Selecting a config issues the
 * same `-dv` console command the old in-page `<select>`'s `reissue()` sent;
 * the visible update happens later, when the adapter's next
 * `cpclib/disassemblyView` event lands and {@link showDisassembly} re-renders
 * - never applied directly from here.
 */
async function pickDisassemblyConfig(): Promise<void> {
    const uri = vscode.window.activeTextEditor?.document.uri;
    const current = uri?.scheme === DISASSEMBLY_SCHEME ? state.get(uri.toString()) : undefined;
    if (!current) { return; }

    const picked = await vscode.window.showQuickPick(
        CONFIG_CHOICES.map(c => ({
            label: configLabel(c),
            picked: configKey(c) === configKey(current.dump.config),
            config: c,
        })),
        { placeHolder: 'RAM configuration for this disassembly view' },
    );
    if (!picked) { return; }

    const anchor = current.dump.followsPc ? '_' : `0x${hex(current.dump.address, 4)}`;
    const count = current.dump.instructions.length || 32;
    const config = picked.config === null ? '_' : String(picked.config);
    await consoleCommand(`-dv ${anchor} ${count} ${config}`);
}

/** Every `cpclib.disassemblyConfigPicker.<key>` command id declared in
 * package.json's `editor/title` toolbar - one per {@link CONFIG_CHOICES}
 * entry, all sharing {@link pickDisassemblyConfig}. */
const CONFIG_PICKER_COMMAND_IDS = CONFIG_CHOICES.map(c => `cpclib.disassemblyConfigPicker.${configKey(c)}`);

export function registerDisassemblyDoc(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.workspace.registerTextDocumentContentProvider(DISASSEMBLY_SCHEME, provider),
        vscode.languages.registerDocumentLinkProvider({ scheme: DISASSEMBLY_SCHEME }, new DisassemblyLinkProvider()),
        { dispose: () => atPcDecorationType.dispose() },
    );

    for (const id of CONFIG_PICKER_COMMAND_IDS) {
        context.subscriptions.push(vscode.commands.registerCommand(id, pickDisassemblyConfig));
    }

    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor(() => refreshActiveDisasmConfigContext()),
        vscode.window.onDidChangeVisibleTextEditors(editors => {
            for (const editor of editors) {
                if (editor.document.uri.scheme !== DISASSEMBLY_SCHEME) { continue; }
                applyAtPcDecoration(editor.document.uri, state.get(editor.document.uri.toString())?.atPcLine);
            }
        }),
    );
}
