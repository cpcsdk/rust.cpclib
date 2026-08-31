import * as vscode from 'vscode';
import { ExtensionContext } from 'vscode';
import { client } from '../lsp/client';
import { pickWorkspaceFile } from '../shared/pickWorkspaceFile';

// Per-file "last SID wait-line-count tried" memory, set once in
// `registerMusic()` - see `musicCommandArgs`. Tuning this value is
// inherently trial-and-error (raise it until playback stops freezing), so
// remembering what was last tried *for this specific song* saves re-typing
// it on every run; `globalState` persists across window reloads/restarts,
// which matters since a tuning session realistically spans several of those.
let musicSidWaitLineCountMemory: vscode.Memento;

/** `cpclib.musicSidInfo`'s response shape - see its own doc comment in backend.rs. */
interface MusicSidInfo {
    isSid: boolean;
    defaultWaitLineCount: number;
}

/** `musicSidWaitLineCountMemory` key for `fileName`'s last-tried value. */
function sidWaitLineCountMemoryKey(fileName: string): string {
    return `cpclib.sidWaitLineCount:${fileName}`;
}

/**
 * Checks whether `fileName` uses Arkos Tracker's experimental SID feature
 * and, if so, prompts for a wait-line-count safety-margin override -
 * pre-filled with the configured default (`[music] sid_wait_line_count` in
 * `cpclib-lsp.toml`), so the common case needs no config file at all. Hand-
 * editing that file isn't something a musician using this feature should
 * have to do just to get playback that doesn't freeze.
 *
 * Returns the `arguments` array to send to `cpclib.musicPlay`/
 * `cpclib.musicBuildDsk` (`[fileName]`, or `[fileName, chosenValue]` for a
 * SID song), or `undefined` if the user cancelled the prompt - the caller
 * should then abort entirely rather than silently falling back to the
 * config default.
 */
async function musicCommandArgs(fileName: string): Promise<unknown[] | undefined> {
    let info: MusicSidInfo | null;
    try {
        info = await client.sendRequest<MusicSidInfo | null>('workspace/executeCommand', {
            command: 'cpclib.musicSidInfo',
            arguments: [fileName],
        });
    } catch {
        info = null;
    }
    if (!info?.isSid) {
        return [fileName];
    }

    // Prefer whatever was last tried for *this song* over the config
    // default - tuning this value is inherently trial-and-error, so
    // remembering it saves re-typing on every run of the same tune.
    const memoryKey = sidWaitLineCountMemoryKey(fileName);
    const remembered = musicSidWaitLineCountMemory.get<number>(memoryKey);

    const picked = await vscode.window.showInputBox({
        title: 'SID wait-line-count',
        prompt: 'This song uses Arkos Tracker\'s SID feature. Safety margin '
            + '(in scanlines) between engine updates - raise it if playback freezes.',
        value: String(remembered ?? info.defaultWaitLineCount),
        validateInput: v => /^\d+$/.test(v) ? undefined : 'Enter a whole number',
    });
    if (picked === undefined) { return undefined; }

    const value = Number(picked);
    await musicSidWaitLineCountMemory.update(memoryKey, value);
    return [fileName, value];
}

/**
 * Prompts for a music source file: every one found in the open workspace,
 * plus a "Browse..." entry - via the shared `pickWorkspaceFile` helper.
 * Needed for the Command Palette form of {@link playMusic}/
 * {@link buildMusicDsk}: unlike most other Palette commands, there is no
 * "active file" fallback to reach for here - a music project file is
 * binary/AT3-native, never something opened as a text editor tab, so there
 * is no "currently edited file" for these commands to mean.
 *
 * The extension glob is a static mirror of `MusicConfig::song_extensions`'s
 * default (`aks`/`sks`/`128`/`vt2`/`wyz`) - kept manually in sync, same
 * constraint as the `explorer/context` menu's `when` regex in package.json.
 */
function pickMusicFile(): Promise<string | undefined> {
    return pickWorkspaceFile({
        glob: '**/*.{aks,sks,128,vt2,wyz,AKS,SKS,VT2,WYZ}',
        browseLabel: '$(folder-opened) Browse for a music file...',
        placeHolder: 'Which music file?',
        dialogFilters: { 'Arkos Tracker song': ['aks', 'sks', '128', 'vt2', 'wyz'] },
        dialogOpenLabel: 'Select',
    });
}

/**
 * "▶ Play music in emulator" - converts an Arkos Tracker source song into a
 * standalone player and launches it (AKG, or a dedicated SID player - see
 * {@link musicCommandArgs}). Unlike `cpclib.runBasic`/`cpclib.runAssembly`
 * (CodeLens-only, invoked with an explicit `arguments: [path]` VS Code never
 * touches), this is a file-browser context-menu / command-palette entry: VS
 * Code hands it a `vscode.Uri`, not a string, and that argument shape is
 * exactly why this can't just be one of the `executeCommandProvider`-
 * advertised names bridged automatically - the server command this forwards
 * to (`cpclib.musicPlay`) is deliberately a different name, same reason as
 * the peephole commands.
 *
 * With no `target` (the Command Palette case - a context-menu invocation
 * always supplies one), {@link pickMusicFile} asks which file, since there
 * is no "active file" to fall back to for a binary music project file.
 *
 * The server reports the outcome itself (`show_message`/`log_message`), so
 * there is nothing to do here with the response.
 */
async function playMusic(target: string | vscode.Uri | undefined): Promise<void> {
    let fileName = target instanceof vscode.Uri ? target.fsPath : target;
    if (!fileName) {
        fileName = await pickMusicFile();
        if (!fileName) { return; }
    }
    const args = await musicCommandArgs(fileName);
    if (!args) { return; }
    await client.sendRequest('workspace/executeCommand', {
        command: 'cpclib.musicPlay',
        arguments: args,
    });
}

/**
 * "💿 Build DSK with music" - same conversion as {@link playMusic}, but only
 * builds a DSK (saved next to the source song, server-side) instead of
 * launching an emulator. See {@link playMusic}'s doc comment for why this
 * is registered here rather than bridged automatically, and why a missing
 * `target` (Command Palette) prompts via {@link pickMusicFile}.
 */
async function buildMusicDsk(target: string | vscode.Uri | undefined): Promise<void> {
    let fileName = target instanceof vscode.Uri ? target.fsPath : target;
    if (!fileName) {
        fileName = await pickMusicFile();
        if (!fileName) { return; }
    }
    const args = await musicCommandArgs(fileName);
    if (!args) { return; }
    await client.sendRequest('workspace/executeCommand', {
        command: 'cpclib.musicBuildDsk',
        arguments: args,
    });
}

export function registerMusic(context: ExtensionContext): void {
    musicSidWaitLineCountMemory = context.globalState;
    context.subscriptions.push(
        vscode.commands.registerCommand('cpclib.playMusic', playMusic),
        vscode.commands.registerCommand('cpclib.buildMusicDsk', buildMusicDsk),
    );
}
