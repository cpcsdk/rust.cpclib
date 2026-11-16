import * as vscode from 'vscode';
import { ExtensionContext } from 'vscode';
import { client } from '../lsp/client';
import { isDebugSessionActive } from '../debug/launch';

// ---------------------------------------------------------------------------
// Breakpoints
// ---------------------------------------------------------------------------

/**
 * Mirror VS Code's breakpoints into `breakpoint` directives in the source.
 *
 * There is no debug adapter here, and writing one would not help: the red dot
 * has to become an *address* for an emulator to stop on, and every emulator
 * takes that list differently - when it takes one at all. basm already has a
 * `breakpoint` directive that travels into the snapshot, so the shortest path
 * from the dot to a stopped emulator is to put the directive in the file.
 *
 * Which means toggling a breakpoint edits the source. That is a real
 * side-effect and not everyone will want it, hence
 * `cpclib.breakpointDirective` - but it is on by default, since a red dot that
 * does nothing at all is worse.
 *
 * The server decides *where* on the line the directive goes: in basm, telling a
 * label from a mnemonic needs a parse, not a regex.
 *
 * A red dot's condition is the directive's `CONDITION="..."` (ACE's language,
 * as the debugger applies it): set, changed or cleared on the dot, the
 * directive follows - only then: a dot touched otherwise (enabled, given a hit
 * count) leaves the directive as written, a condition typed in the source
 * included. Removing the dot removes a plain directive and comments out one
 * with arguments; a dot set again on that line brings it back. A hit count or
 * a log message has no directive attribute: it lasts as long as the dot.
 */
/** Each red dot's condition when last seen, by the dot's id. */
const dotConditions = new Map<string, string | null>();

async function syncBreakpointDirectives(event: vscode.BreakpointsChangeEvent): Promise<void> {
    const before = new Map(dotConditions);
    const remember = (breakpoint: vscode.Breakpoint) => {
        if (breakpoint instanceof vscode.SourceBreakpoint) {
            dotConditions.set(breakpoint.id, breakpoint.condition?.trim() || null);
        }
    };
    event.added.forEach(remember);
    event.changed.forEach(remember);
    event.removed.forEach(breakpoint => dotConditions.delete(breakpoint.id));
    await syncWithConditions(event, before);
}

async function syncWithConditions(
    event: vscode.BreakpointsChangeEvent,
    dotConditionsBefore: Map<string, string | null>,
): Promise<void> {
    // During a debug session a red dot is a *live* breakpoint: VS Code sends it
    // to the adapter, which sets it in the emulator. Writing the directive as
    // well would edit the user's source behind their back and desynchronise the
    // two - so the writer stands down for the duration.
    if (isDebugSessionActive()) {
        return;
    }
    if (!vscode.workspace.getConfiguration('cpclib').get<boolean>('breakpointDirective', true)) {
        return;
    }
    if (!client) {
        return;
    }
    // `showExistingBreakpoints` adds breakpoints, which fires this event right
    // back. Nothing breaks if it runs - the server declines to insert a second
    // directive on a line that already has one - but the round trip is pure
    // waste, so the flag skips it outright.
    if (adoptingExistingBreakpoints) {
        return;
    }

    // `undefined`: the directive's condition is left as it is.
    const wanted: { uri: vscode.Uri; line: number; enable: boolean; condition: string | null | undefined }[] = [];
    const collect = (breakpoints: readonly vscode.Breakpoint[], enable: boolean) => {
        for (const breakpoint of breakpoints) {
            if (!(breakpoint instanceof vscode.SourceBreakpoint)) { continue; }
            const uri = breakpoint.location.uri;
            if (!uri.fsPath.toLowerCase().endsWith('.asm')) { continue; }
            const condition = breakpoint.condition?.trim() || null;
            const known = dotConditionsBefore.get(breakpoint.id);
            // Said only when it is the dot's own news: a new dot's condition, or
            // one changed. Otherwise a condition typed in the source, or the
            // one a commented directive brings back, would be wiped.
            const told = !enable ? undefined
                : known === undefined ? (condition ?? undefined)
                : condition !== known ? condition
                : undefined;
            wanted.push({ uri, line: breakpoint.location.range.start.line, enable, condition: told });
        }
    };
    collect(event.added, true);
    collect(event.removed, false);
    // A changed breakpoint may have moved: VS Code reports the new location
    // only, so the directive is (re)placed there and any stale one elsewhere
    // is left for the user - guessing at the old line would be worse than
    // leaving a directive they can see.
    collect(event.changed, true);

    if (wanted.length === 0) {
        return;
    }

    const edit = new vscode.WorkspaceEdit();
    for (const { uri, line, enable, condition } of wanted) {
        try {
            // The document must be open for the server to have parsed it.
            await vscode.workspace.openTextDocument(uri);
            const textEdit = await client.sendRequest<{
                range: { start: { line: number; character: number }; end: { line: number; character: number } };
                newText: string;
            } | null>(
                'workspace/executeCommand',
                {
                    command: 'cpclib.breakpointEdit',
                    arguments: condition === undefined
                        ? [uri.toString(), line, enable]
                        : [uri.toString(), line, enable, condition],
                },
            );
            if (!textEdit) { continue; }
            edit.replace(
                uri,
                new vscode.Range(
                    textEdit.range.start.line, textEdit.range.start.character,
                    textEdit.range.end.line, textEdit.range.end.character,
                ),
                textEdit.newText,
            );
        } catch (err) {
            client.outputChannel.appendLine(
                `[breakpoints] cpclib.breakpointEdit failed for ${uri.fsPath}:${line + 1}: ${err}`,
            );
        }
    }

    if (edit.size > 0) {
        await vscode.workspace.applyEdit(edit);
    }
}

/**
 * Show a red dot for every `breakpoint` directive already written in a file.
 *
 * Without this the mapping only runs one way: a directive committed last week,
 * or typed by hand, would sit in the source with nothing in the gutter to say
 * so - and clicking that line would then try to add a *second* one.
 *
 * The server finds them, because a directive is not always the first thing on
 * its line (`ld a,0 : BREAKPOINT` is as valid as the other order) and knowing
 * that means parsing, not scanning for a word.
 */
let adoptingExistingBreakpoints = false;

async function showExistingBreakpoints(document: vscode.TextDocument): Promise<void> {
    if (!vscode.workspace.getConfiguration('cpclib').get<boolean>('breakpointDirective', true)) {
        return;
    }
    if (!client || document.languageId !== 'basm') {
        return;
    }

    // Each directive with its condition, for its dot to carry it: a dot
    // without would write "no condition" back the first time it changes.
    let directives: { line: number; condition: string | null }[] = [];
    try {
        directives = await client.sendRequest<{ line: number; condition: string | null }[]>(
            'workspace/executeCommand',
            { command: 'cpclib.breakpointDirectives', arguments: [document.uri.toString()] },
        ) ?? [];
    } catch (err) {
        client.outputChannel.appendLine(
            `[breakpoints] cpclib.breakpointDirectives failed for ${document.uri.fsPath}: ${err}`,
        );
        return;
    }
    if (directives.length === 0) {
        return;
    }

    // Only the ones the editor does not already know about, so reopening a
    // file does not pile duplicates onto the same line.
    const known = new Set(
        vscode.debug.breakpoints
            .filter((b): b is vscode.SourceBreakpoint => b instanceof vscode.SourceBreakpoint)
            .filter(b => b.location.uri.toString() === document.uri.toString())
            .map(b => b.location.range.start.line),
    );
    const missing = directives.filter(({ line }) => !known.has(line));
    if (missing.length === 0) {
        return;
    }

    adoptingExistingBreakpoints = true;
    try {
        vscode.debug.addBreakpoints(missing.map(({ line, condition }) => new vscode.SourceBreakpoint(
            new vscode.Location(document.uri, new vscode.Position(line, 0)),
            true,
            condition ?? undefined,
        )));
    } finally {
        adoptingExistingBreakpoints = false;
    }
}

export function registerBreakpointSync(context: ExtensionContext): void {
    // the dots VS Code kept from the last time, as they are now
    for (const breakpoint of vscode.debug.breakpoints) {
        if (breakpoint instanceof vscode.SourceBreakpoint) {
            dotConditions.set(breakpoint.id, breakpoint.condition?.trim() || null);
        }
    }
    context.subscriptions.push(
        vscode.debug.onDidChangeBreakpoints(e => { void syncBreakpointDirectives(e); }),
        // The other direction: a file that already contains directives shows
        // its dots as soon as it is opened.
        vscode.workspace.onDidOpenTextDocument(doc => { void showExistingBreakpoints(doc); }),
    );
    for (const doc of vscode.workspace.textDocuments) {
        void showExistingBreakpoints(doc);
    }
}
