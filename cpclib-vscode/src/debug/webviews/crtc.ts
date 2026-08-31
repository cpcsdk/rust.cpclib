import * as vscode from 'vscode';
import { hex, escapeHtml } from '../../shared/html';
import { CrtcDump, CrtcWarning } from '../types';

const crtcPanels = new Map<string, vscode.WebviewPanel>();

/**
 * The CRTC registers, with any combination `validate_crtc` (Rust side) knows
 * to misbehave on real hardware highlighted in red - the plain "CRTC" scope
 * in the Variables view shows the same registers, but DAP has no way to mark
 * one row differently from another, so this is where the red actually goes.
 *
 * Opened with `-crtcview` in the debug console; unlike the memory view it is
 * not re-read on every stop (reading it means saving a whole machine, the
 * same cost `-chips` already avoids paying automatically) - re-run the
 * command for a fresh look.
 */
export function showCrtc(session: vscode.DebugSession, dump: CrtcDump | undefined): void {
    if (!dump || !Array.isArray(dump.registers)) { return; }

    let panel = crtcPanels.get(session.id);
    if (!panel) {
        panel = vscode.window.createWebviewPanel(
            'cpclib.crtc',
            `CPC CRTC — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: false, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (crtcPanels.get(session.id) === owned) { crtcPanels.delete(session.id); }
        });
        crtcPanels.set(session.id, panel);
    }

    panel.webview.html = crtcHtml(dump);
    // Unlike the memory view, -crtcview is never a silent per-stop refresh -
    // every call is a person asking, so it always comes forward, not only
    // when the panel is new.
    panel.reveal(vscode.ViewColumn.Beside, true);
}

function crtcHtml(dump: CrtcDump): string {
    const flagged = new Map<string, CrtcWarning[]>();
    for (const warning of dump.warnings) {
        for (const register of warning.registers) {
            const list = flagged.get(register) ?? [];
            list.push(warning);
            flagged.set(register, list);
        }
    }

    const cells = dump.registers.map(reg => {
        const warnings = flagged.get(reg.name) ?? [];
        const severity = warnings.some(w => w.severity === 'error')
            ? 'error'
            : warnings.length > 0 ? 'warning' : '';
        const title = warnings.map(w => w.message).join(' — ');
        return `<div class="reg${severity ? ` ${severity}` : ''}"${title ? ` title="${escapeHtml(title)}"` : ''}>` +
            `<span class="name">${escapeHtml(reg.name)}</span>` +
            `<span class="value">${hex(reg.value, 2)}</span></div>`;
    }).join('');

    const causes = dump.warnings.length
        ? `<ul class="causes">${dump.warnings
            .map(w => `<li class="${w.severity}"><b>${escapeHtml(w.registers.join(', '))}</b> — ${escapeHtml(w.message)}</li>`)
            .join('')}</ul>`
        : '<p class="ok">No known-bad register combination found.</p>';

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
         border: 1px solid var(--vscode-panel-border, #444); border-radius: 3px; min-width: 34px; }
  .reg .name { font-size: 0.75em; color: var(--vscode-descriptionForeground); }
  .reg .value { font-variant-numeric: tabular-nums; font-weight: 600; }
  .reg.error { border-color: var(--vscode-editorError-foreground, #f14c4c);
               background: var(--vscode-inputValidation-errorBackground, #5a1d1d); }
  .reg.warning { border-color: var(--vscode-editorWarning-foreground, #cca700);
                 background: var(--vscode-inputValidation-warningBackground, #5a4a1d); }
  .causes { margin: 0; padding-left: 1.2em; }
  .causes li.error { color: var(--vscode-editorError-foreground, #f14c4c); }
  .causes li.warning { color: var(--vscode-editorWarning-foreground, #cca700); }
  .ok { color: var(--vscode-descriptionForeground); }
  footer { margin-top: 10px; color: var(--vscode-descriptionForeground); font-size: 0.9em; }
</style>
</head>
<body>
<h2>CRTC registers</h2>
<div class="grid">${cells}</div>
${causes}
<footer>Not refreshed automatically - re-run <code>-crtcview</code> in the debug console for a
current look; <code>-help</code> lists the commands.</footer>
</body>
</html>`;
}
