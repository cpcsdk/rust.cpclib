; Minimal MinYMiser ("YMP" Z80 port) harness - plays a `.miny` file (an AY/YM
; song packed by `miny quick`, YM3 input), once per VBL, with the same look as
; `music_akg_harness.asm`'s. `{{...}}` placeholders are substituted by
; `music_run.rs` before this is assembled:
;   MUSIC_DATA_FNAME    - the .miny file
;   YMP_FNAME           - ymp_z80.z80, the Z80 player (embedded in bndbuild)
;   MUSIC_BUFF_SIZE     - the cache size `miny quick` reported
;   MUSIC_EXEC_FNAME    - where the assembled (headerless) binary is saved
;   INFO_PRINT_CODE / INFO_TEXT - the song-info screen, see music_info_print.asm
;
; Adapted from players/miniq/miniq.asm in cpcsdk/amstrad_cpc_players_comparison.
    org 0x500

    ; Load address == entry point (see music_akg_harness.asm).
    jp Start
    defs 3

    assert $ == 0x506
MINIQ_FILE
    incbin "{{MUSIC_DATA_FNAME}}"

    read "{{YMP_FNAME}}"

    run $
Start
    di
    ld sp, 0x500
    ld hl, #c9fb : ld (#38), hl        ; reduced interrupt handler (ei/ret)

    call InfoShow                       ; title/author/comment, see below

    ld hl, MINIQ_FILE                   ; the packed tune
    ld de, player_cache                 ; the player's writable cache
    call ymp_player_init
    ei

MainLoop
    ld b, #f5                           ; PPI port B
WaitVsync
    in a, (c)
    rra
    jr nc, WaitVsync
    halt                                ; a bit of slack past the VBL edge
    halt

    ld bc, #7f10 : out (c), c
    ld a, #4c
    out (c), a                          ; border: red
    di
    call ymp_player_update
    ei
    ld bc, #7f54 : out (c), c

    jr MainLoop

{{INFO_PRINT_CODE}}

    ; Song info text - the last thing saved.
InfoText
{{INFO_TEXT}}

    ; Not saved: the player's state, its cache, then the font buffer.
player_state equ $
player_cache equ player_state + ymp_size
FontBuf equ player_cache + {{MUSIC_BUFF_SIZE}}

    ; Headerless - `music_run.rs` wraps this in a proper AMSDOS header itself.
    save "{{MUSIC_EXEC_FNAME}}", 0x500, $-0x500
