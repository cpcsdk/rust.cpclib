import { LanguageClient } from 'vscode-languageclient/node';
import { activeEmbeddedTaskWriters } from '../tasks/codeLensRunners';

/// Registers the *one* `window/logMessage` handler this extension ever
/// installs beyond vscode-languageclient's own built-in one - and
/// necessarily *replaces* it: `LanguageClient.onNotification`/the
/// underlying `vscode-jsonrpc` connection only support one handler per
/// notification method (a plain `Map.set` keyed by method name in both
/// layers - confirmed directly in `vscode-languageclient/lib/common/client.js`
/// and `vscode-jsonrpc/lib/common/connection.js`), so registering a second,
/// *additional* listener isn't possible - it would just silently steal the
/// slot from whichever registers first.
///
/// This handler therefore reimplements vscode-languageclient's own default
/// `window/logMessage` handling *exactly* (dispatching to `client.error`/
/// `warn`/`info`/`debug` for those message types, `outputChannel.appendLine`
/// directly for anything else - which is the path `MessageType.Log` (4)
/// takes, what the server actually uses for streamed build stdout/stderr;
/// see `vscode-languageclient/lib/common/client.js`'s own `doStart` for the
/// original this must stay byte-for-byte in sync with) so nothing regresses
/// for any other feature that relies on server log messages reaching the
/// output channel - then additionally mirrors the same line into every
/// currently-active embedded-rule task terminal.
///
/// Called from `client.start().then(...)`, not synchronously in `activate()`:
/// vscode-languageclient installs its *own* built-in handler directly on the
/// connection during `doStart()`, so registering ours only after `start()`
/// resolves guarantees the connection already exists and our call is the
/// one that ends up owning the method's single handler slot.
export function installLogMessageMirror(client: LanguageClient): void {
    client.onNotification('window/logMessage', (message: { type: number; message: string }) => {
        switch (message.type) {
            case 1: client.error(message.message, undefined, false); break;   // MessageType.Error
            case 2: client.warn(message.message, undefined, false); break;    // MessageType.Warning
            case 3: client.info(message.message, undefined, false); break;    // MessageType.Info
            case 5: client.debug(message.message, undefined, false); break;   // MessageType.Debug
            default: client.outputChannel.appendLine(message.message);        // MessageType.Log (4), and anything else
        }
        for (const write of activeEmbeddedTaskWriters) {
            write(message.message + '\r\n');
        }
    });
}
