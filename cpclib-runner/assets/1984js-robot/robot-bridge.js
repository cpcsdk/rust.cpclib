// Connects 1984js's emscripten module straight to Robot automation, with
// none of the Debug Adapter Protocol in between: no request/response
// envelopes, no `dap.js`, no session object - just the raw `_poc_debug_*`
// and `_poc_key` primitives the module already exports, called directly.
//
// The wire protocol is an ad hoc one of this file's own: one JSON object per
// message each way, `{"id", "cmd", ...}` downstream and `{"id", "ok", ...}`
// (or `{"id", "ok": false, "error"}`) upstream - no framing needed, since
// each Server-Sent Event and each POST body already carries exactly one
// message.
//
// Activated only when the page carries a session token (injected by the
// server, never read from the URL), so the plain browser UI is unaffected.

(function () {
  "use strict";

  const session = globalThis.__cpclib_session;
  if (!session || !session.token) { return; }
  const token = session.token;

  // Character to SDL_Scancode - the same table app.js's own keydown handler
  // uses. BASIC keywords are case-insensitive, so autotype text is
  // upper-cased rather than taught about Shift.
  const CHAR_TO_SCANCODE = {
    "\n": 40, "\r": 40, " ": 44,
  };
  for (let i = 0; i < 26; i++) { CHAR_TO_SCANCODE[String.fromCharCode(65 + i)] = 4 + i; }
  for (let i = 1; i <= 9; i++) { CHAR_TO_SCANCODE[String(i)] = 29 + i; }
  CHAR_TO_SCANCODE["0"] = 39;

  let module = null;

  // The `/debug` route (the only one that injects the session token this
  // bridge needs to activate at all - see the top of this file) also
  // injects a `<style id="cpclib-bare">` that hides everything but the
  // screen itself, `!important` and all: built for an editor embedding just
  // the picture in its own webview, which is the opposite of what Robot
  // automation needs here - the file-picker inputs' own visible labels,
  // which is the one thing on this page a click has to be able to reach.
  document.getElementById("cpclib-bare")?.remove();

  globalThis.__cpclib_robot_attach = function (m) {
    module = m;
    if (!connected) {
      connected = true;
      connect();
    }
    for (const message of pending.splice(0)) { handle(message); }
  };

  let connected = false;
  let events = null;
  const pending = [];

  function connect() {
    const base = window.location.origin;
    events = new EventSource(base + "/session/events?token=" + encodeURIComponent(token));
    events.addEventListener("message", (event) => {
      let message = null;
      try {
        message = JSON.parse(event.data);
      } catch (error) {
        console.error("[cpclib] malformed robot command", error);
        return;
      }
      if (module) { handle(message); } else { pending.push(message); }
    });
    events.addEventListener("error", () => {
      // The adapter going away is normal at the end of a session; EventSource
      // reconnects on its own, so there is nothing to do but stay quiet.
    });
  }

  function reply(id, body) {
    fetch(window.location.origin + "/session/upstream?token=" + encodeURIComponent(token), {
      method: "POST",
      body: JSON.stringify(Object.assign({ id: id }, body))
    }).catch((error) => console.error("[cpclib] robot reply failed", error));
  }

  function handle(message) {
    if (!message || typeof message.id !== "number" || typeof message.cmd !== "string") { return; }
    try {
      dispatch(message);
    } catch (error) {
      reply(message.id, { ok: false, error: String(error && error.message ? error.message : error) });
    }
  }

  function dispatch(message) {
    switch (message.cmd) {
      case "screenshot": return doScreenshot(message);
      case "keytype": return doKeytype(message);
      case "readMemory": return doReadMemory(message);
      case "writeMemory": return doWriteMemory(message);
      case "clickPoint": return doClickPoint(message);
      case "text": return doText(message);
      default:
        reply(message.id, { ok: false, error: "unknown command: " + message.cmd });
    }
  }

  // Where a real click on `selector` would need to land, in *viewport*
  // coordinates - plus the viewport's own size, so the caller can add
  // whatever window-manager chrome (title bar, borders) sits around it from
  // the OS window's own outer rectangle, which it already has and this page
  // cannot see. `window.screenX`/`screenY` would normally fold that chrome
  // in on their own and save the caller that step, but confirmed live in
  // this workspace's own dev/CI browser (a snap-packaged Chromium under a
  // plain X11 window manager) to read back as 0 regardless of the window's
  // real position - not usable here. Used for the file-picker inputs, which
  // browsers refuse to open except from a real, OS-level click - nothing
  // reachable from script alone opens a native file dialog.
  function doClickPoint(message) {
    const element = document.querySelector(message.selector);
    if (!element) {
      reply(message.id, { ok: false, error: "no element matches " + message.selector });
      return;
    }
    // The page can be taller than the viewport (confirmed live: the media
    // controls sit below the fold at this window's default size) - a rect
    // computed without this would name a point outside the window
    // altogether, missing everything, rather than the element itself.
    // Instant, not smooth: the caller reads the rect the moment this
    // returns, and a smooth scroll would still be animating then.
    element.scrollIntoView({ block: "center", behavior: "instant" });
    const rect = element.getBoundingClientRect();
    reply(message.id, {
      ok: true,
      viewportX: Math.round(rect.left + rect.width / 2),
      viewportY: Math.round(rect.top + rect.height / 2),
      innerWidth: window.innerWidth,
      innerHeight: window.innerHeight
    });
  }

  // Not used by any real Robot action, only cpclib-runner's own tests: a
  // way to check what the page's own UI believes happened (e.g.
  // `#snapshotname`, which app.js itself updates once a snapshot has really
  // loaded) - decoupled from reading memory, which a live CPU keeps
  // changing.
  function doText(message) {
    const element = document.querySelector(message.selector);
    reply(message.id, { ok: true, text: element ? element.textContent : null });
  }

  function doScreenshot(message) {
    const canvas = document.getElementById("screen");
    if (!canvas) {
      reply(message.id, { ok: false, error: "no #screen canvas on this page" });
      return;
    }
    reply(message.id, { ok: true, png: canvas.toDataURL("image/png") });
  }

  // Around a memory read/write: pause only if it was not already paused (an
  // in-flight manual debug session on the same page keeps its own state),
  // and resume only if this call is the one that paused it - so Robot
  // automation never leaves the machine halted behind it, and never steps on
  // a human already stopped at a breakpoint.
  function withPaused(fn) {
    const wasRunning = !module._poc_debug_is_paused();
    if (wasRunning) { module._poc_debug_pause(); }
    try {
      return fn();
    } finally {
      if (wasRunning) { module._poc_debug_continue(); }
    }
  }

  function doReadMemory(message) {
    const address = message.address | 0;
    const count = message.count | 0;
    const bytes = withPaused(() => {
      const out = [];
      for (let i = 0; i < count; i++) {
        out.push(module._poc_debug_mem_read((address + i) & 0xffff) & 0xff);
      }
      return out;
    });
    reply(message.id, { ok: true, bytes: bytes });
  }

  function doWriteMemory(message) {
    const address = message.address | 0;
    const bytes = message.bytes || [];
    withPaused(() => {
      for (let i = 0; i < bytes.length; i++) {
        module._poc_debug_mem_write_byte((address + i) & 0xffff, bytes[i] & 0xff);
      }
    });
    reply(message.id, { ok: true });
  }

  // Queued rather than pressed synchronously: `_poc_key` is a real keypress,
  // and the keyboard-scan interrupt that would notice it only runs a few
  // times a frame, not the instant this call returns. One scancode is
  // pressed, held, then released on the poll below, at roughly the pace a
  // real key gives app.js's own keydown/keyup handlers.
  let typeQueue = [];
  let heldTypeScancode = null;
  let typeReplyId = null;

  function doKeytype(message) {
    const text = message.text || "";
    for (const ch of text.toUpperCase()) {
      const scancode = CHAR_TO_SCANCODE[ch];
      if (scancode !== undefined) { typeQueue.push(scancode); }
    }
    // Only one autotype in flight at a time - Robot calls are sequential, so
    // this never actually races, but the id is remembered rather than
    // assumed so a reply always answers the request that is actually done.
    typeReplyId = message.id;
    if (typeQueue.length === 0 && heldTypeScancode === null) {
      reply(message.id, { ok: true });
      typeReplyId = null;
    }
  }

  setInterval(function () {
    if (!module) { return; }
    if (heldTypeScancode !== null) {
      module._poc_key(heldTypeScancode, 0);
      heldTypeScancode = null;
    } else {
      const scancode = typeQueue.shift();
      if (scancode !== undefined) {
        module._poc_key(scancode, 1);
        heldTypeScancode = scancode;
      }
    }
    if (typeReplyId !== null && typeQueue.length === 0 && heldTypeScancode === null) {
      reply(typeReplyId, { ok: true });
      typeReplyId = null;
    }
  }, 50);
})();
