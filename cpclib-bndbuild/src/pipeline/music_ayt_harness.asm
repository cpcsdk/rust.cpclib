; Minimal AYT harness - plays an `.ayt` file (an AY/YM song packed by Logon
; System's `ym2ayt`), once per VBL, with the same look as
; `music_akg_harness.asm`'s. AYT does not ship a player but a *builder*: a
; routine that writes a player, specialised for the song, at run time.
; `{{...}}` placeholders are substituted by `music_run.rs` before assembling:
;   MUSIC_DATA_FNAME    - the .ayt file
;   AYT_BUILDER_FNAME   - AytPlayerBuilder-CPC.asm (embedded in bndbuild)
;   MUSIC_EXEC_FNAME    - where the assembled (headerless) binary is saved
;   INFO_PRINT_CODE / INFO_TEXT - the song-info screen, see music_info_print.asm
;
; The builder is written for a case-insensitive assembler (basm is run with
; `--case-insensitive` for this harness).
;
; Adapted from players/ayt/ayt.asm in cpcsdk/amstrad_cpc_players_comparison.
AYT_Player equ #100         ; where the builder writes the player (247..317 bytes,
                            ; +57 when the song has fewer than 14 registers)
MyStack equ AYT_Player

    org 0x500

    ; Load address == entry point (see music_akg_harness.asm).
    jp Start

PlayerStart
AYT_Builder
    read "{{AYT_BUILDER_FNAME}}"
PlayerEnd

    run $
Start
    di
    ld sp, MyStack
    ld hl, #c9fb : ld (#38), hl        ; reduced interrupt handler (ei/ret)

    call InfoShow                       ; title/author/comment, see below

    ; Build the player for this song.
    ld ix, AYT_File                     ; the AYT file
    ld de, AYT_Player                   ; where the player is built
    ld a, 1                             ; number of loops of the music
    if PlayerAccessByJP
    ld hl, AYT_Player_Ret               ; where the player comes back
    endif
    call AYT_Builder
    if PlayerAccessByJP
    ld (AYT_Player_ReloadSP), sp        ; the player trashes SP
    endif
    ei                                  ; the builder did a `di`

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

    if PlayerAccessByJP
    jp AYT_Player
AYT_Player_Ret
AYT_Player_ReloadSP equ $+1
    ld sp, 0
    else
    call AYT_Player
    endif

    ld bc, #7f54 : out (c), c

    jr MainLoop

{{INFO_PRINT_CODE}}

AYT_File
    incbin "{{MUSIC_DATA_FNAME}}"

    ; Song info text, then the (unsaved) glyph buffer right after the binary.
InfoText
{{INFO_TEXT}}
FontBuf equ $

    ; Headerless - `music_run.rs` wraps this in a proper AMSDOS header itself.
    save "{{MUSIC_EXEC_FNAME}}", 0x500, $-0x500
