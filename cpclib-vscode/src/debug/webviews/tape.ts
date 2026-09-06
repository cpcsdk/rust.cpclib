import * as vscode from 'vscode';
import { TapeDump } from '../types';
import { showSimpleChipView, SimpleChipViewKind } from './simpleChipView';

const KIND: SimpleChipViewKind = { viewType: 'cpclib.tape', title: 'Tape', command: '-tapeview' };

/** The cassette transport - see `tape_pane`'s own doc comment
 * (`cpclib-dap/src/amspiritlite.rs`) for why `blocks` is shown only as a
 * count, not a decoded per-block table. Only ever answers on a SugarBox
 * session - AmspiritLite has no tape endpoint at all (confirmed live).
 * Opened with `-tapeview`. */
export function showTape(session: vscode.DebugSession, dump: TapeDump | undefined): void {
    showSimpleChipView(KIND, session, dump);
}
