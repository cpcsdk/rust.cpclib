; Minimal CHIPNSFX ("CHP") player harness - plays a song written with Cesar
; Nicolas-Gonzalez' (CNGSOFT) CHIPNSFX tracker, once per VBL, with the same
; look as `music_akg_harness.asm`'s. `{{...}}` placeholders are substituted
; with real, already string-escaped paths by `music_run.rs` before this is
; assembled (see the AKG harness for why they are not `-D` symbols):
;   MUSIC_DATA_FNAME    - the Z80 source `chipnsfx <song>.chp <out>` generates
;   PLAYER_SOURCE_FNAME - CHIPNSFX.I80, from the installed CHIPNSFX
;   MUSIC_EXEC_FNAME    - where the assembled (headerless) binary is saved
;   INFO_PRINT_CODE     - music_info_print.asm, prints InfoText in mode 2
;   INFO_TEXT           - the song's title/comment as `db` rows
;
; Adapted from players/chp/chp.asm in cpcsdk/amstrad_cpc_players_comparison.
    org 0x500

    ; Load address == entry point, like the AKG harness: `music_run.rs` builds
    ; the AMSDOS header itself (load=exec=0x500).
    jp Start

; +4 SONG_ONLY (no sound effects logic), +256 PREBUILT (the player carries
; its own, ready-made work area - no init call needed)
CHIPNSFX_FLAG = 4+256

CHP_File
    include "{{MUSIC_DATA_FNAME}}"

chip_song_a equ song_a
chip_song_b equ song_b
chip_song_c equ song_c

chipnsfx
PlayerStart
    include "{{PLAYER_SOURCE_FNAME}}"
PlayerEnd

    run $
Start
    di
    ld sp, 0x500
    ld hl, #c9fb : ld (#38), hl        ; reduced interrupt handler (ei/ret)

    call InfoShow                       ; title/comment, see below

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
    call chip_play
    ei
    ld bc, 0x7f54 : out (c), c

    jr MainLoop

; Required by the player: A=value, C=register
writepsg
    push bc
    ld b, #f4
    out (c), c
    ld bc, #f6c0
    out (c), c
    dw #71ed                            ; out (c),0
    ld b, #f4
    out (c), a
    ld bc, #f680
    out (c), c
    dw #71ed                            ; out (c),0
    pop bc
    ret

{{INFO_PRINT_CODE}}

    ; Song info text, then the (unsaved) glyph buffer right after the binary.
InfoText
{{INFO_TEXT}}
FontBuf equ $

    ; Headerless - `music_run.rs` wraps this in a proper AMSDOS header itself.
    save "{{MUSIC_EXEC_FNAME}}", 0x500, $-0x500
