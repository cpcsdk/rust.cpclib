        ;SID player harness - ported, deliberately close to verbatim, from
        ;Arkos Tracker 3's own official example,
        ;players/playerAky/sources/z80/tester/PlayerAkySidTester_CPC.asm.
        ;The SID engine is cycle-exact (a hard 64-NOP-per-scanline grid) and
        ;has essentially zero tolerance for improvisation, so this is a port,
        ;not a fresh design - see music_run.rs's doc comments for exactly
        ;what was kept/changed and why. Requires Rasm >= 3.0.8 (COUNTNOPS,
        ;ASSERT, and local-label macro substitution used throughout the
        ;player/macros are rasm-only - basm implements none of them).
        ;
        ;{{PLACEHOLDER}}s substituted by music_run.rs before assembling:
        ;   MUSIC_DATA_FNAME    - SongToAky *source*-mode .asm export
        ;   PLAYER_SOURCE_FNAME - PlayerAkySid_CPC.asm, from the installed AT3
        ;   PLAYER_MACROS_FNAME - PlayerAkySidMacros_CPC.asm, same install
        ;   WAIT_LINE_COUNT     - user-configurable safety margin (default 72,
        ;                         see MusicConfig::sid_wait_line_count) - too
        ;                         low for a given song and this freezes.
        ;   BUILDSNA_DIRECTIVES - either "buildsna"/"bankset 0" (play path) or
        ;                         empty (DSK-build path) - real testing found
        ;                         `buildsna` and `-ob` (rasm's plain-binary
        ;                         output flag) are mutually exclusive: once
        ;                         `buildsna` is present, ALL output goes
        ;                         through the snapshot mechanism and `-ob` is
        ;                         silently ignored.
        ;
        ;Unlike the AKG harness, output is driven purely by rasm's own CLI
        ;flags (-oi for a snapshot, -ob for a plain binary), not an in-source
        ;SAVE directive - deliberately avoids re-risking the kind of
        ;assembler-specific SAVE bug the AKG harness had to work around, and
        ;`Start` already lands exactly at `org`'s address (0x100) with no gap
        ;to reserve, so load address == exec address needs no extra trick for
        ;the DSK-build case either.

{{BUILDSNA_DIRECTIVES}}

        include "{{PLAYER_MACROS_FNAME}}"

PLY_AKYsid_RET_TABLES   equ #a000               ;Can be put anywhere. See PLY_AKYsid_RET_TABLES_END in the player about where it ends.

;Do you want a keyboard test? It takes too long and messes up with the PSG, so it *will* mess up with the SID timings!
;But since it is only done once per frame, it should not be heard.
USE_KEYBOARD_TEST equ 0

PLY_AKY_SID_ADD_STOP_SOUNDS equ 1

        org #100
        run $
        limit PLY_AKYsid_RET_TABLES
Start   equ $

        di

        ld hl,#c9fb
        ld (#38),hl
        ld sp,$

        ld bc,#bc01
        out (c),c
        ld bc,#bd00
        out (c),c

        ;Initializes the music.
        ld hl,Music_Start
        ld de,PlayReturn
        call PLY_AKYsid_Init

        ;First accurate synchronization, we will then bypass all VSync, everything is cycle-accurate!
Sync    ld b,#f5
        in a,(c)
        rra
        jr nc,Sync + 2
Sync2
        in a,(c)
        rra
        jr c,Sync2

        ;Waits. Change this according to where you want to have your effect visible.
        ei
        nop
        halt
        halt
        halt
        halt
        halt
        ld bc,270
.wait   dec bc
        ld a,b
        or c
        jr nz,.wait

        di

        ld bc,#7f10
        out (c),c
        ld a,#4b
        out (c),a

        ;Calls the player, using JP, NOT call!
        ld (SaveSp + 1),sp

                ld sp,PLY_AKYsid_RET_TABLES
                ld hl,0 * 256 + #f6                  ;H' is the line counter.
        exx
        jp PLY_AKYsid_Play
PlayReturn
        ;Nops: 3.

        ;The player has finished. Here you can perform your effects while still being perfectly synced (good luck!).
        ;The trick is to call the SID code every 36 nops *at worst*.
        ;Please read CAREFULLY the notes in the player. Doing SID is very tricky.

        ;H' contains how many lines were accounted for during the player.
        ;Now waits till we have reached a certain line, this is useful if you want to synchronize an effect,
        ;but you may only want to reach 312 (the end of the frame) and loop again, if you've got nothing else to do.
        ;But as an example, we'll wait till the maximum player duration, so that we can synchronize an effect
        ;(this is what I did in the "Signal In Disarray" demo).
        exx
            ld a,h
        exx
        ld e,a
        ld d,0
;This is the maximum the player can reach... with a safety margin. Other songs may take longer, or shorter.
;Decrease this as much as you can, but if the program freezes, it means it is too low!
WAIT_LINE_COUNT = {{WAIT_LINE_COUNT}}
        ld hl,WAIT_LINE_COUNT
        or a
        sbc hl,de

        ;This macro waits for a specific NOP count. Write here how many NOPs were *spent*,
        ;and it will wait for the right amount before calling the SID code.
        ;After this, your NOP count is 0.
        ;You can use the COUNTNOPS mnemonic of Rasm to help you count how many cycles are spent
        ;(check the numerous use in the player).
        AKY_SID_WAIT_AND_CALL_TIMER_CODE (17)

StabilizationLoop
        dec hl
        ld a,l
        or h
        jr z,WaitEndOfFrame

        ld bc,#7f55
        out (c),c
        ld c,#4c
        out (c),c

        ;This macro waits to reach a specific NOP count. Write here how many NOPs were *spent*,
        ;and it will wait for the right amount before calling the SID code.
        ;THEN, it will jump to the indicated address, where you know your NOP count is 0.
        AKY_SID_WAIT_AND_CALL_TIMER_CODE_AND_JUMP_TO StabilizationLoop, (6 + 13)

        ;Now waits for the end of the frame.
        ;You could also manage your line-accurate effect here.
WaitEndOfFrame
                ;Nops: 7

        IF USE_KEYBOARD_TEST == 1
                ;The keyboard test is very bothersome because it breaks our sync, we cannot
                ;interrupt this code.
                ld a,5 + 64
                ld bc,#f782
                out (c),c
                ld bc,#f40e
                out (c),c
                ld bc,#f6c0
                out (c),c
                out (c),0
                ld bc,#f792
                out (c),c
                dec b
                out (c),a
                ld b,#f4
                in a,(c)
                ld bc,#f782
                out (c),c
                dec b
                out (c),0
                rla
                jr nc,KeyPressed
                        ;Nops: 67.
                ;Goes back to the possibly selected SID register.
                ld a,(PLY_AKYsid_SidPsgRegister)
                ld b,#f4
                out (c),a       ;f400 + register.
                ld bc,#f6c0
                out (c),c       ;f6c0
                out (c),0       ;f600.
                        ;Nops: 88.

                ld iy,$ + 5     ;We're already wayyy too late, but let's play 3 SIDs to compensate.
                ret
                ld iy,$ + 5
                ret
                ld hl,312 - 2 - WAIT_LINE_COUNT
                AKY_SID_WAIT_AND_CALL_TIMER_CODE (19)           ;By the end we want to reach a multiple of 64 NOPs.
        ELSE
                ld hl,312 - WAIT_LINE_COUNT
                AKY_SID_WAIT_AND_CALL_TIMER_CODE (7 + 3)        ;The "7" wraps the code in StabilizationLoop above.
        ENDIF


LineLoop
        dec hl
        ld a,l
        or h
        jr z,MainLoopEnd

        ;Some rasters to check the synchronization. If you remove them, remove the "+13" below.
        ld bc,#7f55
        out (c),c
        ld c,#44
        out (c),c

        AKY_SID_WAIT_AND_CALL_TIMER_CODE_AND_JUMP_TO LineLoop, (6 + 13)

MainLoopEnd
        exx
            ld h,0                  ;Resets the line counter.
        exx

        ;Some rasters to check the synchronization. If you remove them, remove the "+7" below.
        ld bc,#7f5c
        out (c),c

        AKY_SID_WAIT_AND_CALL_TIMER_CODE_AND_JUMP_TO PLY_AKYsid_Play, (11 + 7)



KeyPressed
SaveSp  ld sp,0
        call PLY_AKYsid_Stop
        jr $




Music_Start
        include "{{MUSIC_DATA_FNAME}}"
Music_End


Main_Player_Start:
        include "{{PLAYER_SOURCE_FNAME}}"
Main_Player_End




        print "Size of player: ", {hex}(Main_Player_End - Main_Player_Start)
        print "Start of music: ", {hex}(Music_Start)
        print "Size of music: ", {hex}(Music_End - Music_Start)
        print "Total size (player and music): ", {hex}($ - Music_Start)
        print "End: ", {hex}($)

        print "Start of the RET tables: ", {hex}(PLY_AKYsid_RET_TABLES)
        print "End of the RET tables: ", {hex}(PLY_AKYsid_RET_TABLES_END)
