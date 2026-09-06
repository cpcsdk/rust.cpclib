import * as vscode from 'vscode';
import { EMULATOR_MENU_ENTRIES, refreshEmulatorInstallContext } from '../debug/emulators';

/**
 * The ids `Emulator::accept_csl()` (`cpclib-runner/src/runner/emulator.rs`)
 * returns `true` for - AMSpiriT (`--csl=<path>`) and SugarboxV2 (`-s`/`--csl
 * <path>`), confirmed against each emulator's own documented CLI flags.
 * Kept in sync manually with that Rust source of truth (same convention as
 * `SUPPORTED_AUTO_RUN_EMULATORS` elsewhere in this extension) - everyone
 * else runs a `.csl` file through `cpclib_runner::csl_interpreter` instead
 * (leading disk/snapshot instructions folded into launch args,
 * `key_output`/`wait*` replayed live; a mid-script media/reset change has
 * no live backend and is reported, not executed).
 */
const NATIVE_CSL_IDS = new Set(['amspirit', 'sugarbox']);

/**
 * `1984js` is served in a browser, never spawned as a process - `cpclib-lsp
 * emu --csl` refuses it outright (`emucontrol::emulator_from_choice`'s own
 * error), so it has no place in a "run this CSL file with..." picker at
 * all (unlike the `.sna`/`.dsk` "Run with..." menu, which is a subset of
 * every known emulator only because 1984js there gets served instead of
 * refused - CSL has no such fallback).
 */
const CSL_CAPABLE_EMULATORS = EMULATOR_MENU_ENTRIES.filter(e => e.id !== 'emulator1984-js');

/** One codicon prefix marking whether `id` runs a CSL script directly
 * (native) or through the interpreter (degraded - only `key_output`/
 * `wait*` replay live, a mid-script media/reset change is skipped). */
function cslSupportIcon(id: string): string {
    return NATIVE_CSL_IDS.has(id) ? '$(zap)' : '$(warning)';
}

/**
 * Run a `.csl` file with a *named* emulator - `id`, the exact string
 * `cpclib-lsp emu --emulator` accepts. Always goes through `--csl`
 * (`emucontrol::run_csl_file`), which itself picks the native `--csl=`
 * launch or the CSL-interpreter fallback depending on `id`.
 */
function runCslWithEmulatorId(
    target: string | vscode.Uri | undefined,
    getServerPath: () => string,
    id: string,
    label: string,
): void {
    const fileName = target instanceof vscode.Uri ? target.fsPath : target;
    if (!fileName) { return; }

    const task = new vscode.Task(
        { type: 'cpclib-emu-csl', file: fileName, emulator: id },
        vscode.TaskScope.Workspace,
        `Run ${fileName} in ${label}`,
        'cpclib',
        new vscode.ShellExecution(
            `"${getServerPath()}" emu --emulator ${id} --csl "${fileName}" run`,
        ),
    );
    void vscode.tasks.executeTask(task);
}

/**
 * Run a `.csl` file with an emulator picked from a QuickPick - the Command
 * Palette form of the "Run CSL with..." context submenu (a static menu
 * contribution can't show live install status, so the submenu's `$(zap)`/
 * `$(warning)` marker for native-vs-interpreted support is baked in at
 * `package.json` build time per id, while this picker computes both
 * markers fresh). Every CSL-capable emulator is offered, native and
 * interpreted alike - the interpreted ones are still genuinely useful for
 * `key_output`/`wait*`-only scripts, just marked as reduced fidelity.
 */
export async function runCslWithEmulator(
    target: string | vscode.Uri | undefined,
    getServerPath: () => string,
): Promise<void> {
    const picked = await vscode.window.showQuickPick(
        CSL_CAPABLE_EMULATORS.map(e => ({
            label: `${cslSupportIcon(e.id)} ${e.label}`,
            description: NATIVE_CSL_IDS.has(e.id)
                ? 'native CSL support'
                : 'via CSL interpreter - key_output/wait only, no mid-script media/reset',
            entry: e,
        })),
        { placeHolder: 'Which emulator should run this CSL script?' },
    );
    if (!picked) { return; }
    runCslWithEmulatorId(target, getServerPath, picked.entry.id, picked.entry.label);
}

/**
 * Registers the native per-emulator "Run CSL with..." context submenu
 * commands (`cpclib.runCslWith.<id>`, plus its `.needsInstall` variant for
 * the "not installed" icon `package.json` shows) - one pair per
 * {@link CSL_CAPABLE_EMULATORS} entry, so right-clicking a `.csl` file
 * reaches every capable emulator without the QuickPick step
 * {@link runCslWithEmulator} needs. Reuses `debug/emulators.ts`'s own
 * `cpclib.emu.<id>.installed` context keys (refreshed there) rather than
 * tracking install state twice.
 */
export function registerCslEmulatorCommands(
    context: vscode.ExtensionContext,
    getServerPath: () => string,
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'cpclib.runCslWithEmulator',
            (target?: string | vscode.Uri) => runCslWithEmulator(target, getServerPath),
        ),
        ...CSL_CAPABLE_EMULATORS.flatMap(({ id, label }) => [
            vscode.commands.registerCommand(
                `cpclib.runCslWith.${id}`,
                async (target?: string | vscode.Uri) => {
                    runCslWithEmulatorId(target, getServerPath, id, label);
                    await refreshEmulatorInstallContext(getServerPath());
                },
            ),
            vscode.commands.registerCommand(
                `cpclib.runCslWith.${id}.needsInstall`,
                async (target?: string | vscode.Uri) => {
                    runCslWithEmulatorId(target, getServerPath, id, label);
                    await refreshEmulatorInstallContext(getServerPath());
                },
            ),
        ]),
    );
}
