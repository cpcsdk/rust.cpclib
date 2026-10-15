import * as vscode from 'vscode';

/**
 * The breakpoint manager, after ACE's breakpoint window: three tabs -
 * Execution, Memory, Ports - each listing every breakpoint of its kind
 * (the manager's own, editable; the red dots, BREAKPOINT directives, data
 * breakpoints, watches and the stop on entry, shown with where they come
 * from), and a form to add, update or remove one: address or port with its
 * mask, access, value with its mask, condition (ACE's language), mode (stop
 * or watch), pass count and name.
 *
 * The adapter holds the breakpoints: `cpclib/breakpointManager` lists them,
 * `cpclib/managerSet` / `managerRemove` / `managerClear` change the manager's;
 * `cpclib/emulatorBreakpoints` (bemu) adds the hit counts.
 */

let panel: vscode.WebviewPanel | undefined;

/** Where the manager's breakpoints of a program are kept between sessions. */
function savedKey(program: string): string {
    return `cpclib.managerBreakpoints:${program}`;
}

/** The manager's breakpoints saved for `program`, as the launch passes them. */
export function savedManagerBreakpoints(context: vscode.ExtensionContext, program: string): unknown[] {
    return context.workspaceState.get<unknown[]>(savedKey(program)) ?? [];
}

/** Keep the session's manager breakpoints for the next launch of its program. */
async function save(context: vscode.ExtensionContext, session: vscode.DebugSession): Promise<void> {
    const program = session.configuration.program;
    if (typeof program !== 'string') { return; }
    const manager = await session.customRequest('cpclib/breakpointManager');
    await context.workspaceState.update(savedKey(program), manager?.saved ?? []);
}

async function snapshot(session: vscode.DebugSession): Promise<unknown> {
    const manager = await session.customRequest('cpclib/breakpointManager');
    let hits: Record<string, number> = {};
    try {
        const held = await session.customRequest('cpclib/emulatorBreakpoints');
        for (const brk of held?.advanced ?? []) {
            const owner = brk.owner === 'log point' ? 'source' : brk.owner;
            hits[`${owner}:${brk.label}`] = brk.hits ?? 0;
        }
    } catch {
        hits = {};
    }
    return { ...manager, hits, emulatorCounts: Object.keys(hits).length > 0 };
}

async function refresh(): Promise<void> {
    const session = vscode.debug.activeDebugSession;
    if (!panel) { return; }
    if (!session || session.type !== 'basm') {
        void panel.webview.postMessage({ type: 'state', state: null });
        return;
    }
    try {
        void panel.webview.postMessage({ type: 'state', state: await snapshot(session) });
    } catch (error) {
        void panel.webview.postMessage({ type: 'error', text: String(error) });
    }
}

/** Open the manager, or bring it forward. */
export function openBreakpointManager(context: vscode.ExtensionContext): void {
    if (panel) {
        panel.reveal(vscode.ViewColumn.Beside, false);
        void refresh();
        return;
    }
    panel = vscode.window.createWebviewPanel(
        'cpclib.breakpointManager',
        'CPC breakpoint manager',
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: false },
        { enableScripts: true, retainContextWhenHidden: true },
    );
    panel.onDidDispose(() => { panel = undefined; }, undefined, context.subscriptions);
    panel.webview.onDidReceiveMessage(async (message: { type: string; [key: string]: unknown }) => {
        const session = vscode.debug.activeDebugSession;
        if (message.type === 'refresh') { await refresh(); return; }
        if (message.type === 'open' && typeof message.path === 'string' && typeof message.line === 'number') {
            const line = Math.max(0, message.line - 1);
            await vscode.window.showTextDocument(vscode.Uri.file(message.path), {
                selection: new vscode.Range(line, 0, line, 0), viewColumn: vscode.ViewColumn.One,
            });
            return;
        }
        if (!session) { return; }
        const commands: Record<string, string> = {
            set: 'cpclib/managerSet', remove: 'cpclib/managerRemove', clear: 'cpclib/managerClear',
        };
        const command = commands[message.type];
        if (!command) { return; }
        try {
            const answer = await session.customRequest(command, message.arguments ?? {});
            await save(context, session);
            void panel?.webview.postMessage({ type: 'done', id: answer?.id });
        } catch (error) {
            void panel?.webview.postMessage({ type: 'error', text: String(error).replace(/^Error: /, '') });
        }
        await refresh();
    }, undefined, context.subscriptions);
    panel.webview.html = managerHtml();
}

/** Keep an open manager up to date. */
export function registerBreakpointManager(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.openBreakpointManager', () => openBreakpointManager(context)),
        vscode.debug.onDidChangeBreakpoints(() => void refresh()),
        vscode.debug.onDidChangeActiveDebugSession(() => void refresh()),
        vscode.debug.onDidTerminateDebugSession(() => void refresh()),
        vscode.debug.onDidReceiveDebugSessionCustomEvent(event => {
            if (['cpclib/emulatorReady', 'cpclib/stoppedAt', 'cpclib/stoppedWithoutSource'].includes(event.event)) {
                void refresh();
            }
        }),
    );
}

function managerHtml(): string {
    const nonce = Math.random().toString(36).slice(2);
    return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
<style>
body {
  font-family: var(--vscode-font-family); font-size: var(--vscode-font-size);
  color: var(--vscode-foreground); background: var(--vscode-editor-background); margin: 0; padding: 0.6em 1em;
}
.tabs { display: flex; gap: 2px; border-bottom: 1px solid var(--vscode-panel-border); margin-bottom: 0.6em; }
.tabs button {
  background: transparent; color: var(--vscode-foreground); border: none; padding: 0.4em 1.2em;
  border-bottom: 2px solid transparent; cursor: pointer; font-weight: 600;
}
.tabs button.active { border-bottom-color: var(--vscode-focusBorder); }
.tabs .count { font-weight: normal; opacity: 0.7; margin-left: 0.3em; }
.layout { display: flex; gap: 1.2em; align-items: flex-start; flex-wrap: wrap; }
.list { flex: 1 1 36em; min-width: 0; overflow-x: auto; }
table { border-collapse: collapse; width: 100%; font-family: var(--vscode-editor-font-family); font-size: 0.95em; }
th, td { text-align: left; padding: 0.25em 0.6em; white-space: nowrap; border-bottom: 1px solid var(--vscode-panel-border); }
th { position: sticky; top: 0; background: var(--vscode-editorGroupHeader-tabsBackground); font-weight: 600; }
tr.row { cursor: pointer; }
tr.row:hover { background: var(--vscode-list-hoverBackground); }
tr.row.selected { background: var(--vscode-list-activeSelectionBackground); color: var(--vscode-list-activeSelectionForeground); }
tr.inactive td { opacity: 0.55; text-decoration: line-through; }
.origin { font-family: var(--vscode-font-family); font-size: 0.85em; padding: 0 0.4em; border-radius: 3px;
  background: var(--vscode-badge-background); color: var(--vscode-badge-foreground); }
.partial { color: var(--vscode-editorWarning-foreground); }
.empty { padding: 1em 0.6em; opacity: 0.7; }
form { flex: 0 0 22em; border: 1px solid var(--vscode-panel-border); border-radius: 4px; padding: 0.8em; }
form h3 { margin: 0 0 0.6em; font-size: 1em; }
.field { display: grid; grid-template-columns: 6.5em 1fr; gap: 0.4em; align-items: center; margin-bottom: 0.45em; }
.pair { display: flex; gap: 0.4em; }
.pair input { min-width: 0; }
input, select {
  width: 100%; box-sizing: border-box; font-family: var(--vscode-editor-font-family);
  background: var(--vscode-input-background); color: var(--vscode-input-foreground);
  border: 1px solid var(--vscode-input-border, var(--vscode-panel-border)); padding: 0.2em 0.35em;
}
.buttons { display: flex; gap: 0.5em; margin-top: 0.8em; flex-wrap: wrap; }
button.action {
  background: var(--vscode-button-background); color: var(--vscode-button-foreground);
  border: none; padding: 0.35em 0.9em; cursor: pointer;
}
button.secondary { background: var(--vscode-button-secondaryBackground); color: var(--vscode-button-secondaryForeground); }
button:disabled { opacity: 0.5; cursor: default; }
.message { margin-top: 0.6em; min-height: 1.2em; font-size: 0.9em; }
.message.error { color: var(--vscode-errorForeground); }
.note { font-size: 0.85em; opacity: 0.8; margin-top: 0.6em; }
.hidden { display: none; }
</style>
</head>
<body>
<div class="tabs" role="tablist">
  <button data-tab="exec" class="active">Execution<span class="count" id="count-exec"></span></button>
  <button data-tab="mem">Memory<span class="count" id="count-mem"></span></button>
  <button data-tab="io">Ports<span class="count" id="count-io"></span></button>
  <button data-tab="raster">Raster<span class="count" id="count-raster"></span></button>
</div>
<div id="nosession" class="empty hidden">Start a CPC debug session to manage its breakpoints.</div>
<div class="layout" id="main">
  <div class="list">
    <table>
      <thead id="head"></thead>
      <tbody id="rows"></tbody>
    </table>
    <div class="buttons"><button class="action secondary" id="clear">Clear all (manager's)</button></div>
    <div class="note" id="emulatorNote"></div>
  </div>
  <form id="form" autocomplete="off">
    <h3 id="formTitle">Breakpoint</h3>
    <div class="field"><label for="preset">Predefined</label><select id="preset"></select></div>
    <div class="field raster-only"><label for="rasterLine">Line</label><input id="rasterLine" type="number" min="0" max="311" placeholder="every line"></div>
    <div class="field raster-only"><label for="nop">NOP</label><input id="nop" type="number" min="0" max="63" value="0"></div>
    <div class="field not-raster"><label for="address" id="addressLabel">Address</label>
      <div class="pair"><input id="address" list="symbols" placeholder="&C03C or a label"><input id="mask" class="io-mem" placeholder="mask &FFFF"></div></div>
    <div class="field io-mem"><label for="access">Access</label>
      <select id="access"><option value="rw">Read and write</option><option value="r">Read only</option><option value="w">Write only</option></select></div>
    <div class="field mem-only"><label for="size">Size</label><input id="size" type="number" min="1" max="65535" value="1"></div>
    <div class="field io-mem"><label for="value">Value</label>
      <div class="pair"><input id="value" placeholder="&00"><input id="valmask" placeholder="mask &00 (any)"></div></div>
    <div class="field not-raster"><label for="condition">Condition</label><input id="condition" placeholder="BC=7 AND FlagC"></div>
    <div class="field not-raster"><label for="mode">Mode</label>
      <select id="mode"><option value="stop">Stop</option><option value="watch">Watch (report, go on)</option></select></div>
    <div class="field not-raster"><label for="step">Pass</label><input id="step" type="number" min="0" value="0" title="Passes let through before it acts"></div>
    <div class="field"><label for="name">Name</label><input id="name"></div>
    <div class="buttons">
      <button type="submit" class="action" id="add">Add</button>
      <button type="button" class="action" id="update" disabled>Update</button>
      <button type="button" class="action secondary" id="remove" disabled>Remove</button>
      <button type="button" class="action secondary" id="new">New</button>
    </div>
    <div class="message" id="message"></div>
    <div class="note" id="rowNote"></div>
  </form>
</div>
<datalist id="symbols"></datalist>
<script nonce="${nonce}">
const vscode = acquireVsCodeApi();
let state = null;
let tab = 'exec';
let selected = null;
const $ = id => document.getElementById(id);
const ORIGINS = { manager: 'manager', source: 'red dot', directive: 'BREAKPOINT', data: 'data', watch: 'watch', entry: 'entry' };
const ACCESS = { r: 'Read only', w: 'Write only', rw: 'Read and write' };
const COLUMNS = {
  exec: [['symbol', 'Symbol'], ['address', 'Address'], ['condition', 'Condition'], ['pass', 'Pass'], ['mode', 'Mode'], ['name', 'Name'], ['origin', 'From']],
  mem: [['symbol', 'Symbol'], ['address', 'Address'], ['mask', '[mask]'], ['size', 'Size'], ['access', 'Access'], ['value', 'Value'], ['valmask', '[mask]'], ['condition', 'Condition'], ['pass', 'Pass'], ['mode', 'Mode'], ['name', 'Name'], ['origin', 'From']],
  io: [['address', 'Port'], ['mask', '[mask]'], ['access', 'Access'], ['value', 'Value'], ['valmask', '[mask]'], ['condition', 'Condition'], ['pass', 'Pass'], ['mode', 'Mode'], ['name', 'Name'], ['origin', 'From']],
  raster: [['rasterLine', 'Line'], ['nop', 'NOP'], ['name', 'Name'], ['origin', 'From']],
};

// Ready-made breakpoints, as ACE's "Points d'arrêt prédéfinis": the CPC's
// I/O ports decode a few address lines only, hence the masks.
const PRESETS = {
  exec: [
    ['Interrupt (IM 1 handler, &0038)', { address: '&0038' }],
    ['Firmware: TXT OUTPUT (&BB5A)', { address: '&BB5A' }],
    ['Firmware: KM WAIT KEY (&BB18)', { address: '&BB18' }],
    ['Firmware: SCR SET MODE (&BC0E)', { address: '&BC0E' }],
    ['Firmware: MC WAIT FLYBACK (&BD19)', { address: '&BD19' }],
    ['Firmware: CAS IN OPEN (&BC77)', { address: '&BC77' }],
    ['AMSDOS: BIOS READ SECTOR (&C03C, ROM 7)', { address: '&C03C', condition: 'UpperROMOn AND UpperROMNum = 7', name: 'BIOS READ SECTOR' }],
  ],
  mem: [
    ['Screen memory written (&C000-&FFFF)', { address: '&C000', size: 16384, access: 'w' }],
    ['Firmware jumpblock written (&BB00-&BDFF)', { address: '&BB00', size: 768, access: 'w' }],
    ['Restarts written (&0000-&003F)', { address: '&0000', size: 64, access: 'w' }],
  ],
  io: [
    ['Gate Array: any write', { address: '&4000', mask: '&C000', access: 'w' }],
    ['Gate Array: pen select', { address: '&4000', mask: '&C000', access: 'w', value: '&00', valmask: '&C0' }],
    ['Gate Array: ink (colour)', { address: '&4000', mask: '&C000', access: 'w', value: '&40', valmask: '&C0' }],
    ['Gate Array: mode and ROMs (RMR)', { address: '&4000', mask: '&C000', access: 'w', value: '&80', valmask: '&C0' }],
    ['RAM banking (MMR)', { address: '&4000', mask: '&C000', access: 'w', value: '&C0', valmask: '&C0' }],
    ['CRTC: register select', { address: '&0000', mask: '&4300', access: 'w' }],
    ['CRTC: register write', { address: '&0100', mask: '&4300', access: 'w' }],
    ['Upper ROM select', { address: '&0000', mask: '&2000', access: 'w' }],
    ['PPI port A (PSG data)', { address: '&0000', mask: '&0B00' }],
    ['PPI port C (PSG control, keyboard line)', { address: '&0200', mask: '&0B00', access: 'w' }],
    ['PPI control', { address: '&0300', mask: '&0B00', access: 'w' }],
    ['FDC: data register', { address: '&0101', mask: '&0581' }],
    ['FDC: status read', { address: '&0100', mask: '&0581', access: 'r' }],
    ['FDC: motor', { address: '&0000', mask: '&0581', access: 'w' }],
  ],
  raster: [
    ['Frame start (line 0, NOP 0)', { rasterLine: 0, nop: 0 }],
    ['Middle of the frame (line 156)', { rasterLine: 156, nop: 0 }],
    ['Every line, NOP 0', { rasterLine: '', nop: 0 }],
    ['Every line, NOP 40 (right border)', { rasterLine: '', nop: 40 }],
  ],
};

function text(value) { return value === undefined || value === null ? '' : String(value); }

function cell(row, key) {
  const td = document.createElement('td');
  if (key === 'origin') {
    const badge = document.createElement('span');
    badge.className = 'origin';
    badge.textContent = ORIGINS[row.origin] || row.origin;
    td.appendChild(badge);
    if (row.status === 'partial') { td.appendChild(document.createTextNode(' ⚠')); td.className = 'partial'; }
    return td;
  }
  if (key === 'pass') {
    const hits = state.hits[row.key];
    td.textContent = (hits === undefined ? '-' : hits) + '/' + text(row.step);
    td.title = 'hits / passes let through';
    return td;
  }
  if (key === 'access') { td.textContent = ACCESS[row.access] || text(row.access); return td; }
  if (key === 'rasterLine' && (row.rasterLine === null || row.rasterLine === undefined)) { td.textContent = 'every line'; return td; }
  if (key === 'mode') { td.textContent = { stop: 'Stop', watch: 'Watch', log: 'Log' }[row.mode] || text(row.mode); return td; }
  td.textContent = text(row[key]);
  return td;
}

function render() {
  const has = !!state;
  $('nosession').classList.toggle('hidden', has);
  $('main').classList.toggle('hidden', !has);
  for (const id of ['exec', 'mem', 'io', 'raster']) {
    $('count-' + id).textContent = has ? '(' + state.tabs[id].length + ')' : '';
  }
  document.querySelectorAll('.tabs button').forEach(b => b.classList.toggle('active', b.dataset.tab === tab));
  if (!has) { return; }
  const head = $('head');
  head.innerHTML = '';
  const tr = document.createElement('tr');
  for (const [, title] of COLUMNS[tab]) { const th = document.createElement('th'); th.textContent = title; tr.appendChild(th); }
  head.appendChild(tr);
  const body = $('rows');
  body.innerHTML = '';
  const rows = state.tabs[tab];
  if (rows.length === 0) {
    const empty = document.createElement('tr');
    const td = document.createElement('td');
    td.colSpan = COLUMNS[tab].length;
    td.className = 'empty';
    td.textContent = 'No breakpoint here yet: fill the form and press Add.';
    empty.appendChild(td);
    body.appendChild(empty);
  }
  for (const row of rows) {
    const line = document.createElement('tr');
    line.className = 'row' + (row.status === 'inactive' ? ' inactive' : '') + (selected === row.key ? ' selected' : '');
    line.title = row.note || (row.editable ? 'Click to edit' : 'Made elsewhere: double-click to open where');
    for (const [key] of COLUMNS[tab]) { line.appendChild(cell(row, key)); }
    line.addEventListener('click', () => select(row));
    line.addEventListener('dblclick', () => { if (row.path && row.line) { vscode.postMessage({ type: 'open', path: row.path, line: row.line }); } });
    body.appendChild(line);
  }
  // The form's fields for this tab.
  document.querySelectorAll('.io-mem').forEach(e => e.classList.toggle('hidden', tab === 'exec' || tab === 'raster'));
  document.querySelectorAll('.raster-only').forEach(e => e.classList.toggle('hidden', tab !== 'raster'));
  document.querySelectorAll('.not-raster').forEach(e => e.classList.toggle('hidden', tab === 'raster'));
  const preset = $('preset');
  if (preset.dataset.tab !== tab) {
    preset.dataset.tab = tab;
    preset.innerHTML = '';
    const none = document.createElement('option'); none.value = ''; none.textContent = 'Choose one to fill the form…'; preset.appendChild(none);
    PRESETS[tab].forEach(([title], index) => { const option = document.createElement('option'); option.value = String(index); option.textContent = title; preset.appendChild(option); });
  }
  document.querySelectorAll('.mem-only').forEach(e => e.classList.toggle('hidden', tab !== 'mem'));
  $('addressLabel').textContent = tab === 'io' ? 'Port' : 'Address';
  $('emulatorNote').textContent = state.whole
    ? (state.emulatorCounts ? 'Pass shows hits / passes let through, as the emulator counts them.' : '')
    : 'This emulator only takes plain execution breakpoints from the manager: conditions, passes, watch mode, memory, port and raster breakpoints need bemu.';
  if (tab === 'raster') { $('emulatorNote').textContent += ' Raster: stops when the beam reaches the line and NOP (a blank line: every line).'; }
  const list = $('symbols');
  if (list.childElementCount !== state.symbols.length) {
    list.innerHTML = '';
    for (const name of state.symbols) { const option = document.createElement('option'); option.value = name; list.appendChild(option); }
  }
}

function select(row) {
  selected = row.key;
  for (const key of ['address', 'mask', 'size', 'access', 'value', 'valmask', 'condition', 'step', 'name', 'rasterLine', 'nop']) {
    $(key).value = text(row[key]);
  }
  $('mode').value = row.mode === 'watch' ? 'watch' : 'stop';
  $('update').disabled = !row.editable;
  $('remove').disabled = !row.editable;
  $('formTitle').textContent = row.editable ? 'Breakpoint ' + (row.name || row.address) : 'Breakpoint (' + (ORIGINS[row.origin] || row.origin) + ', read-only)';
  $('rowNote').textContent = row.note || (row.editable ? '' : 'Made elsewhere - change it where it comes from. Add makes a manager copy.');
  render();
}

function clearForm() {
  selected = null;
  for (const key of ['address', 'value', 'condition', 'name']) { $(key).value = ''; }
  $('mask').value = ''; $('valmask').value = ''; $('size').value = '1'; $('step').value = '0';
  $('rasterLine').value = ''; $('nop').value = '0'; $('preset').value = '';
  $('access').value = 'rw'; $('mode').value = 'stop';
  $('update').disabled = true; $('remove').disabled = true;
  $('formTitle').textContent = 'Breakpoint'; $('rowNote').textContent = '';
  render();
}

function form() {
  if (tab === 'raster') {
    return { kind: 'raster', line: $('rasterLine').value, nop: $('nop').value, name: $('name').value };
  }
  const breakpoint = { kind: tab };
  for (const key of ['address', 'mask', 'size', 'access', 'value', 'valmask', 'condition', 'mode', 'step', 'name']) {
    breakpoint[key] = $(key).value;
  }
  if (tab === 'exec') { delete breakpoint.mask; delete breakpoint.access; delete breakpoint.value; delete breakpoint.valmask; delete breakpoint.size; }
  return breakpoint;
}

function say(textValue, error) { const m = $('message'); m.textContent = textValue; m.className = 'message' + (error ? ' error' : ''); }

document.querySelectorAll('.tabs button').forEach(b => b.addEventListener('click', () => { tab = b.dataset.tab; clearForm(); }));
$('form').addEventListener('submit', event => { event.preventDefault(); say(''); vscode.postMessage({ type: 'set', arguments: { breakpoint: form() } }); });
$('update').addEventListener('click', () => {
  const row = state.tabs[tab].find(r => r.key === selected);
  if (!row) { return; }
  say('');
  vscode.postMessage({ type: 'set', arguments: { breakpoint: { ...form(), id: row.id } } });
});
$('remove').addEventListener('click', () => {
  const row = state.tabs[tab].find(r => r.key === selected);
  if (row) { vscode.postMessage({ type: 'remove', arguments: { id: row.id } }); clearForm(); }
});
$('new').addEventListener('click', clearForm);
$('preset').addEventListener('change', () => {
  const chosen = PRESETS[tab][Number($('preset').value)];
  if (!$('preset').value || !chosen) { return; }
  const [title, fields] = chosen;
  const keep = $('preset').value;
  clearForm();
  $('preset').value = keep;
  for (const [key, value] of Object.entries(fields)) { $(key).value = String(value); }
  if (!fields.name) { $('name').value = title.replace(/ \\(.*\\)$/, ''); }
});
$('clear').addEventListener('click', () => vscode.postMessage({ type: 'clear', arguments: { kind: tab } }));
window.addEventListener('message', event => {
  const message = event.data;
  if (message.type === 'state') { state = message.state; render(); }
  if (message.type === 'error') { say(message.text, true); }
  if (message.type === 'done') { say('Done.'); if (message.id) { selected = 'manager:' + message.id; } }
});
render();
vscode.postMessage({ type: 'refresh' });
</script>
</body>
</html>`;
}
