import * as vscode from 'vscode';
import { hex } from '../../shared/html';
import { ScreenDump } from '../types';
import { consoleCommand } from '../consoleCommand';

const screenPanels = new Map<string, vscode.WebviewPanel>();

export function disposeScreen(sessionId: string): void {
    screenPanels.get(sessionId)?.dispose();
    screenPanels.delete(sessionId);
}

/**
 * CPC video memory rendered as an actual image (WinAPE-style) - `-sv` in the
 * debug console, or the panel's own controls re-issuing it. Server-side PNG,
 * not a client-side pixel decoder: the mode-aware bit layout stays in one
 * place (`cpclib-image`'s own, already-tested `ColorMatrix`), not duplicated
 * in TypeScript - see the WinAPE-style screen viewer plan's own "reuse,
 * don't reimplement" reasoning. The one piece of address arithmetic that
 * *is* duplicated here, in the page's own script, is the mouse-over
 * readout's coordinate math (screen X/Y -> byte address) - the plan's own
 * explicit exception, since it is simple and low-risk next to the full
 * pixel decode. One panel per session, like `-bv` - there is only ever one
 * screen worth looking at.
 */
export function showScreen(session: vscode.DebugSession, dump: ScreenDump | undefined): void {
    if (!dump || typeof dump.png !== 'string') { return; }

    const key = session.id;
    const existing = screenPanels.get(key);
    // An already-open panel gets the new frame pushed into its *existing*
    // page instead of a fresh one - reported live: replacing the whole
    // `webview.html` on every update reloads the page from scratch, which
    // re-runs its own script, which (now that a stop can trigger this on
    // its own, via `refresh_screen_view`) posts another render request back
    // - a full HTML reload for every single step is a visible flicker on
    // its own, and re-entering the script on every one of them turned that
    // into a self-sustaining reload loop that never let a click or a typed
    // character land before the next reload arrived.
    if (existing) {
        void existing.webview.postMessage({ type: 'cpclib.screenFrame', dump });
        return;
    }

    const panel = vscode.window.createWebviewPanel(
        'cpclib.screen',
        `CPC screen — ${session.name}`,
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
        { enableScripts: true, retainContextWhenHidden: true },
    );
    panel.onDidDispose(() => {
        if (screenPanels.get(key) === panel) { screenPanels.delete(key); }
    });
    // The control row posts back exactly the six `-sv` arguments to
    // re-render with - same round trip the console command itself takes,
    // just triggered from the panel instead of typed. `_` is a placeholder
    // for "no override, use the live default" - `-sv`'s own argument
    // parser already treats anything that fails to parse as a number (or,
    // for `palette`, as a comma-separated ink-index list) that way, and
    // unlike an empty string, `_` survives the adapter's plain
    // `split_whitespace()` tokenising, so a *middle* argument (e.g.
    // address, with width set after it) can be left at its default
    // without shifting every argument after it out of position.
    // `totalHeight` - the one field the page computes from its own
    // available space rather than anything the user typed - is always
    // sent, since the adapter has no way to know that itself.
    panel.webview.onDidReceiveMessage((message: {
        address?: string; width?: string; gap?: string; mode?: string; totalHeight?: number;
        palette?: string; encoding?: string; config?: string;
    }) => {
        if (!message) { return; }
        const raw = [
            message.address, message.width, String(message.totalHeight ?? ''),
            message.mode, message.gap, message.palette, message.encoding, message.config,
        ];
        const parts = raw.map(v => {
            const trimmed = (v ?? '').trim();
            return trimmed === '' ? '_' : trimmed;
        });
        while (parts.length > 0 && parts[parts.length - 1] === '_') { parts.pop(); }
        void consoleCommand(`-sv ${parts.join(' ')}`.trimEnd());
    });
    screenPanels.set(key, panel);

    panel.webview.html = screenHtml(dump);
    panel.reveal(vscode.ViewColumn.Beside, true);
}

const SCREEN_MODE_NAMES = [
    '0 (16 colours)', '1 (4 colours)', '2 (2 colours)', '3 (4 colours)',
];

/** How many of the 16 palette pens each mode actually uses. */
const PENS_PER_MODE = [16, 4, 2, 4];

function screenHtml(dump: ScreenDump): string {
    const nonce = Math.random().toString(36).slice(2);
    const modeName = SCREEN_MODE_NAMES[dump.mode] ?? String(dump.mode);
    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
<style>
html, body { height: 100%; }
body {
  font-family: var(--vscode-editor-font-family, monospace); color: var(--vscode-editor-foreground);
  background: var(--vscode-editor-background); box-sizing: border-box; margin: 0; padding: 0.5em 1em;
  display: flex; flex-direction: column; min-height: 0;
}
h2 { flex: 0 0 auto; margin: 0.2em 0; }
.addr { color: var(--vscode-descriptionForeground); }
footer { flex: 0 0 auto; color: var(--vscode-descriptionForeground); margin-top: 0.4em; font-size: 0.9em; }
code { background: var(--vscode-textCodeBlock-background); padding: 0 0.3em; }
form { flex: 0 0 auto; display: flex; gap: 0.8em; align-items: baseline; flex-wrap: wrap; margin-bottom: 0.3em; }
label { display: flex; gap: 0.35em; align-items: baseline; font-size: 0.9em; }
input, select {
  font-family: var(--vscode-editor-font-family, monospace);
  background: var(--vscode-input-background); color: var(--vscode-input-foreground);
  border: 1px solid var(--vscode-input-border, transparent); border-radius: 2px; padding: 2px 4px;
}
input[type="number"] { width: 4.5em; }
input[type="text"] { width: 5em; text-align: right; }
.stepper { display: inline-flex; align-items: stretch; }
.stepper input { border-radius: 2px 0 0 2px; }
.stepper .arrows { display: flex; flex-direction: column; }
.stepper .arrows button {
  flex: 1; padding: 0 4px; line-height: 1; font-size: 0.6em; border-radius: 0;
}
.stepper .arrows button:first-child { border-radius: 0 2px 0 0; }
.stepper .arrows button:last-child { border-radius: 0 0 2px 0; }
button {
  background: var(--vscode-button-background); color: var(--vscode-button-foreground);
  border: none; border-radius: 2px; padding: 3px 10px; cursor: pointer; font-size: 0.9em;
}
button:hover { background: var(--vscode-button-hoverBackground); }
#palette { flex: 0 0 auto; display: flex; gap: 3px; margin-bottom: 0.4em; flex-wrap: wrap; position: relative; }
#palette .swatch {
  width: 1.1em; height: 1.1em; border: 1px solid var(--vscode-panel-border, #444);
  border-radius: 2px; padding: 0; cursor: pointer;
}
#palette .swatch.overridden { outline: 2px solid var(--vscode-focusBorder, #007acc); outline-offset: 1px; }
#picker {
  position: absolute; top: 1.5em; left: 0; z-index: 10; display: none; flex-wrap: wrap; width: 12em;
  gap: 3px; padding: 4px; background: var(--vscode-editorWidget-background, #252526);
  border: 1px solid var(--vscode-panel-border, #444); border-radius: 3px;
}
#picker.open { display: flex; }
#picker .swatch { width: 1.1em; height: 1.1em; border: 1px solid var(--vscode-panel-border, #444); border-radius: 2px; padding: 0; cursor: pointer; }
#picker .reset { width: 100%; font-size: 0.8em; padding: 2px 4px; }
#imgWrap { flex: 1 1 auto; min-height: 0; overflow: auto; }
canvas#screen { image-rendering: pixelated; border: 1px solid var(--vscode-panel-border, #444); cursor: crosshair; display: block; }
#readout { flex: 0 0 auto; min-height: 1.2em; font-size: 0.9em; }
</style>
</head>
<body>
<h2>Screen &nbsp;<span class="addr">mode ${modeName}</span></h2>
<form id="controls">
  <label>Address (hex) <span class="stepper">
    <input type="text" id="address" value="${hex(dump.address, 4)}" maxlength="4">
    <span class="arrows">
      <button type="button" id="addressUp" title="+1">▲</button>
      <button type="button" id="addressDown" title="-1">▼</button>
    </span>
  </span></label>
  <label>Width (bytes) <input type="number" id="width" min="1" max="255" value="${dump.width}"></label>
  <label>Char row height (lines) <input type="number" id="charRowHeight" min="0" max="2048" value="${dump.charRowHeight}"></label>
  <label>Mode <select id="mode">
    ${[0, 1, 2, 3].map(m => `<option value="${m}"${m === dump.mode ? ' selected' : ''}>${m}</option>`).join('')}
  </select></label>
  <label>Encoding <select id="encoding">
    <option value="0"${dump.encoding === 0 ? ' selected' : ''}>Screen</option>
    <option value="1"${dump.encoding === 1 ? ' selected' : ''}>CPC</option>
  </select></label>
  <label>RAM configuration <select id="config">
    <option value="">Live (CPU)</option>
    ${[0, 1, 2, 3, 4, 5, 6, 7]
        .map(n => `<option value="${n}"${dump.config === n ? ' selected' : ''}>C${n}</option>`)
        .join('')}
  </select></label>
  <button type="button" id="auto">Auto-detect</button>
</form>
<div id="palette" title="This window's own palette - starts from the live Gate Array, click a swatch to change it. Never written back to the emulator: the CPC itself never hears about it."></div>
<div id="imgWrap"><canvas id="screen"></canvas></div>
<div id="readout">&nbsp;</div>
<footer>Move the mouse over the image for the address/value under the cursor. The image fills the panel
automatically, tiling into more columns when there is room for them; everything else applies the moment
it changes. Point it elsewhere with
<code>-sv &lt;address&gt; &lt;width&gt; &lt;height&gt; &lt;mode&gt; &lt;gap&gt; &lt;palette&gt; &lt;encoding&gt; &lt;config&gt;</code>
in the debug console - <code>-help</code> lists every command. Only AMSpiriT
Lite can honour an explicit RAM configuration.</footer>
<script nonce="${nonce}">
  const vscode = acquireVsCodeApi();
  // \`dump\` and \`bytes\` are reassigned by \`applyDump\` on every new frame -
  // including the automatic ones a stop can now trigger on its own
  // (\`refresh_screen_view\`) - so this page is built once and only ever
  // updates in place from here on. Reported live: replacing the whole page
  // (\`panel.webview.html = ...\`) on every frame reloaded it from scratch,
  // which re-ran this very script, which posted another render request of
  // its own - a full-page flicker on every single step, escalating into a
  // reload loop that never let a click or a keystroke land before the next
  // reload arrived.
  let dump = null;
  let bytes = new Uint8Array(0);
  // This window's own palette overrides - pen index -> CPC ink number
  // (0-26), or null for "no override, follow the live Gate Array". Lives
  // only here: never sent to the emulator, and since the page itself no
  // longer reloads on a refresh, a plain JS variable is enough to survive
  // every automatic re-render on its own.
  const paletteOverride = new Array(16).fill(null);
  let openPickerPen = null;

  // WinAPE's own multi-column tiling, generalised to both axes: the panel
  // lays out a full grid of \`rowHeightValue\`-real-lines-tall, \`width\`-
  // bytes-wide tiles, as many as fit both vertically and horizontally, all
  // separated by the *same* padding - reported live: the very first cut of
  // this drew a black pixel row between vertically-stacked tiles but only
  // empty padding between columns, which read as two different features
  // rather than one consistent grid. The server renders one tall,
  // uninterrupted image with no padding or "grid" concept of its own at
  // all (\`columns * rows * rowHeightValue\` real lines, column-major:
  // column 0's own \`rows\` tiles first, in address order, then column 1's);
  // slicing that into the visible grid is done here, on a \`<canvas>\`,
  // entirely client-side - the pixel *decode* still lives in exactly one
  // place (\`cpclib-image\`'s own \`ColorMatrix\`), this is pure re-layout of
  // pixels the server already produced.
  const PADDING = 2;
  let currentColumns = 1;
  let currentRows = 1;
  let currentRowHeightValue = 8;

  const addressField = document.getElementById('address');
  const widthField = document.getElementById('width');
  const charRowHeightField = document.getElementById('charRowHeight');
  const modeField = document.getElementById('mode');
  const encodingField = document.getElementById('encoding');
  const configField = document.getElementById('config');
  const imgWrap = document.getElementById('imgWrap');
  const canvas = document.getElementById('screen');
  const ctx = canvas.getContext('2d');
  const readout = document.getElementById('readout');
  const paletteDiv = document.getElementById('palette');

  // How many \`unitSize\`-plus-padding units fit \`available\` space - the one
  // piece of arithmetic both grid axes share.
  function computeUnitsFitting(available, unitSize) {
    return Math.max(1, Math.floor((Math.max(available, unitSize) + PADDING) / (unitSize + PADDING)));
  }

  // One tile's own real-line count - typed value if there is one, else the
  // live CRTC's own \`charRowHeight\` - and how many of those fit one
  // column's available height.
  function currentRowHeightAndRows() {
    const typed = parseInt(charRowHeightField.value, 10);
    const rowHeightValue = Number.isFinite(typed) && typed > 0 ? typed : (dump ? dump.charRowHeight : 8);
    const rows = computeUnitsFitting(imgWrap.clientHeight, rowHeightValue * 2);
    return { rowHeightValue, rows };
  }

  function paletteArgument(useDefaults) {
    if (useDefaults) { return ''; }
    if (paletteOverride.every(v => v === null)) { return ''; }
    return paletteOverride.map(v => (v === null ? '' : String(v))).join(',');
  }

  // No "Refresh" button: every control applies the moment it changes -
  // 'change' rather than 'input' so typing a multi-digit number does not
  // re-render on every keystroke, while a dropdown, the spinner arrows or
  // the address stepper still apply at once. \`useDefaults\` is what
  // "Auto-detect" asks for: every typed field, and every palette override,
  // is dropped - only the freshly computed total height still goes out
  // (the adapter has no other way to learn the panel's own available
  // space, or how many columns fit it).
  function requestRender(useDefaults) {
    if (useDefaults) { paletteOverride.fill(null); }
    // The address field shows and edits bare hex ("C000"), but the
    // adapter's own \`parse_number\`/\`parse_address\` only read a value as
    // hex when it carries a prefix (\`0x\`/\`&\`/...) - anything else parses
    // as decimal, which "C000" is not, so it silently failed to parse at
    // all and the override was always dropped. Reported live: editing the
    // address field, or the stepper, had no visible effect.
    const addressValue = addressField.value.trim();
    const width = parseInt(widthField.value, 10) || (dump ? dump.width : 80);
    const { rowHeightValue, rows } = currentRowHeightAndRows();
    const columns = computeUnitsFitting(imgWrap.clientWidth, width * 8);
    vscode.postMessage({
      address: useDefaults || addressValue === '' ? '' : ('0x' + addressValue),
      width: useDefaults ? '' : widthField.value,
      gap: useDefaults ? '' : charRowHeightField.value,
      mode: useDefaults ? '' : modeField.value,
      encoding: useDefaults ? '' : encodingField.value,
      config: useDefaults ? '' : configField.value,
      totalHeight: columns * rows * rowHeightValue,
      palette: paletteArgument(useDefaults),
    });
  }

  document.getElementById('controls').addEventListener('change', () => requestRender(false));
  document.getElementById('controls').addEventListener('submit', event => event.preventDefault());
  document.getElementById('auto').addEventListener('click', () => requestRender(true));

  function stepAddress(delta) {
    const current = parseInt(addressField.value, 16);
    const next = ((Number.isFinite(current) ? current : (dump ? dump.address : 0)) + delta) & 0xFFFF;
    addressField.value = next.toString(16).toUpperCase().padStart(4, '0');
    requestRender(false);
  }

  // Reported live: one click already re-rendered, but holding the button
  // down did not keep going the way a typical spinner control would - a
  // single \`click\` handler fires exactly once per press, however long it
  // is held. \`mousedown\` starts a real repeat instead: one immediate step,
  // then a short pause before a steady auto-repeat, stopped by \`mouseup\`/
  // \`mouseleave\` alike (releasing outside the button never gets stuck).
  function holdToRepeat(button, delta) {
    let repeatTimer = null;
    const stop = () => {
      clearTimeout(repeatTimer);
      repeatTimer = null;
    };
    button.addEventListener('mousedown', () => {
      stop();
      stepAddress(delta);
      repeatTimer = setTimeout(function repeat() {
        stepAddress(delta);
        repeatTimer = setTimeout(repeat, 80);
      }, 400);
    });
    button.addEventListener('mouseup', stop);
    button.addEventListener('mouseleave', stop);
  }
  holdToRepeat(document.getElementById('addressUp'), 1);
  holdToRepeat(document.getElementById('addressDown'), -1);

  function closePicker() {
    const existing = document.getElementById('picker');
    if (existing) { existing.remove(); }
    openPickerPen = null;
  }

  function openPicker(pen, anchor) {
    if (openPickerPen === pen) { closePicker(); return; }
    closePicker();
    openPickerPen = pen;
    const picker = document.createElement('div');
    picker.id = 'picker';
    picker.className = 'open';
    const reset = document.createElement('button');
    reset.type = 'button';
    reset.className = 'reset';
    reset.textContent = 'Live Gate Array colour';
    reset.addEventListener('click', () => {
      paletteOverride[pen] = null;
      closePicker();
      requestRender(false);
    });
    picker.appendChild(reset);
    dump.hardwarePalette.forEach((colour, ink) => {
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.className = 'swatch';
      chip.style.background = colour;
      chip.title = 'Ink ' + ink + ': ' + colour;
      chip.addEventListener('click', () => {
        paletteOverride[pen] = ink;
        closePicker();
        requestRender(false);
      });
      picker.appendChild(chip);
    });
    anchor.parentElement.appendChild(picker);
  }

  document.addEventListener('click', event => {
    if (openPickerPen !== null && !event.target.closest('#picker') && !event.target.closest('.swatch')) {
      closePicker();
    }
  });

  function renderPaletteSwatches() {
    closePicker();
    paletteDiv.textContent = '';
    const pensShown = ${JSON.stringify(PENS_PER_MODE)}[dump.mode] ?? 16;
    dump.palette.slice(0, pensShown).forEach((colour, pen) => {
      const swatch = document.createElement('button');
      swatch.type = 'button';
      swatch.className = 'swatch' + (paletteOverride[pen] !== null ? ' overridden' : '');
      swatch.style.background = colour;
      swatch.title = 'Pen ' + pen + ': ' + colour +
        (paletteOverride[pen] !== null ? ' (window override)' : ' (live)');
      swatch.addEventListener('click', () => openPicker(pen, swatch));
      paletteDiv.appendChild(swatch);
    });
  }

  // Applies one rendered frame - the initial one, and every automatic
  // refresh after it - without touching anything the page itself owns
  // (scroll position, focus, the palette picker if one happens to be
  // open). A field the user has focus in right now is left alone even
  // though the server's own answer might disagree with what they are mid-
  // typing - the live address in particular legitimately changes stop to
  // stop (a scrolled screen), so updating the *display* on a refresh is
  // correct, just not while someone's cursor is sitting in that field.
  function applyDump(newDump) {
    dump = newDump;
    bytes = Uint8Array.from(atob(dump.bytes), c => c.charCodeAt(0));

    const focused = document.activeElement;
    if (focused !== addressField) { addressField.value = hexAddress(dump.address); }
    if (focused !== widthField) { widthField.value = String(dump.width); }
    if (focused !== charRowHeightField) { charRowHeightField.value = String(dump.charRowHeight); }
    if (focused !== modeField) { modeField.value = String(dump.mode); }
    if (focused !== encodingField) { encodingField.value = String(dump.encoding); }
    if (focused !== configField) {
      configField.value = typeof dump.config === 'number' ? String(dump.config) : '';
    }

    renderPaletteSwatches();
    readout.textContent = '\\u00a0';

    // The server rendered one tall, uninterrupted, ungapped image: exactly
    // \`columns * rows\` tiles' worth of real lines, column-major (column 0's
    // own \`rows\` tiles first, in address order, then column 1's, and so
    // on - \`dump.height\`, the total real lines actually rendered, divides
    // evenly by \`columns * rows\` for the same reason). Slicing that into
    // the visible grid, with real padding on both axes, happens only here -
    // the multi-column *or* multi-row tiling never touches the server at
    // all, and reported live, an in-image black row for one axis but real
    // padding for the other looked like two different features instead of
    // one grid.
    const source = new Image();
    source.onload = () => {
      const columnPixelWidth = dump.width * 8;
      const requestedColumns = computeUnitsFitting(imgWrap.clientWidth, columnPixelWidth);
      const { rowHeightValue, rows: requestedRows } = currentRowHeightAndRows();
      currentColumns = requestedColumns;
      currentRows = requestedRows;
      currentRowHeightValue = rowHeightValue;

      const tilePixelHeight =
        Math.floor(source.naturalHeight / (requestedColumns * requestedRows)) || source.naturalHeight;

      canvas.width = requestedColumns * columnPixelWidth + (requestedColumns - 1) * PADDING;
      canvas.height = requestedRows * tilePixelHeight + (requestedRows - 1) * PADDING;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      for (let c = 0; c < requestedColumns; c++) {
        for (let r = 0; r < requestedRows; r++) {
          const sy = (c * requestedRows + r) * tilePixelHeight;
          const dx = c * (columnPixelWidth + PADDING);
          const dy = r * (tilePixelHeight + PADDING);
          ctx.drawImage(
            source, 0, sy, source.naturalWidth, tilePixelHeight,
            dx, dy, columnPixelWidth, tilePixelHeight
          );
        }
      }
    };
    source.src = 'data:image/png;base64,' + dump.png;
  }

  function hexAddress(address) {
    return address.toString(16).toUpperCase().padStart(4, '0');
  }

  window.addEventListener('message', event => {
    if (event.data && event.data.type === 'cpclib.screenFrame') {
      applyDump(event.data.dump);
    }
  });

  // The panel's own available height changes with the window and with
  // every other VS Code tab/split the user opens - re-fit on any of that,
  // debounced so a drag-resize does not flood the adapter with requests.
  let resizeTimer = null;
  new ResizeObserver(() => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => requestRender(false), 250);
  }).observe(imgWrap);

  // The very first image was rendered before this script - and therefore
  // this panel's own available height - existed at all, so it used the
  // adapter's flat, un-sized fallback. Applying it directly (not through
  // \`requestRender\`, which would post a request nobody is waiting to
  // answer for the panel's very first paint) then correcting it once, right
  // away, is cheaper than restructuring the launch-time "run -sv, then open
  // the panel" order just to avoid one extra round trip.
  applyDump({
    png: ${JSON.stringify(dump.png)},
    address: ${dump.address},
    width: ${dump.width},
    height: ${dump.height},
    mode: ${dump.mode},
    charRowHeight: ${dump.charRowHeight},
    bytes: ${JSON.stringify(dump.bytes)},
    palette: ${JSON.stringify(dump.palette)},
    hardwarePalette: ${JSON.stringify(dump.hardwarePalette)},
    encoding: ${dump.encoding},
    config: ${JSON.stringify(dump.config ?? null)},
  });
  requestRender(false);

  canvas.addEventListener('mousemove', event => {
    const rect = canvas.getBoundingClientRect();
    const mx = Math.floor((event.clientX - rect.left) * (canvas.width / rect.width));
    const my = Math.floor((event.clientY - rect.top) * (canvas.height / rect.height));
    if (mx < 0 || my < 0 || mx >= canvas.width || my >= canvas.height) { return; }

    // Which grid tile was hovered, and where within it - the thin padding
    // between tiles, on either axis, has no byte of its own.
    const columnPixelWidth = dump.width * 8;
    const columnStep = columnPixelWidth + PADDING;
    const columnIndex = Math.floor(mx / columnStep);
    const px = mx - columnIndex * columnStep;

    const tilePixelHeight = currentRowHeightValue * 2;
    const rowStep = tilePixelHeight + PADDING;
    const rowIndex = Math.floor(my / rowStep);
    const py = my - rowIndex * rowStep;

    if (
      px >= columnPixelWidth || columnIndex >= currentColumns ||
      py >= tilePixelHeight || rowIndex >= currentRows
    ) {
      readout.textContent = '\\u00a0';
      return;
    }

    // The image is already stretched to the CPC's own pixel aspect ratio
    // server-side (\`render_screen_view\`, see its own comment): 8 displayed
    // dots per byte horizontally on every mode alike, and every row doubled
    // vertically - so undoing exactly that recovers the logical column and
    // the row within this tile.
    const col = Math.floor(px / 8);
    const localLine = Math.floor(py / 2);
    // The server rendered one continuous, column-major stream of
    // \`currentColumns * currentRows\` tiles and this page sliced it into a
    // grid for display - undo that slicing to get back the real line's own
    // position in that one continuous stream, which is what the address
    // formulas below (both encodings alike) were written against.
    const line = (columnIndex * currentRows + rowIndex) * currentRowHeightValue + localLine;

    let address;
    if (dump.encoding === 1) {
      // WinAPE's "CPC" encoding: plain sequential bytes, \`charRowHeight\`
      // plays no part at all, wrapped at the full 64K space - see
      // \`ColorMatrix::from_linear_memory\`'s own doc comment.
      address = (dump.address + line * dump.width + col) & 0xFFFF;
    } else {
      // WinAPE's "Screen" encoding, matching \`ColorMatrix::from_screen_at\`
      // server-side exactly (see its own doc comment for the hardware
      // reasoning): \`MA\` (row position) advances by \`dump.width\` once
      // every \`charRowHeight\` lines (the live \`R9 + 1\`), but the raster-
      // within-row term that multiplies \`0x800\` is only 3 bits wide on
      // real hardware - it wraps at 8 regardless of how tall the row is
      // configured to be. Both terms, plus \`col\`, wrap *within the
      // screen's own 16K bank* - real CRTC/Gate Array hardware never lets
      // this arithmetic spill into a different one.
      const rowHeight = dump.charRowHeight > 0 ? dump.charRowHeight : 1;
      const ra = (line % rowHeight) % 8;
      const pageBase = dump.address & 0xC000;
      const offsetInPage = dump.address & 0x3FFF;
      address =
        pageBase + ((offsetInPage + Math.floor(line / rowHeight) * dump.width + ra * 0x800 + col) & 0x3FFF);
    }
    const value = bytes[address];
    const tileNote = (currentColumns > 1 || currentRows > 1)
      ? \` (tile column \${columnIndex}, row \${rowIndex})\`
      : '';
    readout.textContent = \`&\${address.toString(16).toUpperCase().padStart(4, '0')}\` +
      \` = &\${value.toString(16).toUpperCase().padStart(2, '0')} (\${value})\` +
      \` — column \${col}, row \${line}\${tileNote}\`;
  });
  canvas.addEventListener('mouseleave', () => { readout.textContent = '\\u00a0'; });
</script>
</body>
</html>`;
}
