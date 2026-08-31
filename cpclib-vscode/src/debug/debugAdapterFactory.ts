import * as vscode from 'vscode';
import { DEBUG_TYPE } from './launch';

/** Registers the `cpclib-dap` adapter descriptor factory - how VS Code
 * learns which binary to spawn for a `basm`-typed debug session. */
export function registerDebugAdapterFactory(
    context: vscode.ExtensionContext,
    resolveAdapterPath: () => string,
): void {
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory(DEBUG_TYPE, {
            createDebugAdapterDescriptor() {
                // Logging is configured in `cpclib-lsp.toml` (`[dap] log`), not
                // here: a session is started from a CodeLens, the palette, F5 or
                // a launch.json, and a setting that must be repeated in each of
                // them is one nobody turns on when they actually need it.
                return new vscode.DebugAdapterExecutable(resolveAdapterPath(), ['dap'], {
                    // The adapter reads the project's configuration from here.
                    cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
                });
            },
        }),
    );
}

/** F5 on a .asm file with no launch.json at all: synthesise the obvious
 * configuration rather than making the user write one. */
export function registerDebugConfigurationProvider(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.debug.registerDebugConfigurationProvider(DEBUG_TYPE, {
            resolveDebugConfiguration(_folder, config) {
                if (!config.type && !config.request && !config.name) {
                    const editor = vscode.window.activeTextEditor;
                    const languageId = editor?.document.languageId;
                    if (editor && (languageId === 'basm' || languageId === 'locomotive-basic')) {
                        return {
                            type: DEBUG_TYPE,
                            request: 'launch',
                            name: `Debug this ${languageId === 'basm' ? '.asm' : '.bas'} file`,
                            program: editor.document.fileName,
                        };
                    }
                }
                return config;
            },
        }),
    );
}
