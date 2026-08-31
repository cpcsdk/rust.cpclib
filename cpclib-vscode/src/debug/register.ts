// Wires every debug-related module together: command registration, the
// adapter descriptor/configuration providers, the built-in Disassembly view
// suppression, the instruction-hint decoration lifecycle, and the dispatcher
// for the adapter's own custom events (`cpclib/*View`, `cpclib/stoppedAt`,
// ...). The adapter itself is `cpclib-lsp`'s sibling binary `cpclib-dap`;
// everything source-level (turning a line into an address, putting a file
// and line back into a stack frame) happens there, in Rust, where it is
// tested.

import * as vscode from 'vscode';
import {
    DEBUG_TYPE, debugActiveFile, debugAssembly, debugBasic, debugSnapshot, debugDisk, debugRule,
    resolveEntry,
} from './launch';
import { registerEmulatorCommands } from './emulators';
import { registerDebugAdapterFactory, registerDebugConfigurationProvider } from './debugAdapterFactory';
import { closeBuiltInDisassemblyView, looksLikeDisassembly } from './disassemblySuppression';
import { consoleCommand } from './consoleCommand';
import {
    clearInstructionHint, reconcileVisibleEditors, disposeInstructionHintType, applyInstructionHint,
    showInstructionHint,
} from './instructionHint';
import { showEmulator, disposeEmulator, emulatorUrls } from './webviews/emulator';
import { showMemory, disposeMemory } from './webviews/memory';
import { showCrtc } from './webviews/crtc';
import { showScreen, disposeScreen } from './webviews/screen';
import { showBasicListing, registerBasicListingDoc, disposeBasicListing } from './basicListingDoc';
import { showDisassembly, registerDisassemblyDoc, disposeDisassemblyDoc } from './disassemblyDoc';
import { StopLocation } from './types';

/** The last stop, so it can be returned to on demand. */
let lastStop: StopLocation | undefined;

/**
 * Open the line the program stopped on, and put the cursor on the instruction.
 *
 * The stack trace already carries this and the editor is meant to act on it,
 * but whether it *reveals* the file turned out to depend on what happened to
 * hold the editor area - the emulator's own webview, a panel, a view restored
 * from a previous session - so the answer was "sometimes". Doing it here does
 * not depend on any of that.
 *
 * `preserveFocus` is deliberately false: stopping at a breakpoint is exactly
 * the moment you want the keyboard in the source.
 */
async function revealStop(where: StopLocation | undefined): Promise<void> {
    if (!where?.path || !where.line) { return; }
    let document: vscode.TextDocument;
    try {
        document = await vscode.workspace.openTextDocument(vscode.Uri.file(where.path));
    } catch {
        return; // a file we cannot open is not worth an error popup on every stop
    }

    const line = Math.max(0, where.line - 1);
    // Columns are 1-based in the protocol and 0-based here. The range is the
    // *instruction*, not the whole line: `ld e,(hl) : inc hl` is two of them,
    // and landing on the first when the second is running is a wrong answer.
    const from = Math.max(0, (where.column ?? 1) - 1);
    const to = where.endColumn && where.endColumn > (where.column ?? 1)
        ? where.endColumn - 1
        : from;
    const selection = new vscode.Range(line, from, line, to);

    // No `viewColumn` here defaults to the *active* column, not wherever the
    // file already happens to be open - and the active column is whatever the
    // user last clicked into, which between stops is routinely the emulator's
    // own window, not this file. Every stop was opening a fresh tab in
    // whichever column that left active, rather than reusing the one already
    // showing this file - repeatedly stealing focus back from the emulator
    // (1984js needs it to keep running: see cpclib-bridge.js) far more often
    // than a debugger stop actually needs to.
    const already = vscode.window.visibleTextEditors.find(
        editor => editor.document.uri.toString() === document.uri.toString()
    );

    const editor = await vscode.window.showTextDocument(document, {
        viewColumn: already?.viewColumn,
        preserveFocus: false,
        preview: false,
        selection,
    });
    // Selected backwards, so the caret lands on the *first* character of the
    // instruction rather than just past its last one. `showTextDocument` leaves
    // the caret at the end of the selection, which is exactly where the hint's
    // `after` decoration sits; the editor measures a caret across the whole DOM
    // box at that position, and that box carries the hint's text, so the caret
    // came out as wide as the hint. Nothing about the selection changes but
    // which of its ends the caret is on.
    editor.selection = new vscode.Selection(selection.end, selection.start);
    editor.revealRange(selection, vscode.TextEditorRevealType.InCenterIfOutsideViewport);

    if (where.instruction) {
        showInstructionHint(document, line, where.instruction, where.endColumn);
    } else {
        clearInstructionHint();
    }
}

export function registerDebugging(
    context: vscode.ExtensionContext,
    resolveAdapterPath: () => string,
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.debugThisFile', debugActiveFile),
        vscode.commands.registerCommand('cpclib.debugAssembly', (fileName?: string) =>
            debugAssembly(fileName)),
        vscode.commands.registerCommand('cpclib.debugBasic', (fileName?: string) =>
            debugBasic(fileName)),
        // A file-explorer context-menu command's own argument is a `Uri`,
        // not a plain path the way a CodeLens's is - both forms are
        // accepted here so the same handler serves the context menu, the
        // Command Palette (no argument at all) and any future CodeLens.
        vscode.commands.registerCommand(
            'cpclib.debugSnapshot',
            (target?: string | vscode.Uri) =>
                debugSnapshot(target instanceof vscode.Uri ? target.fsPath : target, true),
        ),
        vscode.commands.registerCommand(
            'cpclib.runSnapshot',
            (target?: string | vscode.Uri) =>
                debugSnapshot(target instanceof vscode.Uri ? target.fsPath : target, false),
        ),
        vscode.commands.registerCommand(
            'cpclib.debugDisk',
            (target?: string | vscode.Uri) =>
                debugDisk(target instanceof vscode.Uri ? target.fsPath : target, true),
        ),
        vscode.commands.registerCommand(
            'cpclib.runDisk',
            (target?: string | vscode.Uri) =>
                debugDisk(target instanceof vscode.Uri ? target.fsPath : target, false),
        ),
        // The Run lens asks the same question before building: the file you are
        // looking at is usually not the program.
        vscode.commands.registerCommand('cpclib.runAsm', async (fileName?: string) => {
            if (!fileName) {
                fileName = vscode.window.activeTextEditor?.document.fileName;
            }
            if (!fileName) { return; }
            const entry = await resolveEntry(fileName);
            if (!entry) { return; }
            await vscode.commands.executeCommand('cpclib.runAssembly', entry);
        }),
        vscode.commands.registerCommand('cpclib.debugRule', (rule?: string, buildFile?: string) =>
            debugRule(rule, buildFile)),
        vscode.commands.registerCommand('cpclib.openEmulatorInBrowser', async () => {
            const session = vscode.debug.activeDebugSession;
            const url = session ? emulatorUrls.get(session.id) : undefined;
            if (!url) {
                void vscode.window.showWarningMessage(
                    'No emulator is running. Start a debug session first.',
                );
                return;
            }
            // The full machine, with sound - the same one the editor is
            // debugging, since it is served over loopback.
            await vscode.env.openExternal(vscode.Uri.parse(url.replace(/\/debug$/, '/')));
        }),
        // The two views, from the palette as well as from the debug console.
        // They are the same `-dv` and `-mv` the console takes, sent the same
        // way - one implementation, so the panel that opens is the one that
        // knows how to follow `PC` and how to be clicked back into the source.
        vscode.commands.registerCommand('cpclib.openDisassembly', () => consoleCommand('-dv')),
        vscode.commands.registerCommand('cpclib.openMemoryView', () => consoleCommand('-mv')),
        vscode.commands.registerCommand(
            'cpclib.openAllRegisterMemoryViews',
            () => consoleCommand('-mv all,follow'),
        ),
        vscode.commands.registerCommand('cpclib.openCrtcView', () => consoleCommand('-crtcview')),
        vscode.commands.registerCommand('cpclib.openBasicListing', () => consoleCommand('-bv')),
        vscode.commands.registerCommand('cpclib.revealProgramCounter', async () => {
            const where = lastStop;
            if (!where) {
                void vscode.window.showWarningMessage(
                    'The program has not stopped yet, so there is no line to go to.',
                );
                return;
            }
            await revealStop(where);
        }),
    );

    registerEmulatorCommands(context, resolveAdapterPath);
    registerDebugAdapterFactory(context, resolveAdapterPath);
    registerDebugConfigurationProvider(context);
    registerBasicListingDoc(context);
    registerDisassemblyDoc(context);

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
    context.subscriptions.push(
        vscode.debug.onDidStartDebugSession(async session => {
            if (session.type === DEBUG_TYPE) { await closeBuiltInDisassemblyView(); }
        }),
        // ...and again whenever it comes back. Closing it once is not enough:
        // once the editor has switched a session into instruction stepping it
        // re-opens the view on the next step, so the tab has to be answered
        // where it appears rather than where it was first opened.
        vscode.window.tabGroups.onDidChangeTabs(async change => {
            if (vscode.debug.activeDebugSession?.type !== DEBUG_TYPE) { return; }
            if (change.opened.some(looksLikeDisassembly)) {
                await closeBuiltInDisassemblyView();
            }
        }),
    );

    // The resolved-instruction hint describes the address the program is sitting
    // on, so it has to go the moment it is no longer sitting there. `continued`
    // is the only message that says so - the request that caused it may not have
    // been the editor's (a breakpoint hit is resumed by the emulator itself).
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterTrackerFactory(DEBUG_TYPE, {
            createDebugAdapterTracker(): vscode.DebugAdapterTracker {
                return {
                    onDidSendMessage(message: unknown) {
                        const event = (message as { event?: string })?.event;
                        if (event === 'continued') { clearInstructionHint(); }
                    },
                };
            },
        }),
        vscode.window.onDidChangeVisibleTextEditors(editors => reconcileVisibleEditors(editors)),
        // The decoration type outlives any one session, so it is the extension
        // that owns it.
        { dispose: () => disposeInstructionHintType() },
    );

    // The emulator's own window, inside the editor.
    context.subscriptions.push(
        vscode.debug.onDidReceiveDebugSessionCustomEvent(async event => {
            if (event.session.type !== DEBUG_TYPE) { return; }
            if (event.event === 'cpclib/emulatorReady') {
                emulatorUrls.set(event.session.id, event.body?.url ?? '');
                await showEmulator(event.session, event.body?.url);
            }
            if (event.event === 'cpclib/memoryView') {
                showMemory(event.session, event.body);
            }
            if (event.event === 'cpclib/disassemblyView') {
                await showDisassembly(event.session, event.body);
            }
            if (event.event === 'cpclib/crtcView') {
                showCrtc(event.session, event.body);
            }
            if (event.event === 'cpclib/basicListingView') {
                await showBasicListing(event.session, event.body);
            }
            if (event.event === 'cpclib/screenView') {
                showScreen(event.session, event.body);
            }
            // The adapter opened a disassembly view by itself because the
            // program had left the source, and the program is back on a line it
            // was built from. The view has done its job.
            if (event.event === 'cpclib/closeDisassemblyView') {
                await disposeDisassemblyDoc(event.session.id);
            }
            if (event.event === 'cpclib/stoppedAt') {
                lastStop = event.body;
                await revealStop(event.body);
            }
            // Stopped where no source line exists - inside the firmware, most
            // often. There is nothing to reveal, and leaving the previous stop
            // decorated says the program is on a line it left several
            // instructions ago. The disassembly view the adapter opens is what
            // shows where it really is.
            if (event.event === 'cpclib/stoppedWithoutSource') {
                lastStop = undefined;
                clearInstructionHint();
            }
            // The hint the adapter read out of the emulator's own memory, which
            // it could only send once the read came back. Deliberately a second
            // message: re-announcing the stop would drag the cursor back to it
            // for the sake of a decoration.
            if (event.event === 'cpclib/stoppedInstruction') {
                const where = event.body as StopLocation | undefined;
                // `lastStop` is the very object `revealStop` was handed, so
                // updating it here also settles the case where this arrives
                // while that reveal is still opening the document.
                if (lastStop && lastStop.path === where?.path && lastStop.line === where?.line) {
                    lastStop.instruction = where?.instruction ?? null;
                }
                applyInstructionHint(where);
            }
        }),
        vscode.debug.onDidTerminateDebugSession(async session => {
            if (session.type === DEBUG_TYPE) {
                disposeEmulator(session.id);
                disposeMemory(session.id);
                disposeScreen(session.id);
                await disposeBasicListing(session.id);
                await disposeDisassemblyDoc(session.id);
                emulatorUrls.delete(session.id);
                lastStop = undefined;
                clearInstructionHint();
            }
        }),
    );
}
