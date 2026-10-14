import * as vscode from 'vscode';
import { consoleCommand } from '../consoleCommand';

/** The emulator's own picture, as `-frame` answers it (`cpclib/frameView`). */
export interface FrameDump {
    /** The PNG, base64. */
    png: string;
    width: number;
    height: number;
    /** Where the picture sits in the beam's raster (ticks of 1/16 us, lines). */
    crop?: { x: number; y: number; w: number; h: number };
    /** The beam, in raster coordinates - where the emulation stopped drawing. */
    beam?: { x: number; y: number };
    options?: { whole: boolean; live: boolean; over: boolean };
}

const framePanels = new Map<string, vscode.WebviewPanel>();

export function disposeFrame(sessionId: string): void {
    framePanels.get(sessionId)?.dispose();
    framePanels.delete(sessionId);
}

/**
 * The picture the emulator drew - borders, raster effects, the part of the
 * frame drawn before the stop - with the beam marked where it stands. Unlike
 * the screen view (`-sv`), which renders video memory, this is the monitor's
 * side of the cable: what a raster routine is doing shows here and nowhere
 * else. One panel per session, updated in place at every stop.
 */
export function showFrame(session: vscode.DebugSession, dump: FrameDump | undefined): void {
    if (!dump || typeof dump.png !== 'string') { return; }

    const existing = framePanels.get(session.id);
    if (existing) {
        // In place: replacing the page at every step would flicker and lose
        // the zoom.
        void existing.webview.postMessage({ type: 'cpclib.frame', dump });
        return;
    }

    const panel = vscode.window.createWebviewPanel(
        'cpclib.frame',
        `CPC frame — ${session.name}`,
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
        { enableScripts: true, retainContextWhenHidden: true },
    );
    const key = session.id;
    panel.onDidDispose(() => {
        if (framePanels.get(key) === panel) { framePanels.delete(key); }
    });
    // The options row re-issues `-frame`, the same round trip as typing it.
    panel.webview.onDidReceiveMessage((message: { whole?: boolean; live?: boolean; over?: boolean }) => {
        if (!message) { return; }
        const words = [
            message.whole ? 'whole' : 'visible',
            message.live ? 'live' : 'last',
            message.over ? 'over' : 'alone',
        ];
        void consoleCommand(`-frame ${words.join(' ')}`);
    });
    framePanels.set(key, panel);
    panel.webview.html = frameHtml(dump);
    panel.reveal(vscode.ViewColumn.Beside, true);
}

function frameHtml(dump: FrameDump): string {
    const nonce = Math.random().toString(36).slice(2);
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
form { flex: 0 0 auto; display: flex; gap: 1.2em; align-items: baseline; flex-wrap: wrap; margin-bottom: 0.4em; }
label { display: flex; gap: 0.35em; align-items: baseline; font-size: 0.9em; }
#stage { flex: 1 1 auto; min-height: 0; position: relative; }
canvas { image-rendering: pixelated; max-width: 100%; max-height: 100%; display: block; }
footer { flex: 0 0 auto; color: var(--vscode-descriptionForeground); margin-top: 0.4em; font-size: 0.9em; }
</style>
</head>
<body>
<form id="options">
  <label title="The whole raster: borders, blanking and syncs"><input type="checkbox" id="whole"> whole raster</label>
  <label title="The frame being drawn, up to the beam"><input type="checkbox" id="live"> frame being drawn</label>
  <label title="The part drawn so far over the previous frame"><input type="checkbox" id="over"> over the previous frame</label>
</form>
<div id="stage"><canvas id="frame"></canvas></div>
<footer id="where"></footer>
<script nonce="${nonce}">
const vscode = acquireVsCodeApi();
const canvas = document.getElementById('frame');
const context = canvas.getContext('2d');
const where = document.getElementById('where');
const boxes = ['whole', 'live', 'over'].map(id => document.getElementById(id));

function draw(dump) {
  const options = dump.options || {};
  for (const box of boxes) { box.checked = !!options[box.id]; }
  boxes[2].disabled = !options.live;
  const image = new Image();
  image.onload = () => {
    canvas.width = dump.width;
    canvas.height = dump.height;
    context.imageSmoothingEnabled = false;
    context.drawImage(image, 0, 0);
    const crop = dump.crop || { x: 0, y: 0 };
    if (dump.beam) {
      const x = dump.beam.x - crop.x;
      const y = dump.beam.y - crop.y;
      // A cross-hair through the beam, in a colour no CPC ink is.
      context.strokeStyle = 'rgba(255, 0, 255, 0.8)';
      context.lineWidth = 1;
      context.beginPath();
      context.moveTo(0, y + 0.5); context.lineTo(dump.width, y + 0.5);
      context.moveTo(x + 0.5, 0); context.lineTo(x + 0.5, dump.height);
      context.stroke();
      context.fillStyle = 'rgba(255, 0, 255, 1)';
      context.fillRect(x - 2, y - 2, 5, 5);
      const inside = x >= 0 && y >= 0 && x < dump.width && y < dump.height;
      where.textContent = 'Beam at line ' + dump.beam.y + ', NOP ' + Math.floor(dump.beam.x / 16)
        + ' (tick ' + dump.beam.x + ')' + (inside ? '' : ' - outside this picture: tick "whole raster"');
    } else {
      where.textContent = 'This emulator does not say where the beam is.';
    }
  };
  image.src = 'data:image/png;base64,' + dump.png;
}

for (const box of boxes) {
  box.addEventListener('change', () => {
    vscode.postMessage({ whole: boxes[0].checked, live: boxes[1].checked, over: boxes[2].checked });
  });
}
window.addEventListener('message', event => {
  if (event.data && event.data.type === 'cpclib.frame') { draw(event.data.dump); }
});
draw(${JSON.stringify(dump)});
</script>
</body>
</html>`;
}
