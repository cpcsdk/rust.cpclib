// A hex/decimal/binary calculator - a mix of asm-code-lens's own hex
// calculator (a real accumulator: type a bare number or an operator-prefixed
// one in either the decimal or hex column and press Enter, and it commits
// onto a running total, appending a line to both columns' scrolling
// history) and a plain single-value converter (binary, plus an 8-bit/16-bit
// signed/unsigned decomposition of the current total - reading a byte or
// word out of its two's-complement bit pattern is routine enough in Z80
// work to be worth a dedicated view, and isn't part of asm-code-lens's own
// dec/hex-only calculator). Pure client-side (`enableScripts: true`, no
// server round-trip) - unlike `hexView.ts`'s read-only structural viewer,
// there is no file to read.

import * as vscode from 'vscode';
import { escapeHtml } from '../shared/html';

let panel: vscode.WebviewPanel | undefined;

export function registerHexCalculator(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.openHexCalculator', () => {
            // Singleton, like `screen.ts`'s per-session panel - there is only
            // ever one calculator worth having open.
            if (panel) {
                panel.reveal(vscode.ViewColumn.Beside, true);
                return;
            }
            panel = vscode.window.createWebviewPanel(
                'cpclib.hexCalculator',
                'CPC Hex Calculator',
                { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
                { enableScripts: true, retainContextWhenHidden: true },
            );
            panel.webview.html = hexCalculatorHtml();
            panel.onDidDispose(() => { panel = undefined; });
        }),
    );
}

function hexCalculatorHtml(): string {
    const title = escapeHtml('CPC Hex Calculator');
    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline';">
<title>${title}</title>
<style>
  body { font-family: var(--vscode-editor-font-family, monospace);
         color: var(--vscode-editor-foreground); padding: 10px 14px; font-size: 0.95em; }
  table.ledger { border-collapse: collapse; width: 100%; table-layout: fixed; }
  table.ledger th { text-align: left; color: var(--vscode-descriptionForeground);
                    font-weight: normal; padding-bottom: 4px; }
  .history { height: 12em; overflow-y: auto; border: 1px solid var(--vscode-input-border, #444);
             padding: 4px 8px; white-space: pre; line-height: 1.5em; }
  .history div:last-child { font-weight: 600; }
  input { font-family: inherit; font-size: 1.05em; width: 100%; box-sizing: border-box;
          padding: 6px 8px; margin-top: 6px; background: var(--vscode-input-background);
          color: var(--vscode-input-foreground); border: 1px solid var(--vscode-input-border, transparent); }
  input.invalid { border-color: var(--vscode-inputValidation-errorBorder, #f14c4c); }
  button { margin-top: 8px; padding: 4px 10px; }
  .hint { color: var(--vscode-descriptionForeground); margin-top: 6px; font-size: 0.85em; }
  .section { margin-top: 18px; }
  table.current { border-collapse: collapse; margin-top: 6px; width: 100%; }
  table.current td { padding: 3px 8px; vertical-align: top; }
  td.label { color: var(--vscode-descriptionForeground); white-space: nowrap; width: 1%; }
  td.value { font-weight: 600; word-break: break-all; }
  .oob { color: var(--vscode-errorForeground, #f14c4c); }
</style>
</head>
<body>
<table class="ledger">
  <tr><th>Decimal</th><th>Hex</th></tr>
  <tr>
    <td><div class="history" id="dec_history"></div></td>
    <td><div class="history" id="hex_history"></div></td>
  </tr>
  <tr>
    <td><input id="dec_input" placeholder="0, or +5, -12, *3, /2" /></td>
    <td><input id="hex_input" placeholder="0, or +A, -1C, *3, /2" /></td>
  </tr>
</table>
<p class="hint">Type a bare number, or an operator (<code>+ - * /</code>) followed by one, and press Enter to apply it to the running total.</p>
<button id="clear">Clear</button>

<div class="section">
  <table class="current">
    <tr><th colspan="2" style="text-align:left; color:var(--vscode-descriptionForeground); font-weight:normal;">Current total</th></tr>
    <tr><td class="label">Binary</td><td class="value" id="bin">%00000000</td></tr>
    <tr><td class="label">8-bit unsigned</td><td class="value" id="u8">0</td></tr>
    <tr><td class="label">8-bit signed</td><td class="value" id="s8">0</td></tr>
    <tr><td class="label">16-bit unsigned</td><td class="value" id="u16">0</td></tr>
    <tr><td class="label">16-bit signed</td><td class="value" id="s16">0</td></tr>
  </table>
</div>

<script>
(function () {
  const hexPrefix = '&';
  const decHistory = document.getElementById('dec_history');
  const hexHistory = document.getElementById('hex_history');
  const decInput = document.getElementById('dec_input');
  const hexInput = document.getElementById('hex_input');
  const el = {
    bin: document.getElementById('bin'),
    u8: document.getElementById('u8'),
    s8: document.getElementById('s8'),
    u16: document.getElementById('u16'),
    s16: document.getElementById('s16'),
  };

  let total = 0;

  function pad(n, width, base) {
    return n.toString(base).toUpperCase().padStart(width, '0');
  }

  // Same variable-width hex rendering asm-code-lens's own calculator uses:
  // 2/4/8 digits depending on magnitude, sign handled as a leading '-' on
  // the absolute value rather than two's-complement (this is a decimal-ish
  // signed display for the ledger, not the two's-complement byte/word view
  // below, which is deliberately separate).
  function hexOf(n) {
    let sign = '';
    let v = n;
    if (v < 0) { sign = '-'; v = -v; }
    const width = v > 0xffff ? 8 : v > 0xff ? 4 : 2;
    return sign + hexPrefix + pad(v, width, 16);
  }

  function appendHistoryLine(container, text) {
    const line = document.createElement('div');
    line.textContent = text;
    container.appendChild(line);
    container.scrollTop = container.scrollHeight;
  }

  function renderCurrentValue() {
    const n = total;
    const u8v = n & 0xff;
    const s8v = u8v >= 0x80 ? u8v - 0x100 : u8v;
    const u16v = n & 0xffff;
    const s16v = u16v >= 0x8000 ? u16v - 0x10000 : u16v;

    // Same two's-complement bit pattern the byte/word rows below already
    // show, just spelled out in binary - not a sign-and-magnitude rendering
    // like the ledger's own hex column uses.
    const fitsInByte = n <= 0x7f && n >= -0x80;
    el.bin.textContent = '%' + pad(fitsInByte ? u8v : u16v, fitsInByte ? 8 : 16, 2);

    const oob8 = n > 0xff || n < 0;
    const oob16 = n > 0xffff || n < 0;
    el.u8.textContent = String(u8v) + (oob8 ? ' (wrapped)' : '');
    el.u8.className = 'value' + (oob8 ? ' oob' : '');
    el.s8.textContent = String(s8v) + (oob8 ? ' (wrapped)' : '');
    el.s8.className = 'value' + (oob8 ? ' oob' : '');
    el.u16.textContent = String(u16v) + (oob16 ? ' (wrapped)' : '');
    el.u16.className = 'value' + (oob16 ? ' oob' : '');
    el.s16.textContent = String(s16v) + (oob16 ? ' (wrapped)' : '');
    el.s16.className = 'value' + (oob16 ? ' oob' : '');
  }

  function commit(n, operator) {
    // Echo the typed expression first (only meaningful when there's an
    // operator - a bare number *is* the new total, so echoing it twice
    // would be redundant), then always append the resulting total - same
    // two-line shape asm-code-lens's own calculator uses.
    if (operator) {
      appendHistoryLine(decHistory, operator + n);
      appendHistoryLine(hexHistory, operator + hexOf(n));
    }
    switch (operator) {
      case '+': total += n; break;
      case '-': total -= n; break;
      case '*': total *= n; break;
      case '/': total = Math.trunc(total / n); break;
      default: total = n; break;
    }
    appendHistoryLine(decHistory, String(total));
    appendHistoryLine(hexHistory, hexOf(total));
    renderCurrentValue();
  }

  function makeHandler(input, radix) {
    const pattern = radix === 16 ? /^([+\-*/]?)\s*([0-9a-f]*)$/i : /^([+\-*/]?)\s*(\d*)$/;
    return function (event) {
      const raw = input.value.trim();
      const match = pattern.exec(raw);
      if (!match) {
        input.classList.add('invalid');
        return;
      }
      input.classList.remove('invalid');
      if (event.key !== 'Enter') { return; }
      const digits = match[2];
      if (!digits) { return; }
      const n = parseInt(digits, radix);
      commit(n, match[1]);
      input.value = '';
    };
  }

  decInput.addEventListener('keyup', makeHandler(decInput, 10));
  hexInput.addEventListener('keyup', makeHandler(hexInput, 16));

  document.getElementById('clear').addEventListener('click', function () {
    total = 0;
    decHistory.innerHTML = '';
    hexHistory.innerHTML = '';
    decInput.value = '';
    hexInput.value = '';
    renderCurrentValue();
  });

  renderCurrentValue();
}());
</script>
</body>
</html>`;
}
