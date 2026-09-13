import * as vscode from 'vscode';
import { workspace, ExtensionContext, window } from 'vscode';
import { client } from '../lsp/client';

/// One dead label, as the server reports it.
type UnreferencedLabel = {
    name: string;
    uri: string;
    line: number;
    character: number;
};

/// `file.asm:42`, the form that is worth reading in a notification.
function findingLabel(finding: UnreferencedLabel): string {
    const name = vscode.Uri.parse(finding.uri).path.split('/').pop() ?? finding.uri;
    return `${name}:${finding.line + 1}`;
}

/// Ask the server for every global label, anywhere in the workspace reached
/// from the active file (its own file, its `INCLUDE`s, and every other
/// `.asm` under the workspace root), whose only occurrence anywhere is its
/// own definition.
///
/// Unlike the "N references" CodeLens above each label (always on, scoped
/// cheaply to the current file plus its own direct includes so it stays
/// affordable on every keystroke), this is the accurate, whole-workspace
/// answer - a real text scan of every candidate file, so it's asked for
/// explicitly rather than kept live.
async function findUnreferencedLabels(): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor) {
        window.showInformationMessage('Open an assembly file first.');
        return;
    }
    if (!/\.(asm|z80)$/i.test(editor.document.uri.fsPath)) {
        window.showInformationMessage('The active file is not an assembly file.');
        return;
    }

    let findings: UnreferencedLabel[];
    try {
        findings = await client.sendRequest<UnreferencedLabel[] | null>(
            'workspace/executeCommand',
            {
                command: 'cpclib.findUnreferencedLabels',
                arguments: [editor.document.uri.toString()],
            },
        ) ?? [];
    } catch (err) {
        window.showErrorMessage(`Could not scan for unreferenced labels: ${(err as Error).message}`);
        return;
    }

    if (findings.length > 0) {
        client.outputChannel.appendLine('Labels with no reference:');
        for (const finding of findings) {
            client.outputChannel.appendLine(`  ${findingLabel(finding)}  ${finding.name}`);
        }
    }

    if (findings.length === 0) {
        window.showInformationMessage('No unreferenced labels found.');
        return;
    }

    const count = `${findings.length} unreferenced label${findings.length === 1 ? '' : 's'}`;
    const inline = findings.length <= 3
        ? ` (${findings.map(f => f.name).join(', ')})`
        : '';
    const choice = await window.showInformationMessage(`${count} found${inline}.`, 'Go to…');
    if (choice !== 'Go to…') {
        return;
    }
    await pickFinding(findings);
}

/// A jumpable list of findings, previewing each one as it is highlighted -
/// mirrors `peephole.ts`'s own `pickFinding`.
async function pickFinding(findings: UnreferencedLabel[]): Promise<void> {
    type Item = vscode.QuickPickItem & { finding: UnreferencedLabel };
    const items: Item[] = findings.map(finding => ({
        label: finding.name,
        description: findingLabel(finding),
        finding,
    }));

    const reveal = async (finding: UnreferencedLabel) => {
        const document = await workspace.openTextDocument(vscode.Uri.parse(finding.uri));
        const position = new vscode.Position(finding.line, finding.character);
        await window.showTextDocument(document, {
            selection: new vscode.Range(position, position),
            preview: true,
        });
    };

    const picker = window.createQuickPick<Item>();
    picker.items = items;
    picker.placeholder = 'Labels with no reference';
    picker.matchOnDescription = true;
    picker.onDidChangeActive(active => {
        if (active[0]) {
            void reveal(active[0].finding);
        }
    });
    picker.onDidAccept(() => {
        picker.hide();
    });
    picker.onDidHide(() => picker.dispose());
    picker.show();
}

/// A `Location`, exactly as the server serializes one (`tower_lsp::lsp_types::Location`):
/// a plain URI string, not a `vscode.Uri`.
type PlainLocation = {
    uri: string;
    range: { start: { line: number; character: number }; end: { line: number; character: number } };
};

/// `editor.action.showReferences` is a VS Code built-in that validates its
/// own arguments with `arg instanceof vscode.Uri`/`Position`/`Location` -
/// real class instances, not plain JSON objects. A CodeLens `Command`
/// resolved straight from the language server's response never satisfies
/// that (`vscode-languageclient` hands a click's arguments through exactly
/// as the server sent them, with no such reconstruction), so pointing a
/// server-generated CodeLens directly at the built-in throws a constraint
/// error the moment it's clicked. This command is the fix: it receives the
/// same plain JSON the server sent and rebuilds real instances before
/// forwarding to the built-in - the reference-count CodeLens
/// (`cpclib-lsp/src/basm/references_lens.rs`) targets this command, never
/// the built-in one directly.
function showReferenceLocations(uriString: string, position: { line: number; character: number }, locations: PlainLocation[]): Thenable<unknown> {
    const uri = vscode.Uri.parse(uriString);
    const pos = new vscode.Position(position.line, position.character);
    const locs = locations.map(l => new vscode.Location(
        vscode.Uri.parse(l.uri),
        new vscode.Range(
            new vscode.Position(l.range.start.line, l.range.start.character),
            new vscode.Position(l.range.end.line, l.range.end.character),
        ),
    ));
    return vscode.commands.executeCommand('editor.action.showReferences', uri, pos, locs);
}

// `cpclib.findUnreferencedLabelsInProject` is deliberately not
// `cpclib.findUnreferencedLabels` - that name is already advertised by the
// server's own `executeCommandProvider`, and `vscode-languageclient`
// auto-registers a bridge command for every one of those; a second
// `registerCommand` under the same id throws "command already exists" and
// aborts the whole client start (see `peephole.ts`'s own comment on this,
// and `no_advertised_command_is_also_registered_by_the_vscode_extension` in
// `cpclib-lsp`). `cpclib.showReferenceLocations` has no such conflict - it's
// never in `executeCommandProvider.commands` (client-side-only, like
// `cpclib.debugAssembly`/`cpclib.runAsm` in `debug/register.ts`).
export function registerReferences(context: ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.findUnreferencedLabelsInProject', findUnreferencedLabels),
        vscode.commands.registerCommand('cpclib.showReferenceLocations', showReferenceLocations),
    );
}
