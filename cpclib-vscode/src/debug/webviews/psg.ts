import * as vscode from 'vscode';
import { PsgDump } from '../types';
import { showSimpleChipView, SimpleChipViewKind } from './simpleChipView';

const KIND: SimpleChipViewKind = { viewType: 'cpclib.psg', title: 'PSG', command: '-psgview' };

/** The PSG (AY-3-8912) registers - see `showSimpleChipView`'s own doc
 * comment for why they are shown unstructured rather than decoded into
 * tone Hz/envelope shape names. Opened with `-psgview`. */
export function showPsg(session: vscode.DebugSession, dump: PsgDump | undefined): void {
    showSimpleChipView(KIND, session, dump);
}
