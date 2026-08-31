import * as vscode from 'vscode';
import { workspace, ExtensionContext } from 'vscode';
import * as path from 'path';
import { bndbuildCommandPrefix, buildBndbuildTask, BndbuildTaskProvider } from './bndbuildTaskProvider';

// Every currently-open `EmbeddedRulePseudoterminal`'s writer, so
// `installLogMessageMirror`'s single `window/logMessage` handler can push
// each line into whichever embedded-rule task terminal(s) are running right
// now, live, as `cpclib.runRule` streams them - not just a "check the
// output channel instead" pointer message. A `Set` rather than a single
// slot: nothing stops two embedded-rule tasks from running concurrently
// (each click starts its own), and a message arriving while more than one
// is active has no server-side tag saying which task it belongs to, so it
// goes to all of them - a rare, cosmetic-only interleaving, not a
// correctness issue (the real, authoritative output is still also always in
// the "CPClib LSP" output channel).
export const activeEmbeddedTaskWriters = new Set<(text: string) => void>();

/// A `vscode.Pseudoterminal` wrapping the LSP's own `cpclib.runRule` command
/// - the execution mechanism for a `#!bndbuild`-embedded rule in a `.asm`
/// file, which (unlike a real `.bnd` file) has no on-disk YAML file a
/// `ShellExecution` could target, so it can't become a real terminal Task
/// the way `buildBndbuildTask`'s tasks are. While `cpclib.runRule` runs,
/// this terminal receives a live mirror of the same build output the
/// "CPClib LSP" output channel gets (via `installLogMessageMirror`), so an
/// embedded rule's task terminal behaves like a real one rather than a bare
/// "see elsewhere" pointer.
class EmbeddedRulePseudoterminal implements vscode.Pseudoterminal {
    private readonly writeEmitter = new vscode.EventEmitter<string>();
    private readonly closeEmitter = new vscode.EventEmitter<number>();
    onDidWrite: vscode.Event<string> = this.writeEmitter.event;
    onDidClose: vscode.Event<number> = this.closeEmitter.event;

    constructor(
        private readonly rule: string,
        private readonly hostFilePath: string,
    ) {}

    async open(): Promise<void> {
        const write = (text: string) => this.writeEmitter.fire(text);
        activeEmbeddedTaskWriters.add(write);
        this.writeEmitter.fire(`Running embedded bndbuild rule '${this.rule}' from ${this.hostFilePath}\r\n`);
        try {
            await vscode.commands.executeCommand('cpclib.runRule', this.rule, this.hostFilePath);
            this.closeEmitter.fire(0);
        } catch (err) {
            this.writeEmitter.fire(`Failed to run: ${err}\r\n`);
            this.closeEmitter.fire(1);
        } finally {
            activeEmbeddedTaskWriters.delete(write);
        }
    }

    close(): void {}
}

/// Builds the `vscode.Task` for a `#!bndbuild`-embedded rule - see
/// `EmbeddedRulePseudoterminal`'s own doc comment for why this needs a
/// `CustomExecution` instead of `buildBndbuildTask`'s `ShellExecution`.
export function buildEmbeddedRuleTask(rule: string, hostFilePath: string, taskName: string): vscode.Task {
    const def: vscode.TaskDefinition = {
        type: BndbuildTaskProvider.taskType,
        target: rule,
        file: hostFilePath,
        embedded: true,
    };
    const execution = new vscode.CustomExecution(
        async () => new EmbeddedRulePseudoterminal(rule, hostFilePath),
    );
    const task = new vscode.Task(def, vscode.TaskScope.Workspace, taskName, 'bndbuild', execution);
    task.group = vscode.TaskGroup.Build;
    return task;
}

// ── "▶ Run" CodeLens execution for a real on-disk .bnd file's rule ─────────
//
// `cpclib.runRuleInTerminal(target, filePath)`: a client-only command (never
// sent to the server, deliberately absent from the server's
// `executeCommandProvider.commands`) invoked by the bndbuild file's
// rule-level "▶ Run" CodeLens. Runs the same `bndbuild` CLI invocation as
// `BndbuildTaskProvider`, via a real VS Code Task/terminal, so build errors
// get clickable Problems-panel entries through the already-working `$basm`
// problemMatcher - the LSP's own `cpclib.runRule` streaming path proved
// unreliable at making its own diagnostics clickable.
async function runRuleInTerminal(target: string, filePath: string): Promise<void> {
    const config = workspace.getConfiguration('cpclib-lsp');
    const bndbuildCommand = bndbuildCommandPrefix(config);
    const task = buildBndbuildTask(target, filePath, bndbuildCommand, target);
    await vscode.tasks.executeTask(task);
}

/// Builds the `vscode.Task` for `cpclib.runTaskInTerminal` - `--only-task
/// RULE:INDEX` (`cpclib-bndbuild`'s `BndBuilder::execute_task`) runs just
/// that one task, with the *same* Jinja/automatic-variable context a normal
/// rule build gets (unlike `--direct`, which runs a raw, unexpanded command
/// string - see that method's own doc comment for why that distinction
/// matters), and bypasses dependency resolution/up-to-date checks entirely,
/// so it runs even when the rule's target already exists.
function buildBndbuildOnlyTaskTask(
    rule: string,
    filePath: string,
    taskIndex: number,
    bndbuildCommand: string,
    taskName: string,
): vscode.Task {
    const workDir  = path.dirname(filePath);
    const fileName = path.basename(filePath);
    const def: vscode.TaskDefinition = {
        type: BndbuildTaskProvider.taskType,
        target: rule,
        file: filePath,
    };
    const task = new vscode.Task(
        def,
        vscode.TaskScope.Workspace,
        taskName,
        'bndbuild',
        new vscode.ShellExecution(
            `${bndbuildCommand} -f "${fileName}" --only-task "${rule}:${taskIndex}"`,
            { cwd: workDir },
        ),
        '$basm',
    );
    task.group = vscode.TaskGroup.Build;
    return task;
}

// `cpclib.runTaskInTerminal(rule, filePath, taskIndex)`: the per-command
// "▶ Run this command" CodeLens's terminal-based counterpart to
// `cpclib.runRuleInTerminal`, for a rule in a real on-disk .bnd file - uses
// the very same mechanism (a real Task/terminal, `$basm` problemMatcher) as
// the rule-level runner, per the same reasoning: the LSP's own
// `cpclib.runTask` streaming path doesn't surface clickable errors as
// reliably as a real terminal does. The embedded-bndbuild-in-.asm-block
// CodeLens (no on-disk .bnd file for a CLI invocation to target) still uses
// the LSP path - there's no terminal equivalent for that case.
async function runTaskInTerminal(rule: string, filePath: string, taskIndex: number): Promise<void> {
    const config = workspace.getConfiguration('cpclib-lsp');
    const bndbuildCommand = bndbuildCommandPrefix(config);
    const task = buildBndbuildOnlyTaskTask(rule, filePath, taskIndex, bndbuildCommand, `${rule} #${taskIndex + 1}`);
    await vscode.tasks.executeTask(task);
}

/** Registers `cpclib.runRuleInTerminal`/`cpclib.runTaskInTerminal`, the
 * terminal-based CodeLens runners for a real on-disk `.bnd` file's rules. */
export function registerCodeLensRunners(context: ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.runRuleInTerminal', runRuleInTerminal),
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.runTaskInTerminal', runTaskInTerminal),
    );
}
