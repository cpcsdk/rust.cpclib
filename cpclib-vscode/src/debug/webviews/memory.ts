import * as vscode from 'vscode';
import { hex, escapeHtml } from '../../shared/html';
import { MemoryDump } from '../types';
import { consoleCommand } from '../consoleCommand';
import { showGroupedMemory } from './groupedMemory';

/** Shared by `memory.ts` and `groupedMemory.ts` - a grouped panel and a
 * single-view panel key into the same map (distinguished by key prefix,
 * `session.id:viewId` vs `session.id:group:group`), so a group's *other*
 * members are still known when only one of them just got a fresh read. */
export const memoryPanels = new Map<string, vscode.WebviewPanel>();

/**
 * NOTE (preserved as-is, not fixed - out of scope, see the refactor plan's
 * "what NOT to touch"): this mirrors the original `debug.ts` terminate
 * cleanup exactly, including its pre-existing latent bug - `memoryPanels`'
 * real keys are always composite (`${sessionId}:${viewId}` from
 * `showMemory`, `${sessionId}:group:${group}` from `showGroupedMemory`),
 * never the bare session id, so this lookup/delete pair has always been a
 * no-op and memory-view panels have never actually been closed when a
 * session ends.
 */
export function disposeMemory(sessionId: string): void {
    memoryPanels.get(sessionId)?.dispose();
    memoryPanels.delete(sessionId);
}

/**
 * A memory dump, in a tab of its own.
 *
 * The *command* is typed in the debug console (`-mv 0xC000 0x20`) because that
 * is where your hands already are, but the dump belongs in a panel: it is
 * something you keep open and glance at while stepping, and console output
 * scrolls away the moment anything else is printed. One panel per view - the
 * adapter's own `viewId` (an address or a followed register) tells two open
 * views apart, so `-mv HL,follow` and `-mv DE,follow` open two panels side by
 * side rather than one replacing the other; repeating the same view's command
 * still refreshes its own panel rather than opening a duplicate.
 *
 * `-mv all,follow`'s views are the exception: they share a `group`, and all
 * land in one panel together instead - see `showGroupedMemory`.
 */
export function showMemory(session: vscode.DebugSession, dump: MemoryDump | undefined): void {
    if (!dump || !Array.isArray(dump.bytes)) { return; }
    if (dump.group) {
        showGroupedMemory(session, dump.group, dump);
        return;
    }

    const key = `${session.id}:${dump.viewId ?? 'default'}`;
    let panel = memoryPanels.get(key);
    const isNew = panel === undefined;
    if (!panel) {
        const title = dump.label ?? `&${hex(dump.address, 4)}`;
        panel = vscode.window.createWebviewPanel(
            'cpclib.memory',
            `CPC memory: ${title} — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: true, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (memoryPanels.get(key) === owned) { memoryPanels.delete(key); }
        });
        // The config picker's own change event - reissues -mv with the
        // same anchor/count and the newly chosen RAM-configuration
        // override, the same round trip the disassembly view's own picker
        // takes.
        panel.webview.onDidReceiveMessage(async (
            message: { config?: string; anchor?: string; count?: number },
        ) => {
            if (message?.config === undefined) { return; }
            const config = message.config === '' ? '_' : message.config;
            await consoleCommand(`-mv ${message.anchor ?? '_'} ${message.count ?? 0x40} ${config}`);
        });
        memoryPanels.set(key, panel);
    }

    panel.webview.html = memoryHtml(dump);
    // New, or a person just typed the command that reused it (`-mv HL,follow`
    // again brings the HL panel forward instead of leaving it wherever it
    // was). A stop's own silent refresh does neither - revealing it every
    // step would pull it in front of whatever shares its column.
    // `preserveFocus` keeps the keyboard, not the view.
    if (isNew || dump.requested) { panel.reveal(vscode.ViewColumn.Beside, true); }
}

export const memoryPageStyle = `
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 8px 12px; }
  h2 { font-size: 1em; font-weight: 600; margin: 0 0 8px; }
  table { border-collapse: collapse; font-variant-numeric: tabular-nums; margin-bottom: 4px; }
  td { padding: 1px 10px 1px 0; white-space: pre; }
  .addr { color: var(--vscode-descriptionForeground); }
  .ascii { color: var(--vscode-descriptionForeground); }
  .label { color: var(--vscode-symbolIcon-variableForeground, inherit); }
  .mark { text-decoration: underline; font-weight: 700; }
  /* What moved since the last stop - the reason to keep this open at all. */
  .changed { background: var(--vscode-diffEditor-insertedTextBackground, #2a4);
             color: var(--vscode-editor-foreground); border-radius: 2px; }
  section { margin-bottom: 18px; }
  footer { margin-top: 10px; color: var(--vscode-descriptionForeground); font-size: 0.9em; }
`;

/**
 * Sixteen bytes to a row, hex and ASCII, with the program's own labels marked
 * where they start - which is what turns a wall of digits into "this is
 * `animation_state`, and this is the four bytes after it". Just the table -
 * shared between a single view's own page and a grouped panel's several.
 */
export function memoryTableHtml(dump: MemoryDump): string {
    const marks = new Map((dump.marks ?? []).map(m => [m.offset, m.name]));
    const changed = new Set(dump.changed ?? []);
    const rows: string[] = [];

    for (let offset = 0; offset < dump.bytes.length; offset += 16) {
        const slice = dump.bytes.slice(offset, offset + 16);
        const cells = slice
            .map((byte, i) => {
                const name = marks.get(offset + i);
                const classes = [name ? 'mark' : '', changed.has(offset + i) ? 'changed' : '']
                    .filter(Boolean)
                    .join(' ');
                const cell = hex(byte, 2);
                return classes
                    ? `<span class="${classes}"${name ? ` title="${escapeHtml(name)}"` : ''}>${cell}</span>`
                    : cell;
            })
            .join(' ');
        // Padding keeps the ASCII column aligned on a short last row.
        const padding = '&nbsp;&nbsp;&nbsp;'.repeat(16 - slice.length);
        const ascii = slice
            .map(byte => (byte >= 0x20 && byte < 0x7f ? escapeHtml(String.fromCharCode(byte)) : '.'))
            .join('');
        const labelled = [...Array(slice.length).keys()]
            .map(i => marks.get(offset + i))
            .filter((name): name is string => !!name);

        rows.push(
            `<tr><td class="addr">&amp;${hex(dump.address + offset, 4)}</td>` +
            `<td class="hex">${cells}${padding}</td>` +
            `<td class="ascii">${ascii}</td>` +
            `<td class="label">${escapeHtml(labelled.join(', '))}</td></tr>`,
        );
    }

    return `<table>${rows.join('')}</table>`;
}

/** The viewId a `-mv`-opened panel carries (`"fixed:0000c000"`/
 * `"register:HL"`, see `MemoryAnchor::view_id`) parsed back into the
 * anchor argument `-mv` itself accepts - what the config picker's reissue
 * needs to keep pointing at the same place. */
function anchorArgumentFromViewId(viewId: string | undefined, address: number): string {
    if (viewId?.startsWith('register:')) {
        return `${viewId.slice('register:'.length)},follow`;
    }
    return `0x${hex(address, 4)}`;
}

function memoryHtml(dump: MemoryDump): string {
    const nonce = Math.random().toString(36).slice(2);
    const title = dump.label
        ? `${escapeHtml(dump.label)} &mdash; &amp;${hex(dump.address, 4)}`
        : `&amp;${hex(dump.address, 4)}`;
    // Same convention as the disassembly view's own picker.
    const configOptions = ['<option value="">Live (CPU)</option>']
        .concat(
            [0, 1, 2, 3, 4, 5, 6, 7].map(
                n => `<option value="${n}"${dump.config === n ? ' selected' : ''}>C${n}</option>`,
            ),
        )
        .join('');
    const anchorArgument = anchorArgumentFromViewId(dump.viewId, dump.address);

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
<style>${memoryPageStyle}
  .controls { margin-bottom: 8px; }
  .controls select, .controls input { font-family: inherit; }
</style>
</head>
<body>
<h2>${title} &nbsp;<span class="addr">${dump.bytes.length} bytes</span></h2>
<div class="controls">
  <label>RAM configuration: <select id="config">${configOptions}</select></label>
</div>
${memoryTableHtml(dump)}
<footer>Refreshed on every stop; highlighted bytes changed since the last one.
Point it elsewhere with <code>-mv</code> in the debug console; <code>-help</code> lists the commands.
Only AMSpiriT Lite can honour an explicit RAM configuration.</footer>
<script nonce="${nonce}">
  const vscode = acquireVsCodeApi();
  // Reissues -mv with the same anchor/count and the newly chosen config.
  function reissue() {
    vscode.postMessage({
      config: document.getElementById('config').value,
      anchor: ${JSON.stringify(anchorArgument)},
      count: ${dump.bytes.length || 0x40},
    });
  }
  document.getElementById('config').addEventListener('change', reissue);
</script>
</body>
</html>`;
}
