// The live BASIC listing, straight from the emulator's own memory - `-bv` in
// the debug console, one virtual document per session (there is only ever
// one program loaded at a time, unlike memory views which can point
// anywhere).
//
// Rendered as a real `locomotive-basic` document via a
// `TextDocumentContentProvider`, not a webview: that is what gets it genuine
// TextMate/semantic-token coloring identical to a real `.bas` file on disk,
// instead of the old hand-rolled `BASIC_KEYWORDS` regex tokenizer and
// hardcoded CSS. There is no interactivity to preserve here (no postMessage,
// no click handling) - this is the simpler of the two virtual-document
// conversions; see `disassemblyDoc.ts` for the one with a config picker and
// click-to-navigate.

import * as vscode from 'vscode';
import { BasicListingDump } from './types';

export const BASIC_LISTING_SCHEME = 'cpclib-basic-listing';

/** Keeps a debug session's own URI stable across repeated `-bv` refreshes -
 * `vscode.Uri.from` re-encodes on every call, so this is what lets
 * `showTextDocument`'s own tab-identity-by-URI reuse actually kick in. */
const sessionUris = new Map<string, vscode.Uri>();

/** The listing text per open document (keyed by `uri.toString()`, not
 * session id - the same shape `provideTextDocumentContent` reads from). */
const listingText = new Map<string, string>();

/** A session name is free-form text (spaces, punctuation, anything a debug
 * configuration's `name` can hold) - keep it a plausible-looking, harmless
 * URI path segment rather than passing it through unescaped. */
function sanitizeForUriSegment(name: string): string {
    return name.replace(/[^A-Za-z0-9 ._-]/g, '_');
}

function uriForSession(session: vscode.DebugSession): vscode.Uri {
    let uri = sessionUris.get(session.id);
    if (!uri) {
        uri = vscode.Uri.from({
            scheme: BASIC_LISTING_SCHEME,
            path: `/${session.id}/${sanitizeForUriSegment(session.name)}.bas`,
        });
        sessionUris.set(session.id, uri);
    }
    return uri;
}

class BasicListingContentProvider implements vscode.TextDocumentContentProvider {
    private readonly changeEmitter = new vscode.EventEmitter<vscode.Uri>();
    readonly onDidChange = this.changeEmitter.event;

    provideTextDocumentContent(uri: vscode.Uri): string {
        // Defensive, not the expected path: `onDidTerminateDebugSession`
        // already drops the state and best-effort closes the tab, but a tab
        // moved to another editor group might not be found by that close,
        // and VS Code can re-request content for a still-open tab after its
        // backing state is gone.
        return listingText.get(uri.toString()) ?? '10 REM This debug session has ended.';
    }

    fireChange(uri: vscode.Uri): void {
        this.changeEmitter.fire(uri);
    }
}

const provider = new BasicListingContentProvider();

export function registerBasicListingDoc(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.workspace.registerTextDocumentContentProvider(BASIC_LISTING_SCHEME, provider),
    );
}

export async function showBasicListing(
    session: vscode.DebugSession,
    dump: BasicListingDump | undefined,
): Promise<void> {
    if (!dump || typeof dump.text !== 'string') { return; }

    const uri = uriForSession(session);
    const isNew = !listingText.has(uri.toString());
    listingText.set(uri.toString(), dump.text);

    if (isNew) {
        const document = await vscode.workspace.openTextDocument(uri);
        // Custom-scheme documents are not associated with a language by file
        // extension the way `file:`-scheme ones are - skipping this call
        // silently renders the tab as plain text with zero coloring.
        await vscode.languages.setTextDocumentLanguage(document, 'locomotive-basic');
        await vscode.window.showTextDocument(document, {
            viewColumn: vscode.ViewColumn.Beside,
            preview: false,
            preserveFocus: true,
        });
    } else {
        // Only reveal on first open, exactly like the old webview panel did -
        // a refresh from a fresh `-bv` (or the adapter's own live update)
        // must not steal focus/scroll position on every one of them.
        provider.fireChange(uri);
    }
}

/** Best-effort close of a session's own tab, and drops its cached state -
 * called from `onDidTerminateDebugSession`, mirroring the other debug
 * views' own `xPanels.get(session.id)?.dispose(); xPanels.delete(...)`
 * cleanup. */
export async function disposeBasicListing(sessionId: string): Promise<void> {
    const uri = sessionUris.get(sessionId);
    if (!uri) { return; }
    listingText.delete(uri.toString());
    sessionUris.delete(sessionId);

    // Tab-close isn't guaranteed to find the tab (e.g. if the user moved it
    // to another editor group) - `provideTextDocumentContent`'s own fallback
    // string covers that case.
    const tab = vscode.window.tabGroups.all
        .flatMap(group => group.tabs)
        .find(t => t.input instanceof vscode.TabInputText && t.input.uri.toString() === uri.toString());
    if (tab) {
        await vscode.window.tabGroups.close(tab, false);
    }
}
