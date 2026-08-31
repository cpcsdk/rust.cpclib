// Debug session launch entry points: the command handlers that start a
// session for one .asm/.bas file, a raw .sna/.dsk, or a bndbuild rule.
//
// The adapter itself is `cpclib-lsp`'s sibling binary `cpclib-dap`; everything
// source-level (turning a line into an address, putting a file and line back
// into a stack frame) happens there, in Rust, where it is tested.

import * as vscode from 'vscode';
import { pickWorkspaceFile } from '../shared/pickWorkspaceFile';

/** The debug type contributed in package.json. */
export const DEBUG_TYPE = 'basm';

/**
 * True while a CPC debug session is running.
 *
 * The breakpoint-directive writer consults this: during a session a red dot is
 * a *live* breakpoint sent to the emulator, and the source must not be touched.
 * With no session it keeps writing the `BREAKPOINT` directive, which is what
 * makes a breakpoint survive into a snapshot.
 */
export function isDebugSessionActive(): boolean {
    return vscode.debug.activeDebugSession?.type === DEBUG_TYPE;
}

/** Start a debug session for one .asm or .bas file. */
export async function debugActiveFile(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    const languageId = editor?.document.languageId;
    if (!editor || (languageId !== 'basm' && languageId !== 'locomotive-basic')) {
        void vscode.window.showWarningMessage('Open a .asm or .bas file to debug it.');
        return;
    }
    // Saving first is not politeness: the adapter assembles the file *on disk*,
    // and an unsaved buffer would be debugged as its previous contents.
    await editor.document.save();
    const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri);
    await vscode.debug.startDebugging(folder, {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${editor.document.fileName}`,
        program: editor.document.fileName,
    });
}

/**
 * The program a document belongs to.
 *
 * The file you are looking at is usually not the program: `events.asm` is
 * included by something that is, and assembling it on its own gets an object
 * with no entry point. The server walks the include graph; when more than one
 * program reaches the file, the answer genuinely depends on which is meant, so
 * the choice is put to the user rather than guessed.
 *
 * `undefined` means the user dismissed the question.
 */
export async function resolveEntry(fileName: string): Promise<string | undefined> {
    let answer: { entry?: string; candidates?: string[] } | undefined;
    try {
        // vscode-languageclient auto-registers a bridge command for every
        // name the server advertises, so this reaches the server directly.
        answer = await vscode.commands.executeCommand('cpclib.resolveEntry', fileName);
    } catch {
        // The server could not say; the file itself is the best guess left.
        return fileName;
    }
    if (answer?.entry) { return answer.entry; }
    const candidates = answer?.candidates ?? [];
    if (candidates.length === 0) { return fileName; }

    const picked = await vscode.window.showQuickPick(
        candidates.map(path => ({
            label: vscode.workspace.asRelativePath(path),
            description: path,
        })),
        { placeHolder: `Which program should be built for ${vscode.workspace.asRelativePath(fileName)}?` },
    );
    return picked?.description;
}

/**
 * Debug a named `.asm` file - the "🐞 Debug" CodeLens at the top of it.
 *
 * Distinct from {@link debugActiveFile}, which takes whatever the editor is
 * focused on: a CodeLens names the file it sits in, and honouring that is what
 * makes clicking it do what it looks like it does even if focus has since moved.
 */
export async function debugAssembly(fileName?: string): Promise<void> {
    if (!fileName) {
        await debugActiveFile();
        return;
    }
    const entry = await resolveEntry(fileName);
    if (!entry) { return; }
    fileName = entry;
    const uri = vscode.Uri.file(fileName);
    // Saving first is not politeness: the adapter assembles the file *on disk*,
    // and an unsaved buffer would be debugged as its previous contents.
    const open = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
    if (open?.isDirty) { await open.save(); }

    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${fileName}`,
        program: fileName,
    });
}

/**
 * Debug a named `.bas` file - the "🐞 Debug in emulator" CodeLens at the top
 * of it.
 *
 * Unlike {@link debugAssembly}, there is no entry-point resolution: a `.bas`
 * file is never `#include`d into another, so the file the lens sits in is
 * always the program to run.
 */
export async function debugBasic(fileName?: string): Promise<void> {
    if (!fileName) {
        await debugActiveFile();
        return;
    }
    const uri = vscode.Uri.file(fileName);
    // Saving first is not politeness: the adapter loads the file *on disk*,
    // and an unsaved buffer would be debugged as its previous contents.
    const open = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
    if (open?.isDirty) { await open.save(); }

    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${fileName}`,
        program: fileName,
    });
}

/**
 * Run or debug a raw `.sna` snapshot directly - no build, no source, no
 * assembly. `stopOnEntry` is the entire difference between the two: a
 * snapshot's own `PC` is already mid-program, so "run" is just "don't stop
 * before executing it", the same existing launch property `debugAssembly`'s
 * own `stopOnEntry: false` default already gives .asm/.bas files - nothing
 * new needed on the adapter side for that half. With no `fileName` (the
 * Command Palette case), `pickWorkspaceFile` offers every `.sna` in the
 * workspace plus a file-browser fallback.
 */
export async function debugSnapshot(fileName?: string, stopOnEntry = true): Promise<void> {
    if (!fileName) {
        fileName = await pickWorkspaceFile({
            glob: '**/*.sna',
            browseLabel: '$(folder-opened) Browse for a .sna file...',
            placeHolder: 'Which .sna snapshot should be run or debugged?',
            dialogFilters: { 'CPC snapshot': ['sna'] },
            dialogOpenLabel: 'Run/Debug',
        });
        if (!fileName) { return; }
    }
    const uri = vscode.Uri.file(fileName);
    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `${stopOnEntry ? 'Debug' : 'Run'} ${fileName}`,
        program: fileName,
        stopOnEntry,
    });
}

/**
 * Run or debug a raw `.dsk` disk image directly - mounted in drive A at a
 * cold boot, landing at `Ready` exactly like a real machine with a disk in
 * the drive and no `!BOOT` file: nothing auto-runs, `RUN"..."` is still the
 * user's job. Unlike {@link debugSnapshot}, `stopOnEntry` makes no real
 * difference here (there is no known entry point for a raw disk to stop
 * at) - kept for symmetry with the `.sna` command pair and because the
 * adapter already degrades a no-op `stopOnEntry` to a harmless notice rather
 * than an error. With no `fileName` (the Command Palette case),
 * `pickWorkspaceFile` offers every `.dsk` in the workspace plus a
 * file-browser fallback.
 */
export async function debugDisk(fileName?: string, stopOnEntry = true): Promise<void> {
    if (!fileName) {
        fileName = await pickWorkspaceFile({
            glob: '**/*.dsk',
            browseLabel: '$(folder-opened) Browse for a .dsk file...',
            placeHolder: 'Which .dsk disk should be run or debugged?',
            dialogFilters: { 'CPC disk': ['dsk'] },
            dialogOpenLabel: 'Run/Debug',
        });
        if (!fileName) { return; }
    }
    const uri = vscode.Uri.file(fileName);
    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `${stopOnEntry ? 'Debug' : 'Run'} ${fileName}`,
        program: fileName,
        stopOnEntry,
    });
}

interface DebuggableRule { rule: string; buildFile: string }

/** Minimal shape this module needs from the language client - kept narrow
 * (rather than importing the full `LanguageClient` type) so this module
 * stays decoupled from `lsp/client.ts`'s own construction sequencing; see
 * {@link setDebugClient}. */
let client: { sendRequest: <T>(method: string, param: unknown) => Promise<T> } | undefined;

/** Give this module the language client, so it can ask the server questions. */
export function setDebugClient(
    languageClient: { sendRequest: <T>(method: string, param: unknown) => Promise<T> },
): void {
    client = languageClient;
}

/** The rules the server considers debuggable, across the open build files. */
async function debuggableRules(): Promise<DebuggableRule[]> {
    if (!client) { return []; }
    try {
        return await client.sendRequest<DebuggableRule[]>(
            'workspace/executeCommand',
            { command: 'cpclib.getDebuggableRules', arguments: [] },
        ) ?? [];
    } catch {
        return [];
    }
}

/**
 * Start a debug session from a bndbuild rule that launches an emulator.
 *
 * With no rule given, the server is asked which ones actually end in an
 * emulator command and the list is offered - remembering rule names is not the
 * user's job, and typing one that cannot be debugged only fails later.
 */
export async function debugRule(rule?: string, buildFile?: string): Promise<void> {
    if (!rule) {
        const candidates = await debuggableRules();
        if (candidates.length === 0) {
            void vscode.window.showWarningMessage(
                'No rule in the open build files launches an emulator with `run`.',
            );
            return;
        }
        const picked = await vscode.window.showQuickPick(
            candidates.map(c => ({
                label: c.rule,
                description: vscode.workspace.asRelativePath(c.buildFile),
                candidate: c,
            })),
            { placeHolder: 'Which rule should be debugged?' },
        );
        if (!picked) { return; }
        rule = picked.candidate.rule;
        buildFile = picked.candidate.buildFile;
    }
    await vscode.debug.startDebugging(undefined, {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${rule}`,
        rule,
        ...(buildFile ? { buildFile } : {}),
    });
}
