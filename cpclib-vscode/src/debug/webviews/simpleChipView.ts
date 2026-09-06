import * as vscode from 'vscode';
import { escapeHtml } from '../../shared/html';

/** A register shown as-is: an already-formatted `{name, value}` pair, no
 * client-side decoding. Shared shape for every "simple" chip view (PSG,
 * FDC) - see `simple_chip_view_command`'s own doc comment
 * (`cpclib-dap/src/session.rs`) for why these stay unstructured for now. */
export interface SimpleChipDump {
    registers: { name: string; value: string }[];
}

/** What makes one simple chip view different from another - a title, a panel
 * type id, and the console command that reopens it. */
export interface SimpleChipViewKind {
    /** `vscode.window.createWebviewPanel`'s own `viewType` - must be unique
     * across every panel kind, chip views included. */
    viewType: string;
    /** Shown in the panel's tab and the `<h2>` heading, e.g. "PSG". */
    title: string;
    /** The debug-console command that (re)opens this view, e.g. `-psgview` -
     * named in the panel's own footer so a stale pane says how to refresh
     * itself. */
    command: string;
}

const panelsByKind = new Map<string, Map<string, vscode.WebviewPanel>>();

function panelsFor(kind: SimpleChipViewKind): Map<string, vscode.WebviewPanel> {
    let panels = panelsByKind.get(kind.viewType);
    if (!panels) {
        panels = new Map();
        panelsByKind.set(kind.viewType, panels);
    }
    return panels;
}

/**
 * Show a chip's registers exactly as the emulator/snapshot reports them -
 * already-formatted hex+decimal strings. Shared by every simple chip view
 * (PSG, FDC) since they differ only in title/command, never in how the
 * register grid itself is rendered - one implementation rather than one
 * copy per chip, mirroring `simple_chip_view_command`'s own reasoning on
 * the Rust side.
 *
 * `extraHtml`, when given, is rendered below the register grid, inside the
 * same document/style sheet - used by `fdc.ts` for its own real sector
 * table, which does not fit a flat `{name, value}` grid. Trusted, pre-built
 * HTML: the caller is responsible for escaping anything it embeds.
 *
 * Opened by its own console command; like the CRTC panel it is not re-read
 * on every stop - re-run the command for a fresh look.
 */
export function showSimpleChipView(
    kind: SimpleChipViewKind,
    session: vscode.DebugSession,
    dump: SimpleChipDump | undefined,
    extraHtml?: string,
): void {
    if (!dump || !Array.isArray(dump.registers)) { return; }

    const panels = panelsFor(kind);
    let panel = panels.get(session.id);
    if (!panel) {
        panel = vscode.window.createWebviewPanel(
            kind.viewType,
            `CPC ${kind.title} — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: false, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (panels.get(session.id) === owned) { panels.delete(session.id); }
        });
        panels.set(session.id, panel);
    }

    panel.webview.html = simpleChipViewHtml(kind, dump, extraHtml ?? '');
    // Every call is a person asking, not a silent per-stop refresh - always
    // comes forward, same as the CRTC panel.
    panel.reveal(vscode.ViewColumn.Beside, true);
}

function simpleChipViewHtml(kind: SimpleChipViewKind, dump: SimpleChipDump, extraHtml: string): string {
    const cells = dump.registers.map(reg =>
        `<div class="reg"><span class="name">${escapeHtml(reg.name)}</span>` +
        `<span class="value">${escapeHtml(reg.value)}</span></div>`
    ).join('');

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<style>
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 8px 12px; }
  h2 { font-size: 1em; font-weight: 600; margin: 0 0 8px; }
  .grid { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 12px; }
  .reg { display: flex; flex-direction: column; align-items: center; padding: 4px 8px;
         border: 1px solid var(--vscode-panel-border, #444); border-radius: 3px; min-width: 64px; }
  .reg .name { font-size: 0.75em; color: var(--vscode-descriptionForeground); }
  .reg .value { font-variant-numeric: tabular-nums; font-weight: 600; }
  table { border-collapse: collapse; margin-bottom: 12px; }
  th, td { border: 1px solid var(--vscode-panel-border, #444); padding: 2px 8px; text-align: right; }
  th { color: var(--vscode-descriptionForeground); font-weight: 600; }
  td.bad { color: var(--vscode-editorError-foreground, #f14c4c); }
  h3 { font-size: 0.95em; font-weight: 600; margin: 14px 0 6px; }
  footer { margin-top: 10px; color: var(--vscode-descriptionForeground); font-size: 0.9em; }
</style>
</head>
<body>
<h2>${escapeHtml(kind.title)} registers</h2>
<div class="grid">${cells}</div>
${extraHtml}
<footer>Not refreshed automatically - re-run <code>${escapeHtml(kind.command)}</code> in the debug
console for a current look; <code>-help</code> lists the commands.</footer>
</body>
</html>`;
}
