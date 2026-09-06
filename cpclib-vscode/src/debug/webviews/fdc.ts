import * as vscode from 'vscode';
import { escapeHtml } from '../../shared/html';
import { FdcDrive, FdcDump } from '../types';
import { showSimpleChipView, SimpleChipViewKind } from './simpleChipView';

const KIND: SimpleChipViewKind = { viewType: 'cpclib.fdc', title: 'FDC', command: '-fdcview' };

/**
 * The FDC registers, plus a real per-track sector table when the backend
 * sends one (`drives`, SugarBox only - live-tested against a real v2.1.1
 * instance; AmspiritLite has no equivalent endpoint). See
 * `simple_chip_view_answer`'s own doc comment (`cpclib-dap/src/session.rs`)
 * for the real field names (`c`/`h`/`r`/`n`, not the "track/side/sector"
 * shape `EMULATOR_INTERFACE.md` documents but the real answer does not use).
 * Opened with `-fdcview`.
 */
export function showFdc(session: vscode.DebugSession, dump: FdcDump | undefined): void {
    showSimpleChipView(KIND, session, dump, drivesHtml(dump?.drives));
}

function drivesHtml(drives: FdcDrive[] | undefined): string {
    if (!drives || drives.length === 0) { return ''; }

    return drives
        .map((drive, index) => {
            const letter = String.fromCharCode(65 + index);
            if (!drive.present) {
                return `<h3>Drive ${letter}</h3><p>No disk.</p>`;
            }

            const summary = `Track ${drive.track}, side ${drive.side} — ` +
                `${drive.nbTracks} track(s), ${drive.nbSides} side(s)${drive.writeProtected ? ', write-protected' : ''}` +
                (drive.path ? ` — ${escapeHtml(drive.path)}` : '');

            const rows = drive.sectors.map(sector => {
                const bad = !sector.hdrCrc || !sector.dataCrc;
                return `<tr>` +
                    `<td>${sector.c}</td><td>${sector.h}</td><td>${sector.r}</td><td>${sector.n}</td>` +
                    `<td>${sector.size}</td>` +
                    `<td class="${sector.hdrCrc ? '' : 'bad'}">${sector.hdrCrc ? 'OK' : 'BAD'}</td>` +
                    `<td class="${sector.dataCrc ? '' : 'bad'}">${sector.dataCrc ? 'OK' : 'BAD'}</td>` +
                    `<td>${sector.deleted ? 'yes' : ''}</td>` +
                    `<td class="${bad ? 'bad' : ''}">${sector.st1.toString(16).padStart(2, '0').toUpperCase()}</td>` +
                    `<td class="${bad ? 'bad' : ''}">${sector.st2.toString(16).padStart(2, '0').toUpperCase()}</td>` +
                    `</tr>`;
            }).join('');

            const table = drive.sectors.length
                ? `<table><thead><tr>` +
                  `<th>C</th><th>H</th><th>R</th><th>N</th><th>Size</th>` +
                  `<th>Hdr CRC</th><th>Data CRC</th><th>Deleted</th><th>ST1</th><th>ST2</th>` +
                  `</tr></thead><tbody>${rows}</tbody></table>`
                : '<p>No sectors on this track.</p>';

            return `<h3>Drive ${letter}</h3><p>${summary}</p>${table}`;
        })
        .join('');
}
