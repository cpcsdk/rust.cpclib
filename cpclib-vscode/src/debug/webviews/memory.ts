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
    const existing = memoryPanels.get(key);
    // An already-open panel gets the new frame patched into its *existing*
    // page instead of a fresh one - same fix, and same reason, as
    // `screen.ts`'s own account of the flicker/reload-loop a full
    // `webview.html` reload on every stop caused there: this view exists
    // specifically to "keep open and glance at while stepping" (see the
    // doc comment above), so reloading the whole page on every single step
    // defeats the point of leaving it open.
    if (existing) {
        void existing.webview.postMessage({
            type: 'cpclib.memoryFrame',
            title: memoryTitle(dump),
            byteCount: dump.bytes.length,
            tableHtml: memoryTableHtml(dump),
            config: dump.config,
            anchorArgument: anchorArgumentFromViewId(dump.viewId, dump.address),
            count: dump.bytes.length || 0x40,
        });
        // A stop's own silent refresh doesn't reveal it - pulling it in
        // front of whatever shares its column on every step would defeat
        // the same purpose. Only a person re-typing the command that
        // opened it (`-mv HL,follow` again) does.
        if (dump.requested) { existing.reveal(vscode.ViewColumn.Beside, true); }
        return;
    }

    const title = dump.label ?? `&${hex(dump.address, 4)}`;
    const panel = vscode.window.createWebviewPanel(
        'cpclib.memory',
        `CPC memory: ${title} — ${session.name}`,
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
        { enableScripts: true, retainContextWhenHidden: true },
    );
    panel.onDidDispose(() => {
        if (memoryPanels.get(key) === panel) { memoryPanels.delete(key); }
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

    panel.webview.html = memoryHtml(dump);
    // Always revealed here - this is the panel's very first paint (or a
    // person just typed the command that reused it, `-mv HL,follow` again
    // bringing it forward - but that's now the `existing` branch above).
    // `preserveFocus` keeps the keyboard, not the view.
    panel.reveal(vscode.ViewColumn.Beside, true);
}

/** The `<h2>` title text for `dump` - shared by the initial render and
 * every subsequent `cpclib.memoryFrame` patch. */
function memoryTitle(dump: MemoryDump): string {
    return dump.label
        ? `${escapeHtml(dump.label)} &mdash; &amp;${hex(dump.address, 4)}`
        : `&amp;${hex(dump.address, 4)}`;
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
    const title = memoryTitle(dump);
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
<h2><span id="titleText">${title}</span> &nbsp;<span class="addr"><span id="byteCount">${dump.bytes.length}</span> bytes</span></h2>
<div class="controls">
  <label>RAM configuration: <select id="config">${configOptions}</select></label>
</div>
<div id="tableContainer">${memoryTableHtml(dump)}</div>
<footer>Refreshed on every stop; highlighted bytes changed since the last one.
Point it elsewhere with <code>-mv</code> in the debug console; <code>-help</code> lists the commands.
Only AMSpiriT Lite can honour an explicit RAM configuration.</footer>
<script nonce="${nonce}">
  const vscode = acquireVsCodeApi();
  // Reassigned by the message listener below on every subsequent frame -
  // this page is built once (on the panel's first paint) and only ever
  // updates in place from here on, the same reason \`screen.ts\`'s own script
  // keeps its per-frame state in plain variables rather than the page
  // being rebuilt from scratch on every debug step.
  let anchorArgument = ${JSON.stringify(anchorArgument)};
  let count = ${dump.bytes.length || 0x40};
  const configSelect = document.getElementById('config');
  const titleText = document.getElementById('titleText');
  const byteCount = document.getElementById('byteCount');
  const tableContainer = document.getElementById('tableContainer');

  // Reissues -mv with the same anchor/count and the newly chosen config.
  function reissue() {
    vscode.postMessage({ config: configSelect.value, anchor: anchorArgument, count });
  }
  configSelect.addEventListener('change', reissue);

  window.addEventListener('message', event => {
    const msg = event.data;
    if (!msg || msg.type !== 'cpclib.memoryFrame') { return; }
    titleText.innerHTML = msg.title;
    byteCount.textContent = String(msg.byteCount);
    tableContainer.innerHTML = msg.tableHtml;
    anchorArgument = msg.anchorArgument;
    count = msg.count;
    // Left alone while the picker has focus, same reason \`screen.ts\`
    // leaves a focused field alone on an automatic refresh - a person
    // mid-choosing a config shouldn't have their own in-progress selection
    // overwritten by the next stop's answer.
    if (document.activeElement !== configSelect) {
      configSelect.value = msg.config != null ? String(msg.config) : '';
    }
  });
</script>
</body>
</html>`;
}
