// Connects 1984js's in-page DAP server to an external debugger.
//
// Upstream ships a complete DAP 1.71 implementation (`dap.js`, exported as
// `globalThis.JS1984DAP`) but drives it only from its own on-page monitor:
// "the browser UI currently uses the same protocol engine in process; it does
// not yet expose a WebSocket, TCP, or stdio endpoint to an external IDE".
// This adds that endpoint, and nothing else.
//
// Two things about the upstream engine shape this file:
//
//  * `Connection.push()` is synchronous and *returns* the bytes to send back.
//    Queued events are only flushed by `push()` or `sync()`, so a `stopped`
//    event raised while the program runs would never arrive on its own - hence
//    the polling loop below. This is the single most important detail here.
//  * It expects Content-Length framing, so we keep that over the wire rather
//    than reframing: upstream's parser stays the only parser.
//
// Activated only when the page is opened with `?dap=1&token=...`, so the plain
// browser UI is completely unaffected.

(function () {
  "use strict";

  // The server injects this into the page it serves at /debug. Nothing is read
  // from the URL: this page is loaded inside an editor tab as well as in a
  // browser, and a query string does not reliably survive that trip.
  const session = globalThis.__cpclib_session;
  if (!session || !session.token) { return; }  // plain browsing, no debugger
  const token = session.token;

  // Character to SDL_Scancode, the same table app.js's own keydown handler
  // uses (`CODE2SCAN`) - so a typed 'R' presses the identical key a real
  // keypress would. BASIC keywords are case-insensitive, so autotype text is
  // upper-cased rather than taught about Shift; nothing here needs to type a
  // literal string, only commands like "RUN".
  const CHAR_TO_SCANCODE = {
    "\n": 40, "\r": 40, " ": 44,
  };
  for (let i = 0; i < 26; i++) { CHAR_TO_SCANCODE[String.fromCharCode(65 + i)] = 4 + i; }
  for (let i = 1; i <= 9; i++) { CHAR_TO_SCANCODE[String(i)] = 29 + i; }
  CHAR_TO_SCANCODE["0"] = 39;

  // `__cpclib_attach` is called by the one line the install step adds to
  // app.js, handing us the emscripten module and the DAP session it built.
  let handle = null;
  const pending = [];

  // The hook is appended after *every* `createMlDapSession()` call, and one of
  // those sits inside `adoptCoreMlBreakpoints()` - which `loadSnapshotFile`
  // itself calls. So attach re-enters during a snapshot load, and connecting
  // again there would open a second event stream the server refuses. The
  // handle is refreshed each time (the session object is rebuilt), but the
  // connection is made once.
  let connected = false;
  let ownSession = null;
  let ownConnection = null;

  globalThis.__cpclib_attach = function (attached) {
    handle = attached;

    // A *separate* DAP session over the same emulator backend, rather than the
    // one the page's own ML monitor uses.
    //
    // `takeEvents()` empties the queue it reads, and the monitor drains its
    // session on a timer - so sharing one session means whichever polls first
    // wins, and a `stopped` raised by a breakpoint would frequently vanish into
    // the monitor before the debugger ever saw it. Two sessions over one
    // backend each keep their own queue; the emulator underneath is the same.
    if (!ownConnection && handle.session && handle.session.backend) {
      try {
        ownSession = new JS1984DAP.Session(handle.session.backend);
        ownConnection = new JS1984DAP.Connection(ownSession);
      } catch (error) {
        report("could not open a debug session: " + error);
        return;
      }
    }

    if (!connected) {
      connected = true;
      connect();
    }
    for (const chunk of pending.splice(0)) { deliver(chunk); }
    // The program under test. Without this the emulator comes up on the BASIC
    // prompt and the whole session debugs nothing.
    loadSessionSnapshot();
  };

  // Downstream is an EventSource and upstream a POST: both plain HTTP, so
  // there is no handshake to get wrong and no frame masking to implement.
  let events = null;

  function connect() {
    const base = window.location.origin;
    const auth = "?token=" + encodeURIComponent(token);
    events = new EventSource(base + "/session/events" + auth);

    events.addEventListener("message", (event) => {
      // The adapter sends the JSON body alone, because Server-Sent Events
      // strip carriage returns and the emulator's parser scans for the four
      // bytes "\r\n\r\n" exactly. The frame is rebuilt here, with a *byte*
      // length - Content-Length counts bytes, and a message with any non-ASCII
      // character in it would otherwise be truncated.
      const body = event.data;
      const length = new TextEncoder().encode(body).length;
      const chunk = "Content-Length: " + length + "\r\n\r\n" + body;
      if (ownConnection) { deliver(chunk); } else { pending.push(chunk); }
    });
    events.addEventListener("error", () => {
      // The adapter going away is normal at the end of a session; EventSource
      // reconnects on its own, so there is nothing to do but stay quiet.
    });
  }

  function deliver(chunk) {
    if (!ownConnection) { pending.push(chunk); return; }
    // Watches are not a DAP request: the emulator's write-watch channels sit on
    // the emscripten module, not on its DAP session, and `dap.js` refuses any
    // command it does not know. So `cpclib/setWatches` is answered here and
    // never reaches upstream's parser.
    const intercepted = interceptWatchRequest(chunk);
    if (intercepted) { send(intercepted); return; }
    let out = "";
    try {
      out = ownConnection.push(chunk);
    } catch (error) {
      // A malformed frame must not take the emulator down with it.
      console.error("[cpclib] DAP dispatch failed", error);
      return;
    }
    send(out);
  }

  function send(text) {
    if (!text) { return; }
    fetch(window.location.origin + "/session/dap?token=" + encodeURIComponent(token), {
      method: "POST",
      body: text
    }).catch((error) => console.error("[cpclib] DAP send failed", error));
  }

  // Write watches: labelled addresses the debugger wants to be told about.
  //
  // The emulator has a fixed number of channels and says so by refusing to arm
  // one; the count is not written down here, because it belongs to this
  // emulator and this version of it. Whatever does not fit is named in the
  // answer rather than dropped.
  let armedWatches = [];

  // Autotype: scancodes still to press, and the one currently held down (if
  // any) - see `pumpTypeQueue`.
  let typeQueue = [];
  let heldTypeScancode = null;

  // One step of the queue per call: release whatever is held, or press the
  // next scancode. Called from the 50ms poll below, so every key gets a
  // 50ms-ish press - generous next to the CPC's own keyboard-scan rate, and
  // nothing here is timing-critical enough to want less slack.
  function pumpTypeQueue() {
    const module = handle && handle.module;
    if (!module || typeof module._poc_key !== "function") { typeQueue = []; return; }
    if (heldTypeScancode !== null) {
      module._poc_key(heldTypeScancode, 0);
      heldTypeScancode = null;
      return;
    }
    const scancode = typeQueue.shift();
    if (scancode === undefined) { return; }
    module._poc_key(scancode, 1);
    heldTypeScancode = scancode;
  }

  function interceptWatchRequest(chunk) {
    const separator = chunk.indexOf("\r\n\r\n");
    if (separator < 0) { return null; }
    let message = null;
    try {
      message = JSON.parse(chunk.slice(separator + 4));
    } catch (_) { return null; }
    // Resuming is the moment sound matters, and by then the user has usually
    // interacted with the window at least once - so it is worth one more try.
    if (message && (message.command === "continue" || message.command === "launch")) {
      wakeAudio();
    }
    // The CRTC and the Gate Array are not on the debug API at all - the
    // emulator exposes the Z80, memory, breakpoints and stepping, and nothing
    // about the chips that decide what the Z80's work looks like. But a
    // snapshot header carries every one of their registers and counters, and
    // the emulator can write one. So the machine state is fetched by saving a
    // snapshot to the wasm filesystem and handing back the bytes; the adapter
    // parses it with the same code that reads a `.sna` from disk.
    if (message && message.command === "cpclib/machineState") {
      return machineStateResponse(message);
    }
    // Queued rather than typed here: `_poc_key` is a real keypress, and the
    // keyboard-scan interrupt that would notice it only runs a few times a
    // frame, not synchronously the moment this call returns. `pumpTypeQueue`
    // (driven off the poll timer below) presses one key, releases it on the
    // next tick, and moves on - the same pace app.js's own keydown/keyup
    // handlers give a real key.
    if (message && message.command === "cpclib/autotype") {
      const text = (message.arguments && message.arguments.text) || "";
      for (const ch of text.toUpperCase()) {
        const scancode = CHAR_TO_SCANCODE[ch];
        if (scancode !== undefined) { typeQueue.push(scancode); }
      }
      const response = JSON.stringify({
        seq: 0, type: "response", request_seq: message.seq,
        success: true, command: message.command, body: {}
      });
      return "Content-Length: " + new TextEncoder().encode(response).length +
        "\r\n\r\n" + response;
    }
    if (!message || message.command !== "cpclib/setWatches") { return null; }

    const module = handle && handle.module;
    const requested = (message.arguments && message.arguments.watches) || [];
    const applied = [];
    const rejected = [];

    if (!module || typeof module._poc_debug_watch_set !== "function") {
      for (const watch of requested) { rejected.push(watch.label); }
    }
    else {
      for (const slot of armedWatches.keys()) { module._poc_debug_watch_clear(slot); }
      armedWatches = [];
      for (const watch of requested) {
        const slot = armedWatches.length;
        // A refusal is how the emulator reports that its channels are full;
        // there is no call that says how many there are.
        if (module._poc_debug_watch_set(slot, watch.address & 0xFFFF) !== 0) {
          rejected.push(watch.label);
          continue;
        }
        armedWatches.push(watch);
        applied.push(watch.label);
      }
    }

    const response = JSON.stringify({
      seq: 0,
      type: "response",
      request_seq: message.seq,
      success: true,
      command: message.command,
      body: { applied: applied, rejected: rejected }
    });
    return "Content-Length: " + new TextEncoder().encode(response).length +
      "\r\n\r\n" + response;
  }

  // The emulator records writes in a ring keyed by a serial number, which is
  // read rather than consumed - so polling it here does not take anything away
  // from the page's own monitor, which keeps its own position in the same ring.
  let watchSerial = 0;
  let watchSerialPrimed = false;

  function pollWatchEvents() {
    const module = handle && handle.module;
    if (!module || !ownSession || typeof module._poc_debug_watch_serial !== "function") {
      return;
    }
    const newest = module._poc_debug_watch_serial() >>> 0;
    // Whatever happened before the debugger attached is not this session's.
    if (!watchSerialPrimed) { watchSerial = newest; watchSerialPrimed = true; return; }
    if (newest === watchSerial) { return; }

    let first = watchSerial + 1;
    // The ring holds a bounded history; asking for an entry that has scrolled
    // out returns a slot of -1, so start from the oldest one still there.
    if (newest - first >= 64) { first = newest - 63; }
    for (let serial = first; serial <= newest; serial++) {
      const slot = module._poc_debug_watch_event_slot(serial);
      if (slot < 0) { continue; }
      const watch = armedWatches[slot];
      ownSession.notifyWrite({
        address: module._poc_debug_watch_event_addr(serial),
        pc: module._poc_debug_watch_event_pc(serial),
        oldValue: module._poc_debug_watch_event_old(serial),
        newValue: module._poc_debug_watch_event_new(serial),
        label: watch ? watch.label : "watch_" + slot
      });
    }
    watchSerial = newest;
  }

  let audioReviewTick = 0;

  // Events are queued, not pushed.
  //
  // `Connection.push()`/`sync()` are the only things that flush the engine's
  // event queue, so without this poll the program would hit a breakpoint, stop
  // dead, and nobody would ever be told: no `stopped` event, no toolbar, no
  // call stack. It is the single most important line in this file.
  setInterval(function () {
    if (!ownConnection) { return; }
    try {
      // `frame()` is what steps the CPU, and it only runs from
      // requestAnimationFrame - which a backgrounded tab suspends. Without
      // this, a program never reaches a breakpoint ahead of wherever it was
      // when the tab lost focus, no matter how long the wait: it is not that
      // the stop goes unreported, it is that it never happens. This shares
      // frame()'s own lastFrame counter (see STEP_HOOK), so there is nothing
      // to double-count once requestAnimationFrame resumes.
      if (document.hidden && typeof globalThis.__cpclib_step_catchup === "function") {
        globalThis.__cpclib_step_catchup(performance.now());
      }
      // Writes first: `sync()` is what flushes the queue, so an event raised
      // here goes out in the same pass rather than waiting fifty more
      // milliseconds.
      pollWatchEvents();
      pumpTypeQueue();
      send(ownConnection.sync());
      // Twenty polls apart: often enough that the prompt appears promptly and
      // disappears the moment sound starts, rare enough to cost nothing.
      if (++audioReviewTick % 20 === 0) { reviewAudio(); }
    } catch (error) {
      report("polling the debug session failed: " + error);
    }
  }, 50);

  // The snapshot to debug, fetched from the adapter that served this page.
  // Upstream has no URL parameter for a snapshot - only diska/diskb/cartridge -
  // so this is how the program under test gets in.
  //
  // It goes through upstream's own `loadSnapshotFile` rather than calling
  // `poc_load_snapshot` directly: that function also resets audio, adjusts the
  // memory size to the snapshot's, and arms the emulator's breakpoint channels
  // from the breakpoint chunks the snapshot carries. Doing it by hand would
  // silently drop all of that.
  let loading = false;
  let loaded = false;

  /// Anything that goes wrong here leaves an emulator sitting on the BASIC
  /// prompt, which looks like "the debugger did nothing" - so failures are
  /// reported where they can be seen rather than only in the console.
  function report(message) {
    console.error("[cpclib] " + message);
    try {
      const banner = document.createElement("div");
      banner.textContent = "cpclib: " + message;
      banner.style.cssText =
        "position:fixed;top:0;left:0;right:0;z-index:99999;padding:6px 10px;" +
        "background:#a11;color:#fff;font:12px monospace";
      document.body.appendChild(banner);
    } catch (_) { /* no DOM yet; the console line still stands */ }
  }

  async function loadSessionSnapshot() {
    if (loaded || loading) { return; }
    if (!handle || typeof handle.loadSnapshot !== "function") {
      report("the emulator did not expose its snapshot loader; the patch may be stale");
      return;
    }
    loading = true;
    try {
      const response = await fetch(
        window.location.origin + "/session/snapshot.sna?token=" + encodeURIComponent(token)
      );
      // 404 is normal: a session may legitimately have no snapshot.
      if (response.status === 404) { loaded = true; return; }
      if (!response.ok) {
        report("the snapshot could not be fetched (" + response.status + ")");
        return;
      }
      const bytes = new Uint8Array(await response.arrayBuffer());
      await handle.loadSnapshot(new File([bytes], "session.sna"));
      releaseSnapshotBreakpoints();
      wakeAudio();
      loaded = true;
    } catch (error) {
      report("loading the snapshot failed: " + error);
    } finally {
      loading = false;
    }
  }

  // Hand every breakpoint channel back to the debugger.
  //
  // Loading a snapshot makes the page arm the breakpoints its chunks carry,
  // through *its own* DAP session. Two sessions over one backend each own the
  // slots they set, so those channels are ones this bridge's session cannot
  // clear - and an editor asking to remove such a breakpoint would be answered
  // "done" while the program went on stopping there.
  //
  // The adapter knows every one of those breakpoints already: they come from
  // the same assemble as the source map, and it arms them itself. So the right
  // owner is the adapter, and the channels are released here rather than left
  // held by a UI nobody is looking at.
  // The CPU is *not* paused from here.
  //
  // It was, briefly, and it broke stepping: `_poc_debug_pause()` halts the core
  // directly, while the emulator's DAP session tracks a `running` flag of its
  // own. Pausing behind its back left the two disagreeing - it refused
  // `continue` as "notStopped", and answered every `stepIn` with success and a
  // `stopped` event while the program counter never moved. The debugger has to
  // go through the session's own state machine, which is what the adapter's
  // `pause` request does.

  function releaseSnapshotBreakpoints() {
    const module = handle && handle.module;
    if (!module || typeof module._poc_debug_breakpoint_clear !== "function") { return; }
    // The channel count is the emulator's business and changes between
    // versions, so it is discovered by asking rather than written down: an
    // out-of-range slot answers "not enabled" and costs one call.
    for (let slot = 0; slot < 256; slot++) {
      if (module._poc_debug_breakpoint_enabled(slot)) { module._poc_debug_breakpoint_clear(slot); }
    }
  }

  // The whole machine, as a snapshot.
  //
  // Expensive - it is a full 64/128K save - so it is only ever done when
  // something asks, which the adapter arranges by declaring the chip scopes
  // "expensive" so the editor requests them only when they are expanded.
  function machineStateResponse(message) {
    const module = handle && handle.module;
    let body = { error: "the emulator did not expose a snapshot writer" };
    const path = "/cpclib-state.sna";

    if (module && typeof module.ccall === "function") {
      try {
        const rc = module.ccall("poc_save_snapshot", "number", ["string"], [path]);
        if (rc !== 0) { throw new Error("the snapshot encoder rejected the machine state"); }
        const bytes = new Uint8Array(module.FS.readFile(path));
        let binary = "";
        // Chunked: `apply` on a 128K array overflows the argument stack.
        for (let i = 0; i < bytes.length; i += 0x8000) {
          binary += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
        }
        body = { snapshot: btoa(binary) };
      } catch (error) {
        body = { error: String(error && error.message ? error.message : error) };
      } finally {
        try { module.FS.unlink(path); } catch (_) { /* nothing was written */ }
      }
    }

    const response = JSON.stringify({
      seq: 0,
      type: "response",
      request_seq: message.seq,
      success: true,
      command: message.command,
      body: body
    });
    return "Content-Length: " + new TextEncoder().encode(response).length +
      "\r\n\r\n" + response;
  }

  // Sound.
  //
  // The page starts its AudioContext from a `pointerdown` on the window, which
  // in a browser happens the moment you click anything. In an editor you drive
  // the whole session from the debug toolbar and never click *inside* the
  // emulator's frame, so that gesture never arrives.
  //
  // Asking for it directly is tried first, but a browser may refuse to *resume*
  // a context that was not created from a user gesture - and that refusal is
  // silent. So the state is checked continuously rather than once: whenever
  // audio is not actually running, a button says so, and clicking it is the
  // gesture the browser was waiting for. Anything less leaves "no sound" and
  // no explanation, which is where this started.
  let audioPrompt = null;
  let audioAsked = false;

  function wakeAudio() {
    if (!handle) { return; }
    audioAsked = true;
    if (typeof handle.startAudio === "function") {
      try {
        handle.startAudio();
      } catch (error) {
        console.error("[cpclib] could not start audio", error);
      }
      return;
    }
    // An emulator patched by an older revision does not hand us `startAudio`.
    // Its own one-shot `pointerdown` listener still exists, so poking that is
    // the next best thing - it creates the context, and the prompt below
    // supplies the real gesture that lets it play.
    try {
      window.dispatchEvent(new Event("pointerdown"));
    } catch (_) { /* nothing else to try */ }
  }

  /// Whether sound is actually coming out, as opposed to having been asked for.
  function audioIsRunning() {
    if (!handle || typeof handle.audioContext !== "function") { return null; }
    const context = handle.audioContext();
    if (!context) { return false; }
    return context.state === "running";
  }

  function reviewAudio() {
    if (!audioAsked) { return; }
    const running = audioIsRunning();
    // `null` is "this build cannot tell us" - offer the button rather than
    // assume it worked, because assuming is what produced silence.
    if (running === true) { dismissAudioPrompt(); }
    else { showAudioPrompt(); }
  }

  function dismissAudioPrompt() {
    if (audioPrompt) { audioPrompt.remove(); audioPrompt = null; }
  }

  function showAudioPrompt() {
    if (audioPrompt || !document.body) { return; }
    try {
      audioPrompt = document.createElement("button");
      // The debug page hides everything that is not the screen, and this
      // button is a child of `body` like the rest of the furniture - so it
      // needs the one id that stylesheet exempts, or the offer to fix the
      // sound is itself invisible.
      audioPrompt.id = "cpclib-audio";
      audioPrompt.textContent = "\u{1F507} No sound here - open in a browser";
      audioPrompt.title =
        "A VS Code webview does not give the page a user gesture the browser " +
        "accepts for audio. Run the command \"CPClib: Open the emulator in a " +
        "browser\" to hear it - it is the same machine, served over loopback.";
      audioPrompt.style.cssText =
        "position:fixed;bottom:12px;right:12px;z-index:99999;padding:8px 14px;" +
        "background:#1b6;color:#fff;border:0;border-radius:4px;cursor:pointer;" +
        "font:13px sans-serif;box-shadow:0 2px 8px rgba(0,0,0,.4)";
      // The click *is* the gesture the browser was waiting for, so starting
      // audio from inside the handler is what makes this work at all.
      audioPrompt.addEventListener("click", () => {
        wakeAudio();
        // Let `resume()` settle before deciding whether it took.
        setTimeout(reviewAudio, 300);
      });
      document.body.appendChild(audioPrompt);
    } catch (_) { /* no DOM yet; the next review will try again */ }
  }

  // Keys reach the CPC.
  //
  // The emulator listens for keystrokes on its canvas, and a canvas only gets
  // them when it has focus - which in its own page you give it by clicking the
  // picture. In an editor tab the picture is all there is, and having to click
  // it before every key is not something anyone should have to learn.
  //
  // So focus follows the tab: when the frame becomes active, the canvas takes
  // focus, and any click anywhere in it puts focus back.
  function focusScreen() {
    const canvas = document.getElementById("screen");
    if (!canvas) { return; }
    try {
      canvas.focus({ preventScroll: true });
    } catch (_) {
      canvas.focus();
    }
  }

  window.addEventListener("focus", focusScreen);
  document.addEventListener("pointerdown", focusScreen);
  // The frame is usually still laying out when the bridge loads.
  setTimeout(focusScreen, 200);

  globalThis.__cpclib_load_snapshot = loadSessionSnapshot;
})();
