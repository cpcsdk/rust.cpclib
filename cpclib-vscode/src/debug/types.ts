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
