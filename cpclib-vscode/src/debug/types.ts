// Shared data shapes for the debug session's custom events (`cpclib/*View`,
// `cpclib/stoppedAt`, `cpclib/stoppedInstruction`) - kept in one place since
// several webview/document modules and the event dispatcher in
// `debug/register.ts` all need the same shapes.

/** Where the program stopped, as the adapter reports it. */
export interface StopLocation {
    path?: string;
    line?: number;
    column?: number;
    endColumn?: number;
    /**
     * The instruction the machine really holds there, when it is not what the
     * line says - `ld a,0x01` for a line reading `ld a,ANIMATION_STATE_FINISHED`.
     * Absent when the source already spells it out, and there is nothing to
     * disambiguate.
     *
     * Decoded from the *emulator's* memory, so it is also the answer for an
     * instruction that has modified itself, for one written instruction that
     * became several real ones, and for a line reading `defs` whose code was
     * generated at run time. It therefore arrives a round trip after the stop,
     * in `cpclib/stoppedInstruction`, and is null on the `cpclib/stoppedAt`
     * that revealed the line - except where there was no emulator to ask, when
     * the assembled image answers immediately instead.
     */
    instruction?: string | null;
}

export interface MemoryDump {
    viewId?: string;
    /** `-mv all,follow`'s own views all carry the same group name - the
     * editor renders them together in one panel instead of one apiece. */
    group?: string | null;
    /** A person typed this, rather than a stop silently refreshing an
     * already-open panel. */
    requested?: boolean;
    address: number;
    label?: string | null;
    bytes: number[];
    marks?: { offset: number; name: string }[];
    changed?: number[];
    /** The RAM configuration (0-7, "C0"-"C7") these bytes were read under,
     * if an override was in effect - `null`/absent means the CPU's own
     * live view (the default). See `-mv`'s own `[config]` argument. */
    config?: number | null;
    /** The explicit extended-RAM page these bytes were read under, if one
     * was given (`config`'s own `mode:page` form) - `null`/absent means
     * whichever page was live at the time. */
    page?: number | null;
}

export interface ScreenDump {
    png: string;
    address: number;
    width: number;
    height: number;
    mode: number;
    bytes: string;
    charRowHeight: number;
    palette: string[];
    hardwarePalette: string[];
    encoding: number;
    /** The RAM configuration (0-7, "C0"-"C7") this frame was read under, if
     * an override was in effect - `null`/absent means the CPU's own live
     * view (the default). See `-sv`'s own `[config]` argument. */
    config?: number | null;
    /** The explicit extended-RAM page this frame was read under, if one
     * was given (`config`'s own `mode:page` form) - `null`/absent means
     * whichever page was live at the time. */
    page?: number | null;
}

export interface BasicListingDump {
    text: string;
}

export interface CrtcRegister {
    name: string;
    value: number;
}

export interface CrtcWarning {
    registers: string[];
    severity: 'error' | 'warning';
    message: string;
}

export interface CrtcDump {
    registers: CrtcRegister[];
    warnings: CrtcWarning[];
}

/** One PSG value, already decoded and formatted server-side
 * (`amspiritlite::psg_pane`, `cpclib-dap/src/amspiritlite.rs`) - per-channel
 * tone/volume, mixer, noise, envelope, shown under whichever field name the
 * emulator itself used (SugarBox's `chanAFreq`-style or AmspiritLite's
 * `period_a`-style), plus the raw `R0`-`R15` registers when the backend
 * sends them (SugarBox only). Not converted to Hz - see `psg_pane`'s own
 * doc comment for why. */
export interface PsgRegister {
    name: string;
    value: string;
}

export interface PsgDump {
    registers: PsgRegister[];
}

/** Ports A/B/C and the control byte, already normalised server-side
 * (`ppi_pane`, `cpclib-dap/src/amspiritlite.rs`) to the same names
 * regardless of which backend answered. Only ever real for SugarBox -
 * AmspiritLite has no PPI endpoint (confirmed live: `/api/ppi` is 404). */
export interface PpiRegister {
    name: string;
    value: string;
}

export interface PpiDump {
    registers: PpiRegister[];
}

/** The cassette transport - decoded server-side (`tape_pane`,
 * `cpclib-dap/src/amspiritlite.rs`) from SugarBox's real `getTapeState`.
 * `blocks` is shown only as a count: its own per-element shape was never
 * confirmed live (a synthetic test tape was not accepted by the emulator),
 * so nothing here decodes individual TZX/CDT blocks. Only ever real for
 * SugarBox - AmspiritLite has no tape endpoint (confirmed live: `/api/tape`
 * is 404). */
export interface TapeRegister {
    name: string;
    value: string;
}

export interface TapeDump {
    registers: TapeRegister[];
}

/** A drive's own status - `getFdcState`'s real per-drive shape
 * (`cpclib-dap/src/session.rs`'s `simple_chip_view_answer`), live-tested
 * against a real SugarBox v2.1.1, not `EMULATOR_INTERFACE.md`'s docs (which
 * differ). AmspiritLite has no equivalent endpoint, so `drives` is only
 * ever present for a SugarBox session. */
export interface FdcSector {
    /** Cylinder, head, record (sector) and size-code, read from the
     * sector's own ID field - real FDC terminology, not a made-up
     * `track`/`side`/`sector` renaming. */
    c: number;
    h: number;
    r: number;
    n: number;
    size: number;
    deleted: boolean;
    hdrCrc: boolean;
    dataCrc: boolean;
    st1: number;
    st2: number;
}

export interface FdcDrive {
    present: boolean;
    track: number;
    side: number;
    trackSize: number;
    nbTracks: number;
    nbSides: number;
    writeProtected: boolean;
    path: string;
    sectors: FdcSector[];
}

/** `registers` is the generic `{name, value}` fallback (drive/status
 * summary, or whatever a live peer's own answer has); `drives`, present
 * only for a SugarBox session, is the real per-track sector table. */
export interface FdcDump {
    registers: FdcRegister[];
    drives?: FdcDrive[];
}

export interface FdcRegister {
    name: string;
    value: string;
}

export interface DisassembledInstruction {
    address: string;
    instruction: string;
    instructionBytes?: string;
    symbol?: string;
    line?: number;
    column?: number;
    endColumn?: number;
    /** Labels the addresses in this instruction's operands stand for. */
    symbols?: string[];
    /**
     * Other labels that share this row's own address with `symbol`. No
     * source line names a heading the way a call names its target, so there
     * is no evidence to pick between them - shown rather than guessed at.
     */
    symbolAlternatives?: string[];
    location?: { name?: string; path?: string };
    /** A trailing `;` comment on this instruction's own source line. */
    comment?: string;
    /** The comment/blank-line block immediately above this instruction's
     * source line, in source order - hand-written narration for what
     * follows, not anything computed. */
    precedingComments?: string[];
}

export interface Disassembly {
    address: number;
    label?: string | null;
    instructions: DisassembledInstruction[];
    /** Where the program actually is, so the row can be marked. */
    pc?: number | null;
    /** Whether this view moves with the program on every step. */
    followsPc?: boolean;
    /** The RAM configuration (0-7, "C0"-"C7") this read was made under, if
     * an override was in effect - `null`/absent means the CPU's own live
     * view (the default). See `-dv`'s own `[config]` argument. */
    config?: number | null;
    /** The explicit extended-RAM page this read was made under, if one was
     * given (`config`'s own `mode:page` form) - `null`/absent means
     * whichever page was live at the time. */
    page?: number | null;
}
