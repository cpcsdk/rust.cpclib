import * as vscode from 'vscode';
import { PpiDump } from '../types';
import { showSimpleChipView, SimpleChipViewKind } from './simpleChipView';

const KIND: SimpleChipViewKind = { viewType: 'cpclib.ppi', title: 'PPI', command: '-ppiview' };

/** The PPI (8255) ports A/B/C and the control byte - see `ppi_pane`'s own
 * doc comment (`cpclib-dap/src/amspiritlite.rs`) for why no per-bit
 * decoding (Group A/B mode, VSYNC/keyboard-row bits) is attempted yet.
 * Only ever shows real data on a SugarBox session - AmspiritLite has no
 * PPI endpoint at all (confirmed live). Opened with `-ppiview`. */
export function showPpi(session: vscode.DebugSession, dump: PpiDump | undefined): void {
    showSimpleChipView(KIND, session, dump);
}
