import * as vscode from 'vscode';
import { hex, escapeHtml } from '../../shared/html';
import { MemoryDump } from '../types';
import { memoryPanels, memoryTableHtml, memoryPageStyle } from './memory';

// A grouped panel's members, keyed the same way as the panel itself - kept
// separately from the panel so a group's *other* members are still known
// when only one of them just got a fresh read.
const memoryGroupMembers = new Map<string, Map<string, MemoryDump>>();

// The order this was typed in - PC, SP, HL, DE, BC, IX, IY - not whatever
// order a `Map`/message arrival happens to produce. Shared by the initial
// server-side render and the client script's own insertion logic below, so
// a section arriving after the panel already exists still lands in the
// same place a full rebuild would have put it.
const MEMBER_ORDER = [
    'register:PC', 'register:SP', 'register:HL', 'register:DE',
    'register:BC', 'register:IX', 'register:IY',
];

function memberOrderIndex(id: string): number {
    const i = MEMBER_ORDER.indexOf(id);
    return i === -1 ? MEMBER_ORDER.length : i;
}

/** `id` (a `viewId` like `register:HL`, or a fallback label/address) turned
 * into something safe to use as an HTML element id. */
function sanitizeId(id: string): string {
    return id.replace(/[^a-zA-Z0-9_-]/g, '_');
}

/**
 * One panel for a whole `-mv all,follow` group: each register's read updates
 * its own section without disturbing the others', since the seven reads
 * this triggers do not all complete at once.
 */
export function showGroupedMemory(session: vscode.DebugSession, group: string, dump: MemoryDump): void {
    const key = `${session.id}:group:${group}`;
    const members = memoryGroupMembers.get(key) ?? new Map<string, MemoryDump>();
    const memberId = dump.viewId ?? dump.label ?? String(dump.address);
    members.set(memberId, dump);
    memoryGroupMembers.set(key, members);

    const existing = memoryPanels.get(key);
    // An already-open panel gets just the one member that changed patched
    // into its existing page, instead of a fresh `webview.html` reload of
    // every section - same fix, and same reason, as `memory.ts`'s
    // single-view panel (see its own doc comment). Since the seven reads a
    // `-mv all,follow` group triggers don't all complete at once, a full
    // reload on *each* arrival meant up to seven full-page rebuilds - each
    // re-serializing all seven sections - per single debug step.
    if (existing) {
        void existing.webview.postMessage({
            type: 'cpclib.groupedMemoryFrame',
            id: memberId,
            title: memorySectionTitle(dump),
            tableHtml: memoryTableHtml(dump),
        });
        // A stop's own silent refresh doesn't reveal it, same reasoning as
        // the single-view panel - only a person re-typing the command does.
        if (dump.requested) { existing.reveal(vscode.ViewColumn.Beside, true); }
        return;
    }

    const panel = vscode.window.createWebviewPanel(
        'cpclib.memory',
        `CPC memory: registers — ${session.name}`,
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
        { enableScripts: true, retainContextWhenHidden: true },
    );
    panel.onDidDispose(() => {
        if (memoryPanels.get(key) === panel) {
            memoryPanels.delete(key);
            memoryGroupMembers.delete(key);
        }
    });
    memoryPanels.set(key, panel);

    panel.webview.html = groupedMemoryHtml(members);
    panel.reveal(vscode.ViewColumn.Beside, true);
}

/** The `<h2>` title text for one member's section - shared by the initial
 * render and every subsequent `cpclib.groupedMemoryFrame` patch. */
function memorySectionTitle(dump: MemoryDump): string {
    return dump.label
        ? `${escapeHtml(dump.label)} &mdash; &amp;${hex(dump.address, 4)}`
        : `&amp;${hex(dump.address, 4)}`;
}

/**
 * `-mv all,follow`'s panel: every register's memory in one page, one section
 * apiece, instead of a tab per register - reusing `memoryTableHtml` per
 * member is the whole difference from the single-view page.
 */
function groupedMemoryHtml(members: Map<string, MemoryDump>): string {
    const ids = [...members.keys()].sort((a, b) => memberOrderIndex(a) - memberOrderIndex(b));

    const sections = ids.map(id => {
        const dump = members.get(id)!;
        const safeId = sanitizeId(id);
        return `<section id="section-${safeId}" data-id="${escapeHtml(id)}">` +
            `<h2 id="title-${safeId}">${memorySectionTitle(dump)}</h2>` +
            `<div id="table-${safeId}">${memoryTableHtml(dump)}</div></section>`;
    });

    const nonce = Math.random().toString(36).slice(2);
    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
<style>${memoryPageStyle}</style>
</head>
<body>
<div id="sections">${sections.join('')}</div>
<footer>Refreshed on every stop; highlighted bytes changed since the last one.
<code>-mv &lt;register&gt;,follow</code> opens one of these on its own instead;
<code>-help</code> lists the commands.</footer>
<script nonce="${nonce}">
  const order = ${JSON.stringify(MEMBER_ORDER)};
  const sectionsEl = document.getElementById('sections');

  function memberOrderIndex(id) {
    const i = order.indexOf(id);
    return i === -1 ? order.length : i;
  }

  function sanitizeId(id) {
    return id.replace(/[^a-zA-Z0-9_-]/g, '_');
  }

  window.addEventListener('message', event => {
    const msg = event.data;
    if (!msg || msg.type !== 'cpclib.groupedMemoryFrame') { return; }
    const safeId = sanitizeId(msg.id);
    const existingTitle = document.getElementById('title-' + safeId);
    const existingTable = document.getElementById('table-' + safeId);
    if (existingTitle && existingTable) {
      existingTitle.innerHTML = msg.title;
      existingTable.innerHTML = msg.tableHtml;
      return;
    }
    // A register this panel hasn't shown a section for yet (the seven
    // reads a \`-mv all,follow\` group triggers don't all complete at once) -
    // insert it at the same fixed position a full rebuild would have put
    // it in, rather than just appending at the end.
    const section = document.createElement('section');
    section.id = 'section-' + safeId;
    section.dataset.id = msg.id;
    section.innerHTML = '<h2 id="title-' + safeId + '"></h2><div id="table-' + safeId + '"></div>';
    section.querySelector('h2').innerHTML = msg.title;
    section.querySelector('div').innerHTML = msg.tableHtml;

    const newIndex = memberOrderIndex(msg.id);
    const next = [...sectionsEl.children].find(el => memberOrderIndex(el.dataset.id) > newIndex);
    if (next) {
      sectionsEl.insertBefore(section, next);
    } else {
      sectionsEl.appendChild(section);
    }
  });
</script>
</body>
</html>`;
}
