; Minimal Arkos Tracker player harness - loads the converted song data and
; one of Arkos Tracker's own player routines (AKG, AKM, AKY stabilized or
; unstabilized: they share this shape, and differ only in the few
; placeholders below), then loops calling the player once per VBL. `{{...}}` placeholders are substituted with real, already
; shell/string-escaped paths by `music_run.rs` before this is assembled
; (basm's `include`/`incbin` need a literal quoted string - a `-D`-defined
; symbol used bare, e.g. `incbin MUSIC_DATA_FNAME`, is *not* dereferenced as
; one; it is read as that literal text as a filename, which is what the
; original design this was adapted from actually relied on a different
; assembler behavor for):
;   MUSIC_DATA_FNAME    - the song `SongToAkg`/`SongToAkm`/`SongToAky` wrote
;   PLAYER_CONFIG_FNAME - the `..._playerconfig.asm` AT3 exports alongside it
;   PLAYER_SOURCE_FNAME - the player, from the installed Arkos Tracker 3
;   PLAYER_DEFINES      - the player's own `NAME = value` switches
;   PLAYER_INIT_PRE     - what its init routine wants in registers besides HL
;                         (the subsong number in A, or nothing)
;   PLAYER_INIT / PLAYER_PLAY - the init and play routines to call
;   MUSIC_EXEC_FNAME    - where the assembled AMSDOS binary is saved
;
;   INFO_PRINT_CODE   - music_info_print.asm, prints InfoText in mode 2
;   INFO_TEXT          - the song's title/author/comment as `db` rows
;
; Adapted from the AKM/AKG harnesses in cpcsdk/amstrad_cpc_players_comparison
; (players/{akg,akm,akys,akyu}/*.asm) - the player source is pulled from the real AT3
; install (PLAYER_SOURCE_FNAME) rather than from a locally vendored copy.
    org 0x500
    include "{{PLAYER_CONFIG_FNAME}}"

    ; Reserves the 6 bytes between 0x500 and 0x506 - `SongToAkg -adr 0x506`
    ; encoded the song data to load at 0x506, so MusicFile must land exactly
    ; there. The reference harness fills this gap with two profiler entry
    ; points (`jp profiler_init` / `jp profiler_run`); this build instead
    ; puts a `jp Start` right at 0x500, so the load address doubles as the
    ; entry point - `music_run.rs` builds the AMSDOS binary's header itself
    ; (`AmsdosFile::binary_file_from_buffer`, load=exec=0x500) rather than
    ; asking basm's own SAVE directive for an AMSDOS header directly to the
    ; host filesystem, which real testing found silently writes nothing for
    ; this basm version (SAVE straight into a .dsk, or a headerless SAVE,
    ; both work - just not SAVE ..., AMSDOS to a bare host path).
    jp Start
    defs 3

{{PLAYER_DEFINES}}
MusicFile
    assert $ == 0x506
    incbin "{{MUSIC_DATA_FNAME}}"

    run $
Start
    di
    ld sp, 0x500
    ld hl, #c9fb : ld (#38), hl        ; reduced interrupt handler (ei/ret)

    call InfoShow                       ; title/author/comment, see below

    ld hl, MusicFile
    {{PLAYER_INIT_PRE}}
    call {{PLAYER_INIT}}
    ei
MainLoop
    ld b, #f5                          ; PPI port B
WaitVsync
    in a, (c)
    rra
    jr nc, WaitVsync
    halt                                ; a bit of slack past the VBL edge
    halt

    ld bc, 0x7f10 : out (c), c
    ld bc, 0x7f4b : out (c), c
    di
    call {{PLAYER_PLAY}}
    ei
    ld bc, 0x7f54 : out (c), c

    jr MainLoop

{{INFO_PRINT_CODE}}

    include "{{PLAYER_SOURCE_FNAME}}"

    ; Song info text, then the (unsaved) glyph buffer right after the binary.
InfoText
{{INFO_TEXT}}
FontBuf equ $

    ; Headerless (no AMSDOS header, see the note above the `jp Start` gap) -
    ; `music_run.rs` wraps this in a proper AMSDOS header itself.
    save "{{MUSIC_EXEC_FNAME}}", 0x500, $-0x500
