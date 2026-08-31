import * as vscode from 'vscode';
import { hex, escapeHtml } from '../../shared/html';
import { MemoryDump } from '../types';
import { memoryPanels, memoryTableHtml, memoryPageStyle } from './memory';

// A grouped panel's members, keyed the same way as the panel itself - kept
// separately from the panel so a group's *other* members are still known
// when only one of them just got a fresh read.
const memoryGroupMembers = new Map<string, Map<string, MemoryDump>>();

/**
 * One panel for a whole `-mv all,follow` group: each register's read updates
 * its own section without disturbing the others', since the seven reads
 * this triggers do not all complete at once.
 */
export function showGroupedMemory(session: vscode.DebugSession, group: string, dump: MemoryDump): void {
    const key = `${session.id}:group:${group}`;
    const members = memoryGroupMembers.get(key) ?? new Map<string, MemoryDump>();
    members.set(dump.viewId ?? dump.label ?? String(dump.address), dump);
    memoryGroupMembers.set(key, members);

    let panel = memoryPanels.get(key);
    const isNew = panel === undefined;
    if (!panel) {
        panel = vscode.window.createWebviewPanel(
            'cpclib.memory',
            `CPC memory: registers — ${session.name}`,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            { enableScripts: false, retainContextWhenHidden: true },
        );
        const owned = panel;
        panel.onDidDispose(() => {
            if (memoryPanels.get(key) === owned) {
                memoryPanels.delete(key);
                memoryGroupMembers.delete(key);
            }
        });
        memoryPanels.set(key, panel);
    }

    panel.webview.html = groupedMemoryHtml(members);
    if (isNew || dump.requested) { panel.reveal(vscode.ViewColumn.Beside, true); }
}

/**
 * `-mv all,follow`'s panel: every register's memory in one page, one section
 * apiece, instead of a tab per register - reusing `memoryTableHtml` per
 * member is the whole difference from the single-view page.
 */
function groupedMemoryHtml(members: Map<string, MemoryDump>): string {
    // The order this was typed in - PC, SP, HL, DE, BC, IX, IY - not
    // whatever order the Map happens to iterate in.
    const order = ['register:PC', 'register:SP', 'register:HL', 'register:DE',
        'register:BC', 'register:IX', 'register:IY'];
    const ids = [...members.keys()].sort((a, b) => {
        const ia = order.indexOf(a);
        const ib = order.indexOf(b);
        return (ia === -1 ? order.length : ia) - (ib === -1 ? order.length : ib);
    });

    const sections = ids.map(id => {
        const dump = members.get(id)!;
        const title = dump.label
            ? `${escapeHtml(dump.label)} &mdash; &amp;${hex(dump.address, 4)}`
            : `&amp;${hex(dump.address, 4)}`;
        return `<section><h2>${title}</h2>${memoryTableHtml(dump)}</section>`;
    });

    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
<style>${memoryPageStyle}</style>
</head>
<body>
${sections.join('')}
<footer>Refreshed on every stop; highlighted bytes changed since the last one.
<code>-mv &lt;register&gt;,follow</code> opens one of these on its own instead;
<code>-help</code> lists the commands.</footer>
</body>
</html>`;
}
