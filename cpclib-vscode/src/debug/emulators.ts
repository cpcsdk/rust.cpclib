import * as vscode from 'vscode';
import * as path from 'path';
import { execFile } from 'child_process';
import { DEBUG_TYPE } from './launch';

/** One row of `cpclib-lsp emu-list`'s JSON - see its own doc comment
 * (`cpclib-runner::emucontrol::EmulatorListEntry`) for what each field
 * means. */
interface EmulatorEntry {
    id: string;
    label: string;
    debuggable: boolean;
    installed: boolean;
    dapId: string | null;
}

/**
 * Every emulator `cpclib-runner` knows how to install and run, asked of the
 * same `cpclib-lsp` binary that already serves the language server and the
 * debug adapter - no second binary to locate, and install status is always
 * current (asked fresh every time, not cached at activation) since an
 * emulator installed in a previous session should show as installed
 * without a reload.
 */
function listEmulators(serverPath: string): Promise<EmulatorEntry[]> {
    return new Promise((resolve, reject) => {
        execFile(serverPath, ['emu-list'], (error, stdout) => {
            if (error) {
                reject(error);
                return;
            }
            try {
                resolve(JSON.parse(stdout) as EmulatorEntry[]);
            }
            catch (parseError) {
                reject(parseError);
            }
        });
    });
}

/**
 * Offers a QuickPick of every compatible emulator, each labelled with
 * whether it is already installed - a real submenu can't show that (VS
 * Code menu contributions are static, install status is not), so this is
 * the "select a compatible emulator" affordance for
 * {@link debugWithEmulator}/{@link runWithEmulator} alike.
 *
 * `debugOnly` narrows to the two emulators the DAP layer actually speaks
 * to (`cpclib-dap/src/lib.rs`'s own check) - a "compatible emulator" for
 * debugging is one of those two, already installed or not; for running,
 * every emulator this crate knows is compatible.
 */
async function pickEmulator(
    serverPath: string,
    debugOnly: boolean,
): Promise<EmulatorEntry | undefined> {
    let entries: EmulatorEntry[];
    try {
        entries = await listEmulators(serverPath);
    }
    catch (error) {
        void vscode.window.showErrorMessage(`Could not list emulators: ${error}`);
        return undefined;
    }
    const candidates = debugOnly ? entries.filter(e => e.debuggable) : entries;
    if (candidates.length === 0) {
        void vscode.window.showWarningMessage('No compatible emulator was found.');
        return undefined;
    }
    const picked = await vscode.window.showQuickPick(
        candidates.map(e => ({
            label: `${e.installed ? '$(pass-filled)' : '$(cloud-download)'} ${e.label}`,
            description: e.installed ? 'installed' : 'not installed - fetched on first use',
            entry: e,
        })),
        {
            placeHolder: debugOnly
                ? 'Which emulator should debug this file?'
                : 'Which emulator should run this file?',
        },
    );
    return picked?.entry;
}

/**
 * Debug a `.sna`/`.dsk` file with a *named* emulator - the DAP-side
 * `emulator` launch property, `dapId` (`cpclib-dap/src/lib.rs`'s own
 * `"1984js"`/`"amspiritlite"` strings, from {@link EmulatorEntry.dapId}).
 * Shared by the native per-emulator context-menu commands (already know
 * which one) and {@link debugWithEmulator} (asks first, via a QuickPick).
 */
async function debugWithEmulatorId(
    target: string | vscode.Uri | undefined,
    dapId: string,
    label: string,
): Promise<void> {
    const fileName = target instanceof vscode.Uri ? target.fsPath : target;
    if (!fileName) { return; }
    const uri = vscode.Uri.file(fileName);
    await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(uri), {
        type: DEBUG_TYPE,
        request: 'launch',
        name: `Debug ${fileName} with ${label}`,
        program: fileName,
        stopOnEntry: true,
        emulator: dapId,
    });
}

/**
 * Debug a `.sna`/`.dsk` file with an emulator picked from a QuickPick -
 * the Command Palette form of the same action the native "Debug with..."
 * context submenu offers per-emulator (that submenu's items are static VS
 * Code menu contributions and so cannot ask which file first; from the
 * Palette there is no file already in hand, so asking here instead is the
 * only option). Only the two DAP-debuggable emulators are offered.
 */
export async function debugWithEmulator(
    target: string | vscode.Uri | undefined,
    getServerPath: () => string,
): Promise<void> {
    const picked = await pickEmulator(getServerPath(), true);
    if (!picked?.dapId) { return; }
    await debugWithEmulatorId(target, picked.dapId, picked.label);
}

/**
 * Run a `.sna`/`.dsk` file with a *named* emulator - `id`, the exact
 * string `cpclib-lsp emu --emulator` accepts ({@link EmulatorEntry.id}).
 * Bypasses the debug adapter entirely (which silently substitutes 1984js
 * for anything it doesn't recognise - see {@link runWithEmulator}'s own
 * doc comment) and shells out to `cpclib-lsp emu`, the same already-tested
 * code path this crate's own standalone `emu` CLI and bndbuild's `emu`
 * runner already use for arbitrary emulator launches.
 */
async function runWithEmulatorId(
    target: string | vscode.Uri | undefined,
    getServerPath: () => string,
    id: string,
    label: string,
): Promise<void> {
    const fileName = target instanceof vscode.Uri ? target.fsPath : target;
    if (!fileName) { return; }

    const fileFlag = path.extname(fileName).toLowerCase() === '.dsk' ? '--drivea' : '--snapshot';
    const task = new vscode.Task(
        { type: 'cpclib-emu', file: fileName, emulator: id },
        vscode.TaskScope.Workspace,
        `Run ${path.basename(fileName)} in ${label}`,
        'cpclib',
        new vscode.ShellExecution(
            `"${getServerPath()}" emu --emulator ${id} ${fileFlag} "${fileName}" run`,
        ),
    );
    await vscode.tasks.executeTask(task);
}

/**
 * Run a `.sna`/`.dsk` file with an emulator picked from a QuickPick - the
 * Command Palette form of the "Run with..." context submenu (see
 * {@link debugWithEmulator}'s own doc comment for why the Palette needs a
 * picker where the submenu doesn't). Every installed-or-installable
 * emulator is offered, not only the two debug-capable ones.
 */
export async function runWithEmulator(
    target: string | vscode.Uri | undefined,
    getServerPath: () => string,
): Promise<void> {
    const picked = await pickEmulator(getServerPath(), false);
    if (!picked) { return; }
    await runWithEmulatorId(target, getServerPath, picked.id, picked.label);
}

/**
 * Every emulator id the native context submenus were declared for in
 * package.json, kept in exactly one place so the per-emulator command
 * registrations below and the `setContext` calls that drive their
 * installed/needs-install icons (the `"cpclib.emu.<id>.installed"` key
 * {@link refreshEmulatorInstallContext} sets) can't drift apart. `dapId` is set
 * only for the two DAP-debuggable ones (matches {@link EmulatorEntry}).
 *
 * `id` deliberately keeps dot-namespacing in the generated command ids
 * (`cpclib.runWith.ace`, `cpclib.debugWith.amspiritlite`, ...) rather than
 * the flat camelCase most other commands use - a deliberate choice for a
 * large generated command family, not an oversight (see the refactor plan's
 * Phase 2 item 6).
 *
 * `id` (`emulator1984-js`, this crate's own `--emulator` CLI value - see
 * `cpclib-runner::emucontrol::Emu::Emulator1984Js`) and `dapId` (`1984js`,
 * `cpclib-dap`'s own launch-time `emulator` property) are *not* the same
 * spelling by accident: `EmulatorListEntry`'s own doc comment
 * (`cpclib-runner/src/emucontrol.rs`) states outright that they are two
 * deliberately independent naming schemes for two different backend
 * contracts (the CLI's `--emulator` flag vs. the DAP's `emulator` launch
 * property) - normalizing them to one spelling here would silently break
 * whichever call site expects the other string, so both are kept exactly as
 * the Rust side defines them.
 */
export const EMULATOR_MENU_ENTRIES: { id: string; label: string; dapId?: string }[] = [
    { id: 'ace', label: 'ACE-DL' },
    { id: 'amspirit', label: 'AMSpiriT' },
    { id: 'amspiritlite', label: 'AMSpiriT Lite', dapId: 'amspiritlite' },
    { id: 'winape', label: 'WinAPE' },
    { id: 'cpcec', label: 'CPCEC' },
    { id: 'sugarbox', label: 'SugarBox v2' },
    { id: 'cpcemupower', label: 'CPCEmuPower' },
    { id: 'cpcemu', label: 'CPCEmu' },
    { id: 'caprice', label: 'CaPriCe Forever' },
    { id: 'cadence', label: 'Cadence' },
    { id: 'emulator1984', label: '1984 (native)' },
    { id: 'emulator1984-js', label: '1984js (browser)', dapId: '1984js' },
    { id: 'rvm', label: 'Retro Virtual Machine' },
];

/**
 * Refreshes the `cpclib.emu.<id>.installed` context key every native
 * "Run with.../Debug with..." submenu item's `when` clause is gated on -
 * this is what picks the `$(pass-filled)`/`$(cloud-download)` variant VS
 * Code actually shows, since a static menu contribution cannot compute its
 * own icon at open time. Best-effort, not live: a submenu opening does not
 * itself fire anything this extension can observe, so this only runs at
 * activation and again after each run/debug-with-emulator command
 * completes (the one point an install this session might just have
 * happened). An emulator installed through some other means entirely
 * stays stale here until the next one of those.
 */
export async function refreshEmulatorInstallContext(serverPath: string): Promise<void> {
    let entries: EmulatorEntry[];
    try {
        entries = await listEmulators(serverPath);
    }
    catch {
        return;
    }
    const byId = new Map(entries.map(e => [e.id, e.installed]));
    for (const { id } of EMULATOR_MENU_ENTRIES) {
        await vscode.commands.executeCommand(
            'setContext',
            `cpclib.emu.${id}.installed`,
            byId.get(id) ?? false,
        );
    }
}

/**
 * Registers the native per-emulator "Run with.../Debug with..." context
 * submenu commands (`cpclib.runWith.<id>`/`cpclib.debugWith.<dapId>`, plus
 * their `.needsInstall` variants) - one pair per {@link EMULATOR_MENU_ENTRIES}
 * entry, so right-clicking a `.sna`/`.dsk` reaches every compatible emulator
 * without the QuickPick step {@link debugWithEmulator}/{@link runWithEmulator}
 * need. The install-state context keys these `when` clauses read are
 * refreshed once now and again after every one of these fires, in case it
 * just installed something.
 */
export function registerEmulatorCommands(
    context: vscode.ExtensionContext,
    resolveAdapterPath: () => string,
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'cpclib.debugWithEmulator',
            (target?: string | vscode.Uri) => debugWithEmulator(target, resolveAdapterPath),
        ),
        vscode.commands.registerCommand(
            'cpclib.runWithEmulator',
            (target?: string | vscode.Uri) => runWithEmulator(target, resolveAdapterPath),
        ),
        ...EMULATOR_MENU_ENTRIES.flatMap(({ id, label, dapId }) => {
            const disposables = [
                vscode.commands.registerCommand(
                    `cpclib.runWith.${id}`,
                    async (target?: string | vscode.Uri) => {
                        await runWithEmulatorId(target, resolveAdapterPath, id, label);
                        await refreshEmulatorInstallContext(resolveAdapterPath());
                    },
                ),
                vscode.commands.registerCommand(
                    `cpclib.runWith.${id}.needsInstall`,
                    async (target?: string | vscode.Uri) => {
                        await runWithEmulatorId(target, resolveAdapterPath, id, label);
                        await refreshEmulatorInstallContext(resolveAdapterPath());
                    },
                ),
            ];
            if (dapId) {
                disposables.push(
                    vscode.commands.registerCommand(
                        `cpclib.debugWith.${dapId}`,
                        async (target?: string | vscode.Uri) => {
                            await debugWithEmulatorId(target, dapId, label);
                            await refreshEmulatorInstallContext(resolveAdapterPath());
                        },
                    ),
                    vscode.commands.registerCommand(
                        `cpclib.debugWith.${dapId}.needsInstall`,
                        async (target?: string | vscode.Uri) => {
                            await debugWithEmulatorId(target, dapId, label);
                            await refreshEmulatorInstallContext(resolveAdapterPath());
                        },
                    ),
                );
            }
            return disposables;
        }),
    );

    // First paint for the native submenus' installed/needs-install icons -
    // without this every entry shows "needs install" until the first
    // run/debug-with-emulator command happens to run once.
    void refreshEmulatorInstallContext(resolveAdapterPath());
}
