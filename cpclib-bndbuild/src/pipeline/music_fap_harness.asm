; Minimal FAP (Fast AY Player) harness - plays a `.fap` file (an AY/YM song
; packed by the `fap` cruncher), once per VBL, with the same look as
; `music_akg_harness.asm`'s. `{{...}}` placeholders are substituted by
; `music_run.rs` before this is assembled:
;   MUSIC_DATA_FNAME    - the .fap file
;   FAP_INIT_PATH       - fap-init.bin, FAP's player initialisation code
;   FAP_PLAY_PATH       - fap-play.bin, FAP's replay routine
;   MUSIC_BUFF_SIZE     - the decrunch buffer size the cruncher reported
;   MUSIC_EXEC_FNAME    - where the assembled (headerless) binary is saved
;   INFO_PRINT_CODE / INFO_TEXT - the song-info screen, see music_info_print.asm
;
; Adapted from players/fap/fap.asm in cpcsdk/amstrad_cpc_players_comparison.
    org 0x500

    ; Load address == entry point (see music_akg_harness.asm).
    jp Start

Start
    di
    ld sp, 0x500
    ld hl, #c9fb : ld (#38), hl        ; reduced interrupt handler (ei/ret)

    call InfoShow                       ; title/author/comment, see below

    ; Initialize the player. Once done, the init code could be overwritten.
    ld a, hi(FapBuff)                   ; High byte of the decrunch buffer
    ld bc, FapPlay                      ; Address of the player binary
    ld de, ReturnAddr                   ; Where to jump after a song frame
    ld hl, FapData                      ; Address of song data
    di
    call FapInit
    ei

MainLoop
    ld b, #f5
    in a, (c)
    rra
    jr nc, MainLoop

    halt                                ; Wait to make sure the VBL is over.
    halt

    di                                  ; Prevent interrupt apocalypse
    ld (RestoreSp), sp                  ; Save our precious stack-pointer

    ld bc, #7f10 : out (c), c
    ld a, #4c
    out (c), a                          ; border: red

    jp FapPlay                          ; Jump into the replay-routine

ReturnAddr                              ; the replay-routine jumps back here
RestoreSp = $+1
    ld sp, 0

    ld bc, #7f54 : out (c), c
    ei

    jp MainLoop

{{INFO_PRINT_CODE}}

FapInit
    incbin "{{FAP_INIT_PATH}}"
FapPlay
    incbin "{{FAP_PLAY_PATH}}"
FapData
    incbin "{{MUSIC_DATA_FNAME}}"

    ; Song info text - the last thing saved.
InfoText
{{INFO_TEXT}}

    ; Not saved: the decrunch buffer (page aligned), then the font buffer.
FapBuff equ ($ + 255) & #ff00
FontBuf equ FapBuff + {{MUSIC_BUFF_SIZE}}

    ; Headerless - `music_run.rs` wraps this in a proper AMSDOS header itself.
    save "{{MUSIC_EXEC_FNAME}}", 0x500, $-0x500
