import * as vscode from 'vscode';

const emulatorPanels = new Map<string, vscode.WebviewPanel>();

/**
 * Where each session's emulator is being served.
 *
 * Kept so the emulator can be opened in a real browser. That is the only place
 * its **sound** works: the page starts its AudioContext from a user gesture,
 * and a VS Code webview does not give it one that Chromium accepts - tried
 * directly, on resume, and behind a click-to-enable button, none of which
 * persuade it. A browser tab is not a workaround so much as the honest answer,
 * and the debugger keeps working while you listen: the emulator is served over
 * loopback, so the browser and the editor are looking at the same machine.
 */
export const emulatorUrls = new Map<string, string>();

export async function showEmulator(session: vscode.DebugSession, url: string | undefined): Promise<void> {
    if (!url) { return; }
    if (!vscode.workspace.getConfiguration('cpclib').get<boolean>('debug.openInWebview', true)) {
        await vscode.env.openExternal(vscode.Uri.parse(url));
        return;
    }

    const existing = emulatorPanels.get(session.id);
    if (existing) { existing.reveal(vscode.ViewColumn.Beside); return; }

    const panel = vscode.window.createWebviewPanel(
        'cpclib.emulator',
        `CPC — ${session.name}`,
        vscode.ViewColumn.Beside,
        // The emulator keeps running while the tab is hidden; without this its
        // whole state would be thrown away every time the tab loses focus.
        { enableScripts: true, retainContextWhenHidden: true },
    );

    // The adapter serves the emulator over loopback; `asExternalUri` is what
    // makes that work in a remote or codespace session too.
    // `toString(true)` skips encoding: the URL is already well-formed, and
    // re-encoding it is how a working address becomes one the page cannot use.
    const external = await vscode.env.asExternalUri(vscode.Uri.parse(url));
    panel.webview.html = emulatorHtml(external.toString(true));

    // Unlike a native emulator's window, there is nothing left running once
    // this closes: 1984js executes *inside* this very webview's iframe, so
    // closing the tab kills it, not just hides it - reported live as a
    // session left open forever with no emulator behind it. The adapter's
    // own detection (`ServedPeer::drain`'s `client_gone` check, in
    // `cpclib-dap/src/lib.rs`) only notices on the *next* outgoing SSE
    // frame, which never comes if the session was idle (stopped at a
    // breakpoint, or simply not stepping) when the tab closed - VS Code
    // itself knows the instant this fires, so ending the session here does
    // not depend on that. `stopDebugging` accepts any session, not only the
    // active one, so this is correct however many other sessions are open.
    panel.onDidDispose(() => {
        emulatorPanels.delete(session.id);
        void vscode.debug.stopDebugging(session);
    });
    emulatorPanels.set(session.id, panel);
}

export function disposeEmulator(sessionId: string): void {
    emulatorPanels.get(sessionId)?.dispose();
    emulatorPanels.delete(sessionId);
}

function emulatorHtml(url: string): string {
    const origin = new URL(url).origin;
    return `<!DOCTYPE html>
<html>
<head>
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; frame-src ${origin}; style-src 'unsafe-inline';
               script-src 'unsafe-inline';">
<style>
  html, body { margin: 0; padding: 0; height: 100%; background: #000; }
  iframe { border: 0; width: 100%; height: 100%; display: block; }
</style>
</head>
<body><iframe src="${url}" allow="autoplay; gamepad; keyboard-map"
              allowfullscreen tabindex="0"></iframe>
<script>
  // Hand the keyboard to the emulator whenever this tab is the active one.
  // A webview that keeps focus on its own document swallows every keystroke,
  // and the CPC never sees a key.
  const frame = document.querySelector('iframe');
  const focusEmulator = () => { try { frame.contentWindow.focus(); } catch (_) {} };
  window.addEventListener('focus', focusEmulator);
  document.addEventListener('pointerdown', focusEmulator);
  frame.addEventListener('load', focusEmulator);
  setTimeout(focusEmulator, 300);
</script>
</body>
</html>`;
}
