;===================================
;==  GEM JAM - A BEJEWELED CLONE  ==
;==     for the Amstrad CPC       ==
;==    nostalgia Project 2021     ==
;===================================

;==========
;CONSTANTS
;==========

upperleftcorner equ &C108	;top left of the gem field
;process keyboard input and cursor movement
mvright equ 1
mvleft equ -1
mvup equ -8
mvdown equ 8
;exitgame messages:
exit_timeup equ &E1
exit_aborted equ &E2
exit_nomoves equ &E3
menu_selected equ &E4
afterexplosion equ &B		;the variable set after the explosion of a tile (empty)
shortwaitinterval equ &2000
longwaitinterval equ &FF00
tilesetsize equ &1c0
statusline1 equ &C6E0
statusline2 equ &C730
statusline3 equ &C780

;=============
;PROGRAM START
;=============
org &7000

call Init_InitializeMemory	;set our Vars & Tileset to initial state (in case game was restarted)
call Init_InitializeScreen	;set mode &colors using firmware routines
call wait_short		;and let the firmware interrupts process it
		;install interrupt handler, thank you Keith (ChibiAkumas)
		di
		exx
		push BC		;The CPC needs BC' to be backed up or the firmware will die
		exx
		ld HL,(&0038)	;Back up the CPC interrupt handler (at RST7)
		push HL
		ld HL,(&003A)
		push HL
		ld A,&C3
		ld (&0038),A	;Overwrite RST7 with JP
		ld HL,InterruptHandler
		ld (&0039),HL	;Put our label address after the JP
		ei	;Let our interrupt handler run when it wants
call Gfx_DrawIntro
jp Menu_Main

;==========
;GAME MENUS
;==========

Menu_WaitForSelection:
	ld A,(menu_confirm)
	or A
	jr z,Menu_WaitForSelection
	xor A
	ld (menu_confirm),A
	ld A,(menu_highlighted)
ret

Menu_Main:
	;Initialize the global Variables to Menu mode
	call Gfx_Clean_TimerArea
	ld A,1
	ld (toggle_menucursoractive),A
	ld (menu_highlighted),A
	xor A
	ld (toggle_IR_DrawTimer),A
	;check, if we played for the first time
	ld HL,toggle_firstload
	xor A
	cp (HL)
	jr Z,Main_Menu_Loop
	ld IX,message_menumanual;if so, show the manual
	call Gfx_PrintLineInTimerArea
	xor A ;and don't show the score
	ld (toggle_drawscore),A
	;ld (toggle_gamecursoractive),A
	Main_Menu_Loop:
	ld IX,textMenu_Main
	ld HL,statusline1+25
	call Gfx_CleanAndPrintMenu
	call Wait_Short
	call Menu_WaitForSelection
	cp 3
	jp Z,Exit_Game
	cp 2
	jr Z,Menu_Settings
	;else startGame
	call Gfx_Clean_MenuArea
	call Init_SetVarsNewGame
	jp Game_Main

Menu_Settings:
	ld IX,menutext_settings
	ld HL,statusline1+29
	call Gfx_CleanAndPrintMenu
	call Menu_WaitForSelection
	cp 3
	jr Z,Menu_Instructions
	cp 2
	jr Z,Menu_GameSpeed
	;Menu_Tileset comes next automatically

Menu_Tileset:
	ld IX,Menu_Text_Tileset
	ld HL,statusline1+25
	call Gfx_CleanAndPrintMenu
	call Menu_WaitForSelection
	dec A
	call Init_Load_Tileset
	xor A
	ld (toggle_menucursoractive),A
	call Gfx_Clean_MenuArea
	call Gfx_DrawIntro
	jp Menu_Main

Menu_GameSpeed:
	ld IX,Menu_Text_GameSpeed
	ld HL,statusline1+24
	call Gfx_CleanAndPrintMenu
	call Menu_WaitForSelection
	call Init_SetGameSpeed
	jp Menu_Main

Menu_Instructions:
	ld IX,Menu_Text_Instructions
	ld HL,statusline1+1
	call Gfx_CleanAndPrintMenu
	call Menu_WaitForSelection
	jp Menu_Main

Time_Up:
	xor A
	ld (toggle_gamecursoractive),A
	ld (toggle_IR_DrawTimer),A
	call IR_DrawCursor
	ld IX,message_timeup
	call Gfx_PrintLineInTimerArea
	call Wait_Long
	call Wait_Long
	call Logic_Delete_GameTable
	call Gfx_CleanGameField
	jp Menu_Main

No_More_Moves:
	ld a,(logic_gametimer)
	push AF ;first, save the timer
	xor A
	ld (toggle_IR_DrawTimer),A
	ld (toggle_gamecursoractive),A
	call IR_DrawCursor
	call Gfx_Clean_TimerArea
	ld IX,message_nomoves
	call Gfx_PrintLineInTimerArea
	call Wait_Long
	call Wait_Long
	call Logic_CreateNewGameTable
	call Gfx_Clean_TimerArea
	call Gfx_DrawGamedecoration
	ld A,1
	ld (toggle_IR_DrawTimer),A
	ld (toggle_gamecursoractive),A
	ld A,8
	ld (logic_checkmovesindex),a ;reset the index for searching moves
	pop AF
	ld (logic_gametimer),a ;after the animation, restore timer
	jp Game_Loop

Exit_Game:
	;restore the interrupt handler, thank you Keith (ChibiAkumas)
	di
	pop HL		;Restore the firmware interrupt handler
	ld (&003A),HL
	pop HL
	ld (&0038),HL
	exx
	pop BC		;restore BC' to keep the CPC happy
	exx
	ei
	ld A,2
	call &BC0E ;set mode 2
	call Wait_Short
	ld IX,after_game_instructions
	ld HL,statusline3
	call Gfx_PrintMenu
	call wait_long
	ret

Wait_Long:
	push BC		;backup BC
	push AF		;and A
	ld BC,longwaitinterval
	jr wait_loop	;do the loop of the other wait routine
Wait_Short:
	push BC		;backup BC
	push AF		;and A
	ld BC,shortwaitinterval
	wait_loop:
	nop
	nop
	nop
	nop
	nop
	dec BC
	ld A,0
	cp B
	jr NZ,wait_loop
	pop AF
	pop BC
	ret

;================
; INITIALIZATION
;================

Init_InitializeMemory:
	;first, erase all old data that might be at the end (nothing was
	;loaded there but we will store variables there now)
	ld HL,gfx_spriteblacktile
	ld (HL),0
	ld DE,gfx_spriteblacktile+1
	ld BC,&269
	ldir
	xor A
	ld (toggle_menucursoractive),A
	ld (toggle_IR_DrawTimer),A
	ld (toggle_drawscore),A
	inc A ;A=1
	ld (toggle_firstload),A ;is set to 1 bc first load
	inc A ;A=2
	ld (logic_gametimer),a 	;set timer to more than 0 (or we get a panic blink)
	call Init_SetGameSpeed	;is set to 2, bc medium is standard
	xor A
	call Init_Load_Tileset ;to standard: Gems
	;prepare the starfield (each time different!)
	ld HL, gfx_starpositions
	ld B,63
	preparestarsloop:
		getrandomstarpos:
		ld A,R ;get "random" 0-127
		ld E,A
		ld A,R
		add E ;+ another "random" 0-127
		add B ;without that, it's just too evenly distributed
		cp 239
		jr nc,getrandomstarpos
	ld (HL),A ;load the offset to the list
	inc HL
	djnz preparestarsloop
	ret

Init_SetVarsNewGame:
	xor A
	ld (logic_gameinfo),A
	ld (toggle_IR_DrawTimer),A
	ld (toggle_menucursoractive),A
	ld (toggle_firstload),A
	;clear score (A is still 0)
	ld HL,logic_gamescore	;reset logic_gamescore
	ld (HL),A
	inc HL
	ld (HL),A
	inc HL
	ld (HL),A
	inc A
	ld (toggle_drawscore),A ;set to 1, now we show the score
	call Gfx_Clean_TimerArea
	ld A,8		;reset CheckForMoves Index
	ld (logic_checkmovesindex),a
	ld A,20		;reset timer
	ld (logic_gametimer),a
	ld a,&1C
	ld (logic_cursorposition),a
	call Gfx_CleanGameField
	ret

Init_Load_Tileset: ;a is tileset Number from 0 to x
	ld HL,gfx_alltilesets
	ld BC,tilesetsize
	CompareZeroTileset:
	cp 0
	jr Z,dotheldir
	add HL,BC ;add the increment as often
	dec A     ;as a
	jr CompareZeroTileset
	dotheldir:
	ld DE,gfx_spritetiles
	ldir
	ret ;a is ruined

Init_SetGameSpeed: ;a=1 (slow) -3 (fast)
	cp 1
		jr NZ,speednotone
		ld C,24 ;C is gonna be the Clock Speed
		ld B,4 	;B is gonna be the Rasterbar Speed
		jr savespeedvars
	speednotone:
		cp 2
		jr NZ,speednottwo
		ld C,16
		ld B,3
		jr savespeedvars
	speednottwo:
		cp 3
		ret NZ ;a has no valid number
		ld C,8
		ld B,1
	savespeedvars:
	ld HL,__gamespeed_1-1 ;16 standard
	ld (HL),C	;clock speed
	ld HL,__rasterspeed_1-1 ;3 standard
	ld (HL),B	;rasterSpeed
	ret

Init_InitializeScreen:
	xor A ;(a=0)
	call &BC0E ;set mode
	ld B,0
	call &BC38 ;border 0
	ld HL, gfx_maininkcolors
	xor A	;(a=0)
	SetColorLoop:		;read palette from gfx_maininkcolors and set inks
		ld B,(HL)
		ld C,B
		push AF
		push HL
		call &BC32
		pop HL
		pop AF
		inc HL
		inc A
		cp &10
	jr NZ,SetColorLoop
	ret

;============
; GAME LOGIC
;============

Logic_GetRandom1to7: 	;returns a random number between 1 and 7 in A.
	push DE
	ld HL,logic_randomnumbers ;by looking up in a table
		loadnumber:
		ld A,R
		cp 126
		jr NC,loadnumber	;the lookup table only contains 126 values (7*18)
		ld E,A
		ld D,0
		add HL,DE	;add the offset
		ld A,(HL)	;and get the number
	pop DE
	ret	;random number is in A. HL is corrupted

Logic_CreateNewGameTable:
	ld HX,&C
	jr startdeletegametable
Logic_Delete_GameTable:
	;if IXH=0 only delete table
	;if IXH=&C also create another one
	ld HX,0
	startdeletegametable:
	ld HL,logic_gametable
	ld DE,logic_gametable+1
	ld (HL),0
	ld BC,63
	ldir
	xor a
	ld (logic_tableanimationtilescounter),a	;tiles Counter
	dgt_outerloop:
	call GfxC_WaitForDrawing
	ld IY,logic_gametable
	dgt_innerloop:
	ld A,(IY)	;Lies Tile an Position
	cp 8 ;7?	;if it's a tile
	jr C,dgt_nextposition ;do nothing
	cp &D		;if it's destroyed
	jr Z,dgt_nextposition ;neither
	cp &A
	jr NZ,dgt_drawtile	;If not afterexplosion
	ld HL,logic_tableanimationtilescounter
	inc (HL)
	ld A,&C		;check if we delete or rebuild
	cp HX		;if we delete, leave a &C
	jr NZ,dgt_afterincrement
	call Logic_GetRandom1to7	;if not, get a random tile
	jr dgt_afterincrement
	dgt_drawtile:
	inc A		;Wenn explo (8-A) - Inc
	dgt_afterincrement
	ld (IY),a	;write it to this position
	ld B,LY
	call GfxC_GetScreenPosFromTilePos
	push HL
	ld A,(IY)
	call GfxC_GetSpriteAdress
	pop HL
	call Gfx_DrawSprite
	dgt_nextposition:
 	inc IY
	ld A,72
	cp LY
	jr NZ,dgt_innerloop
	ld A,(logic_tableanimationtilescounter)
	cp 64 ;if counter is here, field is full
	ret Z
	cp 62
	jr NC,dgt_outerloop
	dgt_getrandom:
	ld HL,logic_gametable
	ld A,R
	res 6,A ; (0-64)
	add 8
	ld L,A
	xor A
	cp (HL)		;Is there already anything here
	jr NZ,dgt_getrandom
	ld (HL),8 ;(gfx_spriteexplosion1)
	ld B,L
	call GfxC_GetScreenPosFromTilePos
	ld DE,gfx_spriteexplosion1
	call Gfx_DrawSpriteTransparent
	jp dgt_outerloop

Logic_CheckValidRows:
	ld HL, logic_gametable	;Copy Contents of Game Table To Evaluation Table
	ld DE, logic_evaluationtable	;(is this clever?)
	ld BC, 64
	ldir
	ld C,8	;C is Y (we run coordinates for border check parallel to incrementing IX)
	ld IX,logic_gametable
	ld E,0	;evaluation flag - if it's 1, there's at least one row
	gtr_yloop:
		ld B,8		;B is X
		gtr_xloop:
			;evaluate horizontal rows:
			ld A,1			;don't check tiles at the
			cp B			;border bc/ they could have
			jr Z, evaluatevertical	;wrong "neighbors" in the
			ld A,8			;previous/next row
			cp B
			jr Z, evaluatevertical
			ld A,(IX)
			cp (IX-1)		;check for horizontal row
				jr NZ,evaluatevertical
				cp (IX+1)
					jr NZ,evaluatevertical
					ld E,1
					ld (IX+63),0	;if all true, set all 3
					ld (IX+64),0	;to Zero (in the Eval
					ld (IX+65),0	;Table, hence the offset)
			evaluatevertical: ;evaluate vertical rows:
			ld A,&0F
			cp IXL
			jr nc,evaluationdone	;don't check for verticals in the upper row (if IXL<16)
			ld A,&40 ;???
			cp IXL
			jr c,evaluationdone	;neither check for verticals in the bottom row (if IXL>64)
			ld A,(IX)
			CP (IX-8)		;check for vertical row
				jr NZ,evaluationdone
				cp (IX+8)
					jr NZ,evaluationdone
					ld E,1
					ld (IX+56),0	;same as above
					ld (IX+64),0
					ld (IX+72),0
		evaluationdone:
		inc IXL
		djnz gtr_xloop
	dec C
	jr NZ,gtr_yloop
	ret ;E is 1 if we found row(s)

Logic_Set_EmptyToZero:
	;transfer the zeroes of the evaluationtable to the gametable
	ld HL,logic_gametable
	ld DE,logic_evaluationtable
	ld B,64
	SELoop:
		ld A,(DE)
		or a
			jr NZ, SEGoon
			ld (HL),0
		SEGoon:
		inc HL
		inc DE
	djnz SELoop
	ret

Logic_TileSwap:
	;B=tile Position+8
	;A=direction
	ld HL,logic_gametable
	ld L,B		;use as offset
	ld C,(HL) 	;tile at cursor position
	ld D,L		;backup position
	add L
	ld L,A		;move hl to the right direction
	ld B,(HL)	;tile to swap with
	ld (HL),C	;put c in swap position
	ld L,D
	ld (HL),B	;put b in original position
	ret

Logic_CheckForPossibleMoves:
	;is called after every move by setting logic_checkmovesindex to 8
	;while waiting for [logic_gameinfo] input, it's called again and again
	;every time called, it checks, if the horizontal or the vertical
	;swap from logic_checkmovesindex leads to a valid row.
	;If it does, logic_checkmovesindex is set to 0. If not, it's incremented.
	;After the check, you have always to swap back
	;If it reaches 64, logic_gameinfo is set to exit_nomoves
	ld A,(logic_checkmovesindex) ;7 means zero, bc we work directly with the adress
	cp 0
	ret Z	;if a is to zero, we don't have to look anymore
	;horizontal (to the left)
	ld B,A		;Logic_TileSwap needs b as origin tile
	and %00000111	;check if it's the leftmost tile (8,16,24...)
	jr Z, CFPverti	;if it is, skip over to the vertical check
	ld A,mvleft
	call Logic_TileSwap
	call Logic_CheckValidRows :	horizontal
	ld A,(logic_checkmovesindex)
	ld B,A
	ld A,mvleft
	call Logic_TileSwap ;swap back immediately, E (the flag set by Evaluate) stays preserved
	xor A
	cp E	;check if rows found
	jr NZ,CFPMEnd
	;vertical (down)
	CFPverti:
	ld A,(logic_checkmovesindex) ;get it again
	cp &40			;check if bottom row
	jr NC,CFPChecksdone	;if so, skip over
	ld B,A
	ld A,mvdown
	call Logic_TileSwap
	call Logic_CheckValidRows : ;hverticaal
	ld A,(logic_checkmovesindex)
	ld B,A
	ld A,mvdown
	call Logic_TileSwap ;swap back immediately, E (the flag set by Evaluate) stays preserved
	xor A
	cp E	;check if rows found
	jr NZ,CFPMEnd
	CFPChecksdone:
	ld A,(logic_checkmovesindex)
	inc A
	ld (logic_checkmovesindex),A
	cp 72
	ret NZ	;if a is still below 72, we're done. next call uses next index
	;if a=64 i.e. did the whole loop without finding good moves:
	ld A,exit_nomoves
	ld (logic_gameinfo),A
	CFPMEnd ;If we found rows, we just land here
	xor A
	ld (logic_checkmovesindex),A
	ret

Logic_FallUntilNoZeroesLoop:
	xor A		;reset after being
	ld (logic_gameinfo),A	;moved (???)
	call Logic_FillInvisibleAndSetFalling
	call GfxC_PrepareSpriteAdressesforFalling
	call Gfx_FallingTilesListAnimation
	;write the falling in Gametable:
	ld DE, logic_fallingtileslist
	ld HL, logic_gametable
	ld C,8	;column index (+8 bc. GameTable)
		EnterFallingLoop:
		ld L,C
		EFLnexttile:
		ld A,(DE)
		inc E
		cp &FF			;check if end of list
		jp Z,EnterFallingDone
		cp &F0			;check if end of column
		jp z,EFLnextcolumn:
		ld (HL),A	;write in table
		ld A,8		;and go to tile
		add L		;below
		ld L,A
		jp EFLnexttile
		EFLnextcolumn:
		inc C
		jp EnterFallingLoop
	EnterFallingDone:
	ld A,(logic_fallingcolumnsamount)
 	or a
	jr NZ,Logic_FallUntilNoZeroesLoop
	ret

Logic_FillInvisibleAndSetFalling:
	;Fill up Invisible line:
	ld DE,logic_invisibletopline
	ld B,8
	FillInvisible:		;fill the Invisible line
		call Logic_GetRandom1to7
		ld (DE),A	;and put it in the line
		inc E
		djnz FillInvisible
	;set Falling columns:
	;"logic_fallingtileslist" contains a list of the tile ids in every falling column
	; separated by &F0. If there's just F0, this column doesn't fall
	ld C,0	;column index
	ld B,0	;counter of how many columns have zeroes/are gonna fall
	ld DE,logic_fallingtileslist ;adress of the list
	ld HL,logic_invisibletopline
	setfallingcolumnloop:
	ld L,C	;set L to the column
	processtileoffallingcolumn:
	ld A,(HL) ;get tile type
	cp afterexplosion	;if tile has just exploded
	jr Z,bottomofcolumn	;this is the bottom of the falling column
	ld (DE),A	;add tile to list
	inc E
	ld A,8
	add L
	ld L,A		;go to next line
	ld A,71		;if L => 72
	cp L		;we reached the bottom w/o finding zeroes
	jr c,bottomandnozeroes
	jr processtileoffallingcolumn		;if not process this tile
	bottomandnozeroes:
	ld A,E		;so go back in DE!!!
	sub 9		;(up to 9 positions bc we start at
	ld E,A		;the invisible line
	jr fallingcloumnloopend ;and write the columnend sign
	bottomofcolumn:
	inc B		;one falling column more
	fallingcloumnloopend:
	ld A,&F0	;sign for column end
	ld (DE),A
	inc E
	inc C
	bit 3,C		;=8
	jr Z,setfallingcolumnloop
	ld A,&FF
	ld (DE),A		;&FF marks end of list
	ld A,B
	ld (logic_fallingcolumnsamount),A
	ret

Logic_Add_Points:
	;add a*2 to gamescore 
	ld B,0	;reset our 100 mark flag
	push AF
	add A
	cp 10	;in case it's bigger than 9
	jr C,scoreunder10
	sub 10	;we have to convert A
	add &10	;to BCD
	scoreunder10:
	ld HL,logic_gamescore+2	;logic_gamescore has 6 digits
	adc (HL)
	daa
	jr NC,nocarry
	ld B,1	;set the flag if score passed a 100 mark
	nocarry:
	ld (HL),a
	dec HL
	ld A,0
	adc (HL)
	daa
	ld (HL),a
	dec HL
	ld A,0
	adc (HL)
	daa
	ld (HL),a
	ld A,B  ;check the flag
	or A	;we stored in B
	jr Z,addatotimer	;if it's 0, skip the color change
	ld A,(logic_gamescore+1);if not, let's check if the third digit
	and &0F			;is zero (that means 1000 mark passed)
	jr NZ,addatotimer	;if not, skip color change
	ld A,(gfx_mode1colors) ;if not -> color reward
	add 3			;we increase the offset for
	cp 16			;starfield Colors
	jr NZ,savenewstarfieldcoloroffset
	ld A,1
	savenewstarfieldcoloroffset:
	ld (gfx_mode1colors),A
	addatotimer:	;add A to timer
	pop AF	;and add A to timer
	;sra A
	ld HL,logic_gametimer
	add (HL)
	cp 32
	jr C,timernotovermax ;if it's too high,
	ld A,32 ;reset timer to max
	timernotovermax:
	ld (HL),A
	ret

;==========
; GAMEPLAY
;==========

Game_Main:
	call Gfx_DrawGamedecoration
	call Logic_CreateNewGameTable
	ld A,1
	ld (toggle_gamecursoractive),A
	ld (toggle_IR_DrawTimer),A
	ld A,8
	ld (logic_checkmovesindex),A ;reset the index for possible move search
Game_Loop:
	;that's the main game loop
	call Logic_CheckValidRows ;look if there are rows
	xor A
	cp E
	jr Z,Game_WaitForEvent 		;if not, wait for input or events
	call Logic_Set_EmptyToZero 	;if yes, processs
	call Gfx_ExplosionAnimation			;explode
	call Logic_FallUntilNoZeroesLoop	;and refill
	jr Game_Loop

Game_WaitForEvent:
	call Logic_CheckForPossibleMoves 	;while waiting for input, we check if there are any moves.
	ld A,(logic_gameinfo)	;check if something is
	or a					;going on
	jr Z,Game_WaitForEvent	;if not return to loop
	cp exit_timeup
	jp Z,Time_Up			;is the time up?
	cp exit_nomoves
	jp Z,No_More_Moves		;are there no more moves?
	cp exit_aborted
	jp Z,Game_aborted		;did the player abort the game?
	;nothing of these? -> then it's a tile swap:
	xor A
	ld (toggle_gamecursoractive),A ;deactivate cursor
	call Game_TileSwapByInput
	call Logic_CheckValidRows	 ;check if the swap is allowed
	xor A
	cp E	;(were there any changes at all?)
	jr NZ,Game_ProcessChanges
	call Wait_Short
	call Game_TileSwapByInput	;if not swap back!

Game_ProcessChanges:
	call Logic_Set_EmptyToZero ;
	xor A
	ld (logic_gameinfo),A ;reset swap info
	inc A
	ld (toggle_gamecursoractive),A ;reactivate cursor
	ld A,8
	ld (logic_checkmovesindex),A ;reset the index for possible move search
	call Gfx_ExplosionAnimation
	;todo - sion
	call Logic_FallUntilNoZeroesLoop
	jr Game_Loop

Game_aborted:
	xor A
	ld (toggle_gamecursoractive),A
	ld (toggle_IR_DrawTimer),A
	call IR_DrawCursor
	ld IX,message_aborted
	call Gfx_PrintLineInTimerArea
	call Logic_Delete_GameTable
	call Gfx_CleanGameField
	jp Menu_Main

Game_TileSwapByInput:
	ld HL,logic_cursorposition
	ld B,(HL)	;get the cursor position
	ld A,(logic_gameinfo)
	cp 128		;check if the number is negative (->right or up movement)
	jr C,movementOK	;if not, go on
	ld C, A		;if so, invert it- Backup A
	add B		;add the movement
	ld B,A		;save it as the new position
	xor A
	sub C		;and turn A positive (movement in other direction)
	movementOK:
	call Logic_TileSwap
	push BC		;back up the positions
	ld B,L
	call GfxC_GetScreenPosFromTilePos ;needs b as input
	ld A,H
	ld (swapanimorigintile-1),A
	ld A,L
	ld (swapanimorigintile-2),A
	pop BC
	call Gfx_TileSwapAnimation
	ret

;======================
; GRAPHIC CALCULATIONS
;======================

GfxC_GetSpriteAdress:
	;A is gem type -> DE contains address
	ADD A,A			;2 bytes per aderss
	ld HL, gfx_tileslookuptable	;(reads the adress
	ld D,0			;from a lookup table)
	LD E,A
	add HL, DE
	ld E, (HL)
	inc HL
	ld D, (HL)		;sprite address is now in DE
	ret

GfxC_GetScreenPosFromTilePos:
	;B = Tile Number
	ld A,%11111000		;reduce to highest 5 bits to calc x
	and B
	sra A			;half
	ld HL, gfx_screenmemoryoffsets 	;(aligned to &100, so we can operate with L alone)
	ld L, A
	ld E,(HL)
	inc HL
	ld D,(HL)
	ld A,%00000111		;reduce to the lowest 3 bits to calc y
	and B
	sla A			;double twice
	sla A			;-> Y
	ld HL,upperleftcorner	;TopLeftAdressPlusTwo		;top left corner
	ADD L
	ld L,A
	add HL,DE		;->HL=Screen Adress
	ret

GfxC_GetNextLineInScreenMem:
	ld A,H		;Add &08 to H (each CPC line is &0800 bytes below the last
	add &08
	ld H,A
	bit 7,H		;Change this to bit 6,h if your screen is at &8000!
	ret NZ
	ld A,C		;save C for indexing in Drawing routines
	ld BC,&c050	;if we got here we need to jump back to the top of the screen - the command here will do that
	add HL,BC
	ld C,A
	ret

GfxC_PrepareSpriteAdressesforFalling:
	ld BC, logic_fallingtileslist
	ld IX, gfx_fallingspritesaddresses
	PSAFFloop
	ld A,(BC)
	inc BC
	cp &FF
	ret z ;&FF marks End of Table
	cp &F0
	jr nz,normalSpriteAdress
	ld (IX),&F0
	inc IX
	jr PSAFFloop
	NormalSpriteAdress:
	call GfxC_GetSpriteAdress
	ld (IX),E
	ld (IX+1),D
	inc IX
	inc IX
	jr PSAFFloop

GfxC_WaitForDrawing:
	push HL
	ld hl,toggle_drawgamefieldnow
	xor A
	waitforgoodtime:
	cp (HL)
	jr Z,waitforgoodtime
	ld (HL),0
	pop HL
	ret

;==================
; DRAWING ROUTINES
;==================

Gfx_Clean_TimerArea:
	ld HL, &C107
	ld DE, &2208
	call Gfx_DrawBlack
	ret

Gfx_Clean_MenuArea:
	ld HL,statusline1
	ld DE,&8018
	call Gfx_DrawBlack
	ret

Gfx_CleanAndPrintMenu:
	xor A		;this is just a service
	push HL
	push IX
	call Gfx_Clean_MenuArea
	pop IX
	pop HL
Gfx_PrintMenu:
	;IX Adress of String, any letter higher than "Z" ends
	;HL dest on screen
	;load DE with adress of char - gfx_menufont has to be aligned to 100
	ld (gfx_textpositionbackup),HL
	NextCharacter
	ld DE,gfx_menufont
	ld A,(IX)
	inc IX
	sub 32	;ascii 32 is our 0
	cp "["-32 ;after letter Z means EOF
	ret NC
	cp "#"-32 ;means carriage return
	jp NZ,normalCharacter
	ld HL,(gfx_textpositionbackup) ;if not, calc NewLine
	ld BC,&50
	add HL,BC
	ld (gfx_textpositionbackup),HL
	jp NextCharacter ;and go to next
	normalCharacter:
	sla A	;*2
	sla A	;*4
	sla A	;*8
	jp NC,aftersettingD
	inc D ;if A*8 > &FF we have to increase D
	aftersettingD:
	ld E,A
	ld B,8
	charlineloop:
	ld A,(DE)
	inc DE
	ld (HL),a
	ld A,&08
	add H
	ld H,A
	djnz charlineloop
	ld De,&C001 ;=-&3800, +1
	add HL,DE ;returns HL on the next position
	jp NextCharacter

Gfx_PrintLineInTimerArea:
	;IX Adress of String, always 18 Symbols long
	;Dest on screen is always over timer bar
	textdest equ &C106
	;load DE with adress of char - gfx_menufont has to be aligned to 100
	ld HL,textdest
	NextCharacter1
	ld DE,gfx_menufont
	ld A,(IX)
	inc IX
	sub 32
	sla A	;*2
	sla A	;*4
	sla A	;*8
	jp NC,aftersettingD1
	inc D ;if A*8 > &FF we have to increase D
	aftersettingD1:
	ld E,A
	ld B,8
	charlineloop1:
	ld A,(DE)
	inc DE
	ld C,A
	and %11110000
	ld (HL),A ;left bit in ink 2
	inc L
	ld A,C
	and %00001111
	;sla A
	;sla A
	;sla A
	;sla A
	ld (HL),a ;right bit in ink 1
	dec L
	ld A,&08
	add H
	ld H,A
	djnz charlineloop1
	inc L
	inc L
	ld A,&2A ;(&08 + 2*18 )
	cp L	;check if after position 18
	ret Z
	ld H,&C1 ;if not, set H back to first row (L) is already incremented
	jp NextCharacter1

Gfx_TileSwapAnimation:		;has to be called dircetly after TileSwap
	ld A,C		;in order to have the registers correct
	call GfxC_GetSpriteAdress
	ld HL,gemaplustwo-2	;load the adresses of the sprites
	ld (HL),E		;directly to the code in Gfx_TileSwapAnimation
	inc HL
	ld (HL),D
	ld A,B
	call GfxC_GetSpriteAdress
	ld HL,gembplustwo-2
	ld (HL),E
	inc HL
	ld (HL),D
	ld A,(logic_gameinfo)
	;a is logic_gameinfo
	;sprite adress of left/upper gem is in "gemaplustwo"-2
	;sprite adress of right/lower gem is in "gembplustwo"-2
	;HL is screen adress of left tile
	;left tile is always in foreground
	bit 0,A
	jr NZ,horizontalswap
	ld IY, gfx_swapanimationvertical
	ld HL, neighbortileoffsetplustwo-2
	ld (HL),0
	inc HL
	ld (HL),00
	jr swapanimloop
	horizontalswap:
	ld IY, gfx_swapanimationhorizontal
	ld HL, neighbortileoffsetplustwo-2
	ld (HL),&64
	inc HL
	ld (HL),&FF
	swapanimloop:
	call GfxC_WaitForDrawing
	ld A,(IY+1)
	cp 1	;end marker
	ret Z
	ld HL,&c150 :swapanimorigintile
	push HL
	ld DE,gfx_spriteblacktile
	call Gfx_DrawSprite
	ld DE,&FF64 :neighbortileoffsetplustwo
	add HL,DE
	ld DE,gfx_spriteblacktile
	call Gfx_DrawSprite
	pop HL
	ld E,(IY)
	ld D,(IY+1)
	add HL,DE	;new adress in HL
	ld DE,&0000 :gemaplustwo
	call Gfx_DrawSprite	;pos 1, LT
	ld E,(IY+2)
	ld D,(IY+3)
	add HL,DE	;new adress in HL
	ld DE,&0000 :gembplustwo
	call Gfx_DrawSpriteTransparent	;pos 1, RT
	ld DE,4
	add IY,DE
	jr swapanimloop
	ret

Gfx_ExplosionAnimation:
	call GfxC_WaitForDrawing
	call GfxC_WaitForDrawing
	ld HL,logic_gametable
	ld IXH,0	;reset  counter
	EAMainLoop;go through all Tiles
		ld A,(HL)
		cp 0 ;if you find a 0
		jr nz,EAGoOn
		ld (HL),8	;change it to gfx_spriteexplosion1 (8)
		push HL
		ld B,L
		call GfxC_GetScreenPosFromTilePos
		ld DE,gfx_spriteexplosion1
		call Gfx_DrawSpriteTransparent
		ld A,IXH	;this way, the points rise exponentially
		call Logic_Add_Points	;(...i believe...)
		pop HL
		jr Gfx_ExplosionAnimation		;restart from beginning
	EAGoOn:
		cp 8	;if you find >7 and <A:
		jr C,EANextTile
		cp &B
		jr NC,EANextTile
		inc IXH		;inc counter
		inc a 		;inc explosion status
		ld (HL),A	;and put it back
		ld IYL,A	;and save it to IYL
		push HL		;the index in GameTable
		ld B,L
		call GfxC_GetScreenPosFromTilePos
		push HL		;screen des
		ld A,IYL
		call GfxC_GetSpriteAdress
		pop HL		;screen dest
		call Gfx_DrawSprite
		pop HL		;the index in GameTable
	EANextTile:
		inc L	;next tile
		ld A,72
		cp L
		jr NZ,EAMainLoop
		xor A
		cp IXH	;if flag=0 done
		ret Z
		jr Gfx_ExplosionAnimation

Gfx_FallingTilesListAnimation:
	;New Version
	;this is an idea I'm quite proud of. After messing around with whole
	;sprites coming from above the 8x8 field And then having to redraw
	;the upper border of the game field over them, I thought it would
	;be easier to just draw the visible part of the appearing tile ;-)
	;So I created he "Gfx_DrawSpriteformA"-Routine
	ld A,48 ;loop  from line 12 to line 0 step 4, saved in tilesvisiblepart
	AnimationSection:
	ld (gfx_tilesvisiblepart),A
	ld HL,&C1A4	;origin of Table-4 bc addition is at beginning of loop
	push HL
	call GfxC_WaitForDrawing
	ld IY, gfx_fallingspritesaddresses ;list contains the Addresses of the sprites
	FSASColLoop:
	pop HL
	ld A,4
	add L
	cp &C8	;check if one tile to far right
	jr Z,endofFAS
	ld L,A
	push HL
	ld A,(IY)
	inc IY
	cp &F0	;if Column End Marker
	jp Z,FSASColLoop: ;go to next Column
	ld E,A	;if not, it's the Low Byte of the sprite adress
	ld D,(IY) ;and here's the High Byte
	ld A,(gfx_tilesvisiblepart) ;get the current offset
	call Gfx_DrawSpriteFragment ;draw only visible part of first sprite
	inc IY
		FSASRowLoop:	;then draw all the other sprites of the column
		ld A,(IY)
		cp &F0		;(...if there are any others)
		inc IY
		jp Z,FSASColLoop ;after last Sprite of column, &F0 marks end
		ld E,A
		ld D,(IY)
		call Gfx_DrawSprite ;normal Sprite after the first one
		inc IY
		jp FSASRowLoop
	endofFAS:
	ld A,(gfx_tilesvisiblepart)
	sub 16
	jr NC,AnimationSection	;if result negative, end
	ret


Gfx_DrawSpriteFragment:	;Draw only from the byte indicated in A (first line=0)
	ld C,A	;Backup for second Calc
	add E	;E=A+E
	ld E,A	;this is the position in the source memory
	ld A,64	;the full byte counter
	sub C	;...it has to be reduced by that much bytes
	ld C,A
	ld B,0
	jp RepeatYB
Gfx_DrawSprite:	;HL=destination, DE=source sprite adress
	ld BC,64 ;(16*4 ldis's)
	RepeatYB:
		push HL
		ex DE,HL
			ldi
			ldi
			ldi
			ldi
		ex DE,HL ;We need to Put DE back, so swap DE and HL
		pop HL
	call GfxC_GetNextLineInScreenMem
	xor A
	cp C
	jr nz,RepeatYB	;Loop if we're not finished
	ret

Gfx_DrawSpriteTransparent:
	;DE = Adress of Sprite
	;HL = Destination in Screen Memory
	ld IXL,16
	Transrowloop:
	push HL
	ld B,4
		Transcolumnloop:
		ld A,(DE)
		or a
		jr Z,nextST
		ld C,A
		and %10101010
		cp C
		jr Z,orthisbyte
		ld A,C
		and %01010101
		cp C
		jr Z,orthisbyte
		or (HL)
		ld (HL),C
	nextST:	inc HL
		inc DE
		djnz Transcolumnloop
	pop HL
	call GfxC_GetNextLineInScreenMem
	dec IXL
	jr nz,Transrowloop	;Loop if we're not finished
	ret
	orthisbyte:
	ld A,(HL)
	or C
	ld (HL),A
	jr nextST

Gfx_DrawTwoBinaryCodedDigits:
	;take the two digits from A
	;and write them to HL and HL+3 on the screen
	ld IXL,A	;compressed binary digital
	ld A,%11110000
	and IXL	;get left  digit first - we don't have to *16 bc it's the left half
	call Gfx_DrawDigitsRepeatlines
	ld A,%000011111
	and IXL ;get right digit
	sla A ;now we have to shift if 4 times -> *16
	sla A
	sla A
	sla A
	call Gfx_DrawDigitsRepeatlines
ret

Gfx_DrawDigitsRepeatlines:
	;HL is dest adress on Screen
	;A is digit * 16 (address offset)
	ld DE,gfx_font_scoredigits	;every symbol has 16 Bytes
	ld E,A  ;A is the adress offset
	ld B,8
	ddrlloop: ;the loop through all lines
		ld A,(DE)
		ld (HL),a
		inc L	;next screenpos
		inc E ;next source byte
		ld A,(DE)
		ld (HL),a
		dec L ;go back one pos, so we just have to add to H for next line
		inc E
		ld A,&08		;Add &08 to H (each CPC line is &0800 bytes below the last)
		add H
		ld H,A
		djnz ddrlloop	;Loop if we're not finished
	ld DE,&C003 ;-&4000 (8 lines) and +3 (space to next digit)
	add HL,DE
ret

Gfx_DrawBlack:
	;HL start Adress
	;D width, E height
	ld IXH,D
	LineLoopBlack:
	push HL
	ld D,IXH
	ColLoopBlack:
	ld (HL),0
	inc HL
	dec D
	jp NZ,ColLoopBlack
	dec E
	jp Z,endofBlack
	pop HL
	call GfxC_GetNextLineInScreenMem
	jp LineLoopBlack
	endofBlack:
	pop HL
	ret

Gfx_DrawIntro:
	ld HL,toggle_drawscore
	ld (HL),0
	ld HL,&c000
	ld DE,&50C7
	call Gfx_DrawBlack
	ld HL,toggle_firstload
	xor A
	cp (HL)
	jr Z,notfirsttimeintro
	ld IX,message_opening
	call Gfx_PrintLineInTimerArea
	notfirsttimeintro:
	xor A
	ld (toggle_drawscore),A
	ld IXH,18
	ld HL,&C46E
	ld (hlstorage),HL
	introtilesloop:
	ld C,IXH
	ld B,0
	ld HL,0
	sbc HL,BC
	ld B,H
	ld C,L
	call Logic_GetRandom1to7
	add C
	ld C,A
	push BC
		tileLineLoop:
		call Logic_GetRandom1to7
		call GfxC_GetSpriteAdress
		pop BC			;get BC from before
		ld A,C
		add IXH			;add the current offset
		add 2			;+2
		ld C,A			;if not, take the new value to C
		push BC			;save BC
		cp 77			;see if the line is over the end
		jr NC,nexttileline	;and jump to next line if so
		ld HL,(hlstorage)
		add HL,BC		;and add it to the dest Adress
		call Gfx_DrawSpriteTransparent
		jr tilelineloop
	nexttileline:
	call GfxC_WaitForDrawing
	ld HL,(hlstorage)
	call GfxC_GetNextLineInScreenMem
	call GfxC_GetNextLineInScreenMem
	call GfxC_GetNextLineInScreenMem
	call GfxC_GetNextLineInScreenMem	;move 2 lines down
	ld (hlstorage),HL
	dec IXH
	pop BC
	jr NZ,introtilesloop
	call Wait_Long
	ld DE,gfx_labelGEM
	ld HL,&C191
	call Gfx_PutGEMJAMLabel
	call Wait_Long
	ld DE,gfx_labelJAM
	ld HL,&c190+58
	call Gfx_PutGEMJAMLabel
	call Wait_Long

	;clean under gemheap
	ld HL,&D690
	ld D,80
	ld E,4
	call Gfx_DrawBlack
	ret

Gfx_CleanGameField:
	ld HL,&C1A8	;clean in gamefield
	ld D,32
	ld E,128
	call Gfx_DrawBlack
	ret

Gfx_DrawGamedecoration:
	;call Gfx_CleanGameField
	ld HL,&C6A8	;clean under gamefield
	ld D,32
	ld E,4
	call Gfx_DrawBlack
	;timer Frame
	ld HL,&C108
	ld DE,&C109
	ld BC,31
	ld (HL),15
	ldir
	ld HL,&E908
	ld DE,&E909
	ld BC,31
	ld (HL),15
	ldir
	;left and right border
	ld HL,&C1A6
	borderloop:
	ld (HL),4
	inc L
	ld (HL),128
	ld DE,33
	add HL,DE
	ld (HL),132
	inc L
	ld A,(HL)
	and %01010101
	ld (HL),a
	ld DE,&FFDD
	add HL,DE
	call GfxC_GetNextLineInScreenMem
	ld A,&A6	;do this until &DEAE
	cp L
	jp NZ,borderloop
	ld A,&DE
	cp H
	jp NZ,borderloop
	;border patch
	ld DE,gfx_borderpatchleft ;source
	ld HL,&c106	;dest
	call Gfx_DrawBorderPatch
	ld DE,gfx_borderpatchright ;source
	ld HL,&c128	;dest
	call Gfx_DrawBorderPatch
	ld IX,gfx_gamefieldupperborder
	ld HL,&E158
	UPLoop
	ld A,(IX)
	cp &FF
	ret z
	ld (HL),a
	ld D,H
	ld E,L
	inc E
	ld BC,&1F
	ldir
	inc IX
	ld DE,&7E1
	add HL,DE
	jr UPLoop
	ret

Gfx_DrawBorderPatch:
	ld IXL,16
	borderpatchloop:
	ld A,(DE)
	ld (HL),A
	inc HL
	inc DE
	ld A,(DE)
	ld (HL),A
	dec HL
	inc DE
	call GfxC_GetNextLineInScreenMem
	dec IXL
	jr NZ,borderpatchloop
	ret

Gfx_PutGEMJAMLabel:
	ld iyl,48
	RepeatJJ:
	push hl		 ;We have the memory pos of our screen line in HL
		ld B,0	 ;set B to zero (LDIR uses BC
		ld C,21 ;set C to the width

		ex de,hl ;We need to write to HL from DE, so swap DE and HL
		ldir
		ex de,hl ;We need to Put DE back, so swap DE and HL
	pop hl
	call GfxC_GetNextLineInScreenMem ;Get back the memory pos of our screen line
	dec iyl		;Decrease our line counter
	jp nz,RepeatJJ	;Loop if we're not finished
	ret

;================
; THE INTERRUPTS
;================

InterruptHandler:
	;IM1 calls &0038 (RST7) when interrupts occur
	;On the CPC this interrupt handler run 6 times per screen (300hz)
	exx		;Flip over to the shadow registers - we use these so we don't hurt the normal registers
	ex af,af' ;'
	;Read in the screen status - we use this to check if we're at the top of the screen
	ld      b,&f5
	in      a,(c)	;Actually in a,(b)
	rra     ;pop the rightmost bit into the carry
	jp nc,AfterCounterReset
	ld D,1		;reset d, which counts our (six) different actions
	AfterCounterReset:
	ld A,1
	cp d
	jr Z, Interrupt1
	inc a
	cp d
	jr Z, Interrupt2
	inc a
	cp d
	jp Z, Interrupt3
	inc a
	cp d
	jp z, Interrupt4
	inc a
	cp d
	jp z, Interrupt5
	inc a
	cp d
	jp z, Interrupt6
	FinishInterrupt:
	ex af,af' ;'Restore the registers
	exx
	ei
	ret

Interrupt1:
	ld BC,&7fAD ;mode 1
	out (C),C
	ld C,&00	;BG black
	ld A,&54
	out (C),C
	out (C),A
	ld C,&10	;border black
	out (C),C
	out (C),A
	ld HL,gfx_mode1colors
	ld E,(HL) ;load the offset
	ld D,0
	add HL,DE ;and add it to HL
	call Set_Inks1to3
	;TimePanic - red blink
	ld A,(logic_gametimer)
	cp 10
	jr NC,Interrupt1End
	ld A,(logic_clockfortimer)
	cp 8
	jr C,Interrupt1End
	ld HL,gfx_paniccolors
	call Set_Inks1to3
	ld A,&10
	ld BC,&7F4C
	out (C),A	;and the border red too
	out (C),C
	Interrupt1End:
	ld D,2		;next Interrupt next time
	jr FinishInterrupt

Interrupt2: ;draw cursor & timer if needed
	ld A,(toggle_IR_DrawTimer)
	or a
	jp Z,checkIR_DrawCursor
	call IR_DrawTimer
	checkIR_DrawCursor
	ld A,(toggle_gamecursoractive)
	or a
	jp Z,EndofInterrupt2
	call IR_DrawCursor
	EndofInterrupt2:
	ld D,3		;next Interrupt next time
	jr FinishInterrupt

Interrupt3:		;set border to mode 0 and draw stars or read keyboard if needed
	ld BC,&7fAC ;mode 0
	out (C),C
	ld HL,gfx_mode0ink1to3
	call Set_Inks1to3
	;turn the draw gamefield toggle of, bc now the ray is exactly in the field
	xor a
	ld (toggle_drawgamefieldnow),a
	;read the toggle what to do next
	ld A,(toggle_drawstars_readkeyboard)
	inc A
	ld (toggle_drawstars_readkeyboard),a
	bit 0,A
	jp z,IR_Draw_Stars
	call IR_ReadKeyboardAndChangeCursor ;if not, Keyboard Game Routine
	ld a,(toggle_gamecursoractive)
	or A	;if the gamecursor is active (not zero)
	call nz,IR_DecreaseTimerRoutine	;take care of the timer
	jr EndofInterrupt3	;and then finish
	keyboardmenu:
	call ReadKeyboardMenu
	EndofInterrupt3:
	ld D,4		;next Interrupt next time
	jp FinishInterrupt

Interrupt4:
	;turn the draw gamefield toggle back on, the ray is past the field
	ld A,1
	ld (toggle_drawgamefieldnow),a
	;apart from this we leave IR4 empty, because
	;we'll need the processing power for falling tiles etc.
	EndofInterrupt4:
	ld D,5		;next Interrupt next time
	jp FinishInterrupt

Interrupt5:
	ld A,(toggle_drawscore)
	or A		;if game is on
	jr Z,dontdrawscore	;don't draw logic_gamescoreboard
	jp IR_Draw_Score	;otherwise do
	dontdrawscore:
	ld a,(toggle_menucursoractive)
	or a ;If there is a Menu, draw the bottom raster
	jr Z,EndofInterrupt5
	ld B,&6C
	wastetimeinsteadofdrawingscore:
		push BC
		pop BC
		djnz wastetimeinsteadofdrawingscore
	nop
	nop	;waste time
	backtoInterrupt5:
	ld B,&99
	waitforhorizontalline:
		nop
	djnz waitforhorizontalline
	ld BC,&7f00	;BG white
	ld A,&4B
	ld D,&10	;border white
	out (C),C
	out (C),A
	out (C),D
	out (C),A
		ld B,&1C
		waitXX1:
		djnz waitxx1
	ld BC,&7f00	;BG grey
	ld A,&41
	ld D,&10	;border grey
	out (C),C
	out (C),A
	out (C),D
	out (C),A
		ld B,9
		waitXX2:
		djnz waitxx2
	ld BC,&7f00	;BG blac
	ld A,&54
	ld D,&10	;border black
	out (C),C
	out (C),A
	out (c),D
	out (c),A
	ld BC,&7fAE 	;mode 2
	out (C),C
	ld A,(toggle_menucursoractive)
	or a			;either draw the menu bar
	jp NZ,IR_DrawMenuBar
	ld A,(toggle_drawscore)
	or A
	jp NZ,IR_DrawRasterBarAnim ;or the raster anim
	EndofInterrupt5:
	call IR_RasterbarNextstep
	ld D,6		;next Interrupt next time
	jp FinishInterrupt

Interrupt6:
	;not much happens here, bc the main game ^will be busy drawing tile animations
	EndofInterrupt6:
	jp FinishInterrupt

IR_DrawMenuBar:
	ld A,(menu_highlighted)
	cp 1
	jr NZ,notone
	ld B,&0E
	jr menudrawingroutine:
	notone:
	cp 2
	jr NZ,nottwo
	ld B,&73
	jr menudrawingroutine:
	nottwo:
	cp 3
	jp NZ,EndofInterrupt5
	ld B,&D9
	menudrawingroutine:
		waitxx5:
		nop
		djnz waitxx5
	ld HL,gfx_rasterbarcolors
	ld A,(HL) ;first number is the offset for animation
	add L
	ld L,A
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	inc HL
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	inc HL
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	inc HL
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	inc HL
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	dec HL
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	dec HL
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	nop
	nop
	dec HL
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	nop
	nop
	dec HL
	ld A,(HL)
	call IR_DrawRasterlineAndWait
	ld A,&54
	nop
	call IR_DrawRasterlineAndWait
	jp EndofInterrupt5


IR_DrawRasterBarAnim:
			ld B,2
			waitbbb:
			djnz waitbbb:
	ld HL,gfx_rasterbarcolors
	ld A,(HL) ;first number is the increment for animation, but not 0
	add L
	ld L,A
	ld E,24
		gamerasterloop:
		ld A,(HL)
		ld BC,&7f00
		ld D,&10
		;draw raster
		out (C),C
		out (C),A
		out (c),D
		out (C),A
			ld B,8
			waitzzz:
			djnz waitzzz:
		inc BC ;just to waste the right amount of time
		inc HL
		dec E
		jp NZ,gamerasterloop
	ld A,&54
	call IR_DrawRasterlineAndWait
	nop
	ld A,&40
	call IR_DrawRasterlineAndWait
	nop
	ld A,&4b
	call IR_DrawRasterlineAndWait
	nop
	ld A,&54
	call IR_DrawRasterlineAndWait
	nop
	jp EndofInterrupt5

Set_Inks1to3:
	ld B,&7F
	ld A,1
	siloop:
	ld C,(HL)
	out (C),A
	out (C),C
	inc A
	inc HL
	cp 4
	jr nz,siloop
	ret

IR_DrawRasterlineAndWait:
	ld BC,&7f00
	ld D,&10
	out (C),C
	out (C),A
	out (c),D
	out (C),A
	ld B,7
	waitXX6:
	dec B
	JP NZ,waitxx6
	ret

IR_RasterbarNextstep: ;and increase Cursor color
	ld A,1 :countdownrasterbarplusone
	dec A
	jr NZ, writerasterbarcounter ;only if the counter is at 0 change to next offset & cursor color
	ld HL,gfx_ink15blinkingcolors
	ld A,(HL)
	inc A
	cp 13
	jr NZ,LoadTheCursorColor
	ld A,1
	LoadTheCursorColor:
	ld (HL),A
	add L		;add the offset to HL
	ld L,A
	ld A,(HL) ;get the color
	ld BC,&7f0F ;ink 15
	out (C),C
	out (C),A
	ld A,(gfx_rasterbarcolors)
	dec A	;increase offset in the color list
	jr NZ, writebackrasteroffset: ;if reached 0
	ld A,52	;reset to 52 (length of list)
	writebackrasteroffset:
	ld (gfx_rasterbarcolors),A
	ld A,3 :__rasterspeed_1 ;reset counter = speed of raster animation
	writerasterbarcounter:
	ld (countdownrasterbarplusone-1),a
	ret

IR_DrawCursor:
	ld hl,logic_cursorposition
	ld B,(HL)
	call GfXc_GetScreenPosFromTilePos
	dec hl
	ld DE,gfx_cursorbitmap
	ld C,18;x
	cursorXloop:
		ld B, 5 ;y
		cursorYloop:
		ld A,(DE)
		or a
		jr Z,dontdraw
		OR (HL)
		ld (HL),A
		dontdraw:
		inc HL
		inc E
		djnz cursorYloop
	push DE
	ld DE,5
	SBC HL,DE	;back to first column
	pop DE
	call GfxC_GetNextLineInScreenMem
	dec C		;Decrease our line counter
	jr nz,cursorXloop;Loop if we're not finished
ret

IR_EraseCursor:
	ld HL,logic_cursorposition
	ld B,(HL)
	call GfXc_GetScreenPosFromTilePos
	dec HL ;bc the cursor starts one byte to the left of the tile
	ld DE,gfx_cursorbitmap
	ld C,18;x
	EcursorXloop:
		ld B, 5 ;y
		EcursorYloop:
		ld A,(DE)
		or a
		jr Z,ECdoneDrawing
		CP &FF
		jr NZ,drawXOR
		ld (HL),0
		jr ECdoneDrawing
		drawXOR:
		XOR &FF
		AND (HL)
		ld (HL),A
		ECdoneDrawing:
		inc HL
		inc E
		djnz EcursorYloop
	push DE
	ld DE,5
	SBC HL,DE	;back to first column
	pop DE
	call GfxC_GetNextLineInScreenMem
	dec C		;Decrease our line counter
	jp nz,EcursorXloop;Loop if we're not finished
ret

IR_Draw_Stars:
	ld HL, gfx_starpositions	;go to beginning of list
	ld D,&C0
	ld B,64		;counter for speed calc
	Starloop:	;(0=1px speed, 1=2px, 2=4 px, 3=6px speed)
	ld A,(HL)		;if the high byte
	ld E,A			;else make DE complete
	ld A,(DE)		;and check the pixels
	ld C,A			;back it up
	Xor A
	LD (DE),A		;and overwrite on screen
	ld A,B
	and 3	;get counter mod 4
	jp Z,star1pxspeed
	cp 1
	jp Z,star2pxspeed
	cp 2
	jp Z,star4pxspeed
	;star6pxspeed:
	inc E	;go to next byte
	ld A,C	;First, change the pixel bitmap
	cp 8	;if it's (X---) (pen 2)
	jr NZ,its2at6px
	ld C,2	;then change to (--X-)
	jp checkoverthirdline
	its2at6px:
	ld C,8	;else transfer to X---
	inc E	;and go one more byte farther
	jp checkoverthirdline
	star1pxspeed:
	ld A,C
	cp 136
	jp NZ,not136
	ld C,68
	jp checkoverthirdline
	not136:	cp 68
	jp NZ,not68
	ld C,34
	jp checkoverthirdline
	not68:	cp 34
	jp NZ,not34
	ld C,17
	jp checkoverthirdline
	not34:	ld C,136
	inc E
	jp checkoverthirdline
	star2pxspeed:
	ld A,C	;First, change the pixel bitmap
	cp 8	;if it's (X---) (pen 2)
	jr NZ,its2at2px
	ld C,2	;then change to (--X-)
	jp checkoverthirdline
	its2at2px:
	ld C,8 ;else transfer to X---
	inc E	;and go one byte farther
	jp checkoverthirdline
	star4pxspeed:
	ld C,64
	inc E
	checkoverthirdline:
	ld A,239		;check if over third line
	cp E
	jr NC, setnewposition
	ld A,R
	and %00000111
	ld E,A
	setnewposition:
	ld A,C		;load the new bitmap to A
	ld (DE),A	;and draw it on the screen
	ld (HL),E	;save the new lowbyte of current star
	inc HL		;and go to next position in the table
	ld A,8		;as for the "line" offset
	add D		;increase (by &800)
	jr NC,finishstarloop ;if we exceeded &FF, C is set
	ld A,&C0	;in this case reset to &C0(00)
	finishstarloop
	ld D,A		;ld the hibyte offset
	djnz Starloop
	jp EndofInterrupt3

IR_ReadKeyboardAndChangeCursor:
;Keyboard Routine from CPCWiki
	;I actually don't understand how it works in detail
	ld BC,#f782     ;3
	out (C),C       ;4
	ld BC,#f40e     ;3
	ld E,B          ;1
	out (C),C       ;4
	ld BC,#f6c0     ;3
	ld D,B          ;1
	out (C),C       ;4
	xor A           ;1
	out (C),A       ;4
	ld BC,#f792     ;3
	out (C),C       ;4
	ld A,#40        ;2
	ld C,#4a        ;2 43
	ld B,D          ;1
	out (C),A      ;4 select line
	ld B,E          ;1
	in A,(C)
	ld L,A
	ld A,#41
	ld B,D         ;1
	out (C),A	;4 select line
	ld B,E          ;1
	in A,(C)
 	LD H,A
	ld A,#45
	ld B,D          ;1
	out (C),A	;4 select line
	ld B,E          ;1
	in A,(C)
	bit 7,A
	jr NZ,afterspacecheck
	res 5,H 	;set bit 5 in H
	afterspacecheck:
	ld BC,#f782     ;3
	out (C),C       ;4
	ld A,(kbd_oldinput+1) ;check if
	CP H               ;the keybard
	jr NZ,kbd_newinput ;status has changed
	ld A,(kbd_oldinput)
	CP L
	ret Z
	kbd_newinput:
	ld (kbd_oldinput),HL		;save the keybard status
	xor a				;see if a key is pressed:
	xor H				;then it would Be FF now
	xor L 				;and would Be 0 now
	ret Z
	notzeroentry:
	ld A,(toggle_menucursoractive)
	or A ;if the Menu is active, go on with its own routine
	jr nz, ReadKeyboardMenu
	ld HL,logic_cursorposition
	ld B,(HL)
	push BC ;backup cursor position for processing afterwards
	;erase the cursor bc we're gonna draw it somewhere else
	call IR_EraseCursor
	pop BC ;B contains cursor
	ld C,0  ;C will contain direction
	ld HL,(kbd_oldinput)
		;L - 6. ENTER , 2.DOWN 1.RIGHT 0.UP
		;H - 5.SPACE 0.LEFT
		;check he space bar:
	checkspace: bit 5,H
		jr NZ, checkleft
		bit 6,L 	;check if enter was pressed
		jr NZ,checkleft	;(copy+enter ends game)
		ld C,exit_aborted
		jr writelogic_gameinfo
	checkleft: bit 0,H
		jr NZ, checkright
		ld A,%00000111	;check if movement possible
		AND B
		jr Z, checkright
		ld C,mvleft
	checkright: bit 1,L
		jr NZ,checkup
		ld A,%0000111	;check if movement possible
		AND B;
		cp %111
		jr Z, checkup
		ld C, mvright
	checkup: bit 0,L
		jr NZ,checkdown
		ld A,&F
		cp B		;not possible if top row
		jr NC,checkdown
		ld C, mvup
	checkdown: bit 2,L
		jr NZ,allcursorchecksdone
		ld A,&3F
		cp b		;not possible if bottom row
		jr C,allcursorchecksdone
		ld C, mvdown
	allcursorchecksdone:
		xor a
		cp c
		ret Z
		bit 5,H	;check again if copy
		jr NZ, cursormovement
	writelogic_gameinfo:
		ld A,C
		ld (logic_gameinfo),a
		ret
	cursormovement:
		ld A,B
		add C	;a has now the new position
		;or a8
		ld (logic_cursorposition),a
ret

ReadKeyboardMenu: ;where we only check up,down and space
	ld HL,(kbd_oldinput)
	;check space:
		bit 5,H
		jr NZ,checkmenuup
		ld A,1
		ld (menu_confirm),A ;that's the variable the waitforkey routine expects
		ret
	checkmenuup: bit 0,L ;up
		jr NZ,checkmenudown
		ld A,(menu_highlighted)
		cp 1	;if it's 1 you can't go up
		ret Z
		dec a
		ld (menu_highlighted),a
		ret
	checkmenudown:
		bit 2,L ;down
		jr NZ,nomenurelevantkeys
		ld A,(menu_highlighted)
		cp 3	;if it's 3 you can't go down
		ret Z
		inc a
		ld (menu_highlighted),a
		ret
	nomenurelevantkeys:
		ret

IR_DecreaseTimerRoutine:
	ld A,(logic_clockfortimer) ;load the clock for the clock
	inc a                       ;increment it
	ld (logic_clockfortimer),a  ;load it back
	cp 16				:__gamespeed_1  ;self-modifying variable for game speed
	ret NZ	       ;if it's not there, we're done
	xor A
	ld (logic_clockfortimer),a ;reset clock for the clock
	ld A,(logic_gametimer)     ;load the actual gametime
	dec a     ;and decrease it
	cp 0     ;check if 0
	jr NZ,WriteTimeBack ;if not, finish
	ld A,exit_timeup  ;if so,
	ld (logic_gameinfo),a   ;tell the main game
	WriteTimeBack:
	ld (logic_gametimer),a
	ret

IR_Draw_Score:
	push IX		;backup these
	push IY		;(bc we did't when starting the interrupt)
	ld hl,&c06F	;screen adress of the score Table
	ld IY,logic_gamescore
	ld A,(IY)			;the six digits are stored and drawn in pairs
	call Gfx_DrawTwoBinaryCodedDigits
	ld A,(IY+1)
	call Gfx_DrawTwoBinaryCodedDigits
	ld A,(IY+2)
	call Gfx_DrawTwoBinaryCodedDigits
	pop IY
	pop IX
	jp backtoInterrupt5

IR_DrawTimer:
	ld HL,logic_gametimer
	ld C,(HL)	;load current time to C
	ld A,32
	sub C
	ld E,A		;E is now the "black" part of the bar
	ld C,(HL)	;anc C is the "white" part.
	ld HL,&C908	;starting address on screen
	ld D,4			;counter for four lines
	timerlines:
	push HL		;for adding line later
	ld B,C
	whitepart:	;the "time" part of the bar
		ld (HL),240 ;bitmap for all ink 3
		inc L
	djnz whitepart
	xor A
	cp E	;in case time is full
	jr Z,finishtimerlines	;don't draw blackpart
	ld B,E
	blackpart: ;the "no more time" part
		ld (HL),0 ;bitmap for all ink 0
		inc L
	djnz blackpart
	finishtimerlines:
	pop HL		;restore from before
	ld A,&08	;add one line offset
	add H
	ld H,A
	dec D
	jr NZ,timerlines
	ret

;==============
;  MENU TEXTS
;==============

textMenu_Main:      db '       START A NEW GAME#CONFIGURATION AND INSTRUCTIONS#         EXIT GEM JAM_'
menutext_settings:  db '   SELECT  TILESET#  CHOOSE GAME SPEED#READ THE INSTRUCTIONS_'
Menu_Text_Tileset:  db "  PLAYING WITH PRECIOUS GEMS#  JAMMING WITH HEALTHY FRUIT#PUZZLING WITH ABSTRACT SHAPES_"
Menu_Text_GameSpeed:db "  SLOW (FOR BEGINNERS & LEARNERS)#MEDIUM (FOR OCCASIONAL PLAYERS)#  FAST (FOR THE MASTERS OF ZILOG)_"
Menu_Text_Instructions: db "BLOW UP TILES BY BUILDING ROWS OR COLUMNS OF THREE OR MORE MATCHING TILES.#"
                        db "BUT BE CAREFUL, THE CLOCK IS RUNNING! MOVE THE CURSOR WITH THE ARROW KEYS.#"
                        db "   PRESS SPACE+ARROW TO SWAP TWO TILES. PRESS SPACE+ENTER TO ABORT GAME._"

message_opening:    db " LEOSOFT PRESENTS "
message_nomoves:    db " - NO MOVES LEFT -"
message_menumanual: db "    </>/SPACE     "
message_aborted:    db " - GAME ABORTED - "
message_timeup:     db "  ! TIME IS UP !  "

after_game_instructions: db 'RESTART WITH "RUN 50"_'

align &10
gfx_ink15blinkingcolors: db 1, &4B, &4B, &43, &4B, &4B, &5B, &4B, &4B, &59, &4B, &4B, &4B

gfx_mode1colors: db 1 ;offset
		db &53, &5F, &55 ;blue
		db &59, &52, &56 ;green
		db &4F, &4D, &58 ;Magenta
		db &4E, &45, &4C ;red
		db &4B, &4A, &5E ;yellow
gfx_paniccolors:db &4C, &4C, &5C ;panic red
gfx_mode0ink1to3: db &4B, &40, &4E ;the first three colors of the main screen

gfx_starbitmaps:
	db &44, &08, &80, &02, &22, &02, &20, &08, &11, &02, &40, &08, &88, &08, &10, &02

gfx_tileslookuptable:
	db &40, &94 ;empty
	db &80, &92, &C0, &92, &00, &93, &40, &93, &80, &93, &C0, &93, &00, &94  ;tiles
	db &C0, &91, &00, &92, &40, &92 ;explosions (come first in memory, but later in indexing)
	db &40, &94, &40, &94, &40, &94 ;afterexplosion

align &100
gfx_font_scoredigits:
	db &47, &2E, &8F, &1F, &2E, &47, &0C, &03, &0C, &03, &2E, &47, &8F, &1F, &47, &2E
	db &11, &0C, &23, &0C, &47, &0C, &06, &0C, &66, &0C, &00, &0C, &00, &0C, &00, &0C
	db &47, &2E, &8F, &1F, &00, &17, &47, &1F, &8F, &2E, &0C, &00, &0F, &0F, &0F, &0F
	db &47, &2E, &8F, &1F, &00, &17, &23, &1F, &23, &0F, &00, &03, &0F, &0F, &8F, &1F
	db &03, &06, &47, &06, &17, &06, &8E, &06, &0F, &0F, &0F, &0F, &00, &06, &00, &06
	db &8F, &0E, &8F, &0E, &AE, &00, &8F, &1F, &8F, &0F, &00, &03, &0F, &0F, &8F, &1F
	db &47, &2E, &8F, &1F, &2E, &00, &0F, &1F, &0F, &0F, &0C, &03, &0F, &0F, &8F, &1F
	db &0F, &0F, &0F, &1F, &00, &2E, &01, &4C, &23, &08, &03, &88, &03, &00, &03, &00
	db &47, &2E, &07, &0E, &06, &06, &47, &2E, &8F, &1F, &2E, &47, &0F, &0F, &8F, &1F
	db &47, &2E, &07, &1F, &06, &57, &07, &0F, &47, &0F, &00, &03, &0F, &0F, &8F, &1F

gfx_borderpatchleft:
	db &00, &01, &00, &23, &00, &23, &00, &23, &00, &23, &00, &01, &00, &00, &00, &00
	db &00, &00, &00, &00, &00, &00, &00, &00, &00, &0C, &04, &48, &04, &C0, &04, &84

gfx_borderpatchright:
	db &08, &00, &4C, &00, &4C, &00, &4C, &00, &4C, &00, &08, &00, &00, &00, &00, &00
	db &00, &00, &00, &00, &00, &00, &00, &00, &08, &00, &0C, &00, &84, &00, &84, &00


align &100
gfx_screenmemoryoffsets:
	defb &00,&00, &50,&00, &A0,&00, &F0,&00, &40,&01, &90,&01, &E0,&01, &30,&02
	defb &80,&02, &D0,&02, &20,&03, &70,&03, &C0,&03, &10,&04, &60,&04, &B0,&04
	defb &00,&05, &50,&05, &A0,&05, &F0,&05, &40,&06, &90,&06, &E0,&06, &30,&07
	defb &80,&07

gfx_swapanimationhorizontal: ;(from tile a to b and back to next position for a)
	db &00, &00, &64, &FF
	db &01, &00, &62, &FF
	db &02, &00, &60, &FF
	db &03, &00, &5E, &FF
	db &04, &00, &5C, &FF
	db 0, 1

gfx_swapanimationvertical: ;(from tile a to b and back to next position for a)
	db &00, &00, &00, &00
	db &00, &20, &B0, &FF
	db &50, &00, &60, &FF
	db &50, &20, &10, &FF
	db &A0, &00, &C0, &FE
	db 0,1

gfx_maininkcolors:
	db 0, 26, 13, 15, 6, 11, 2, 24, 12, 18, 9, 4, 8, 3, 7, 26

gfx_gamefieldupperborder: db 12, 192, 192, 12, &FF

logic_randomnumbers:
	db 6, 3, 2, 5, 4, 1, 2, 1, 7, 1, 7, 4, 6, 6, 4, 3
	db 6, 2, 2, 7, 1, 2, 6, 3, 6, 7, 2, 1, 7, 5, 7, 6
	db 5, 5, 3, 1, 5, 3, 2, 5, 2, 6, 5, 3, 3, 3, 4, 5
	db 6, 6, 6, 2, 7, 4, 2, 3, 2, 5, 3, 7, 5, 1, 6, 4
	db 1, 7, 1, 4, 7, 1, 2, 3, 4, 4, 4, 7, 5, 6, 4, 3
	db 2, 6, 3, 2, 2, 5, 4, 1, 1, 6, 5, 5, 1, 1, 1, 5
	db 4, 6, 6, 7, 6, 3, 1, 7, 1, 1, 3, 5, 3, 7, 5
	db 4, 2, 2, 7, 3, 4, 4, 2, 5, 7, 7, 7, 4, 4, 3

align &100
gfx_rasterbarcolors:
	db 52 ;starting position
	db &54, &54, &54, &54, &54, &5C, &4C, &4E, &4A, &4A, &4E, &4C, &5C
	db &54, &54, &54, &54, &54, &44, &55, &57, &53, &53, &57, &55, &44
	db &54, &54, &54, &54, &54, &58, &45, &4D, &4F, &4F, &4D, &45, &58
	db &54, &54, &54, &54, &54, &56, &5E, &52, &5A, &5A, &52, &5E, &56
	db &54, &54, &54, &54, &54, &5C, &4C, &4E, &4A, &4A, &4E, &4C, &5C
	db &54, &54, &54, &54, &44, &55, &57, &53, &5B, &57, &55, &44

gfx_cursorbitmap:
	db &55,&FF,&00,&55,&FF, &55,&FF,&00,&55,&FF
	db &55,&00,&00,&00,&55, &55,&00,&00,&00,&55
	db &55,&00,&00,&00,&55, &00,&00,&00,&00,&00
	db &00,&00,&00,&00,&00, &00,&00,&00,&00,&00
	db &00,&00,&00,&00,&00, &00,&00,&00,&00,&00
	db &00,&00,&00,&00,&00, &00,&00,&00,&00,&00
	db &00,&00,&00,&00,&00, &55,&00,&00,&00,&55
	db &55,&00,&00,&00,&55, &55,&00,&00,&00,&55
	db &55,&FF,&00,&55,&FF, &55,&FF,&00,&55,&FF

gfx_alltilesets:
;gems
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &04, &00, &00, &00, &48, &08, &00, &04, &C0, &84, &00, &04, &C0, &AC, &00, &48, &C0, &5C, &08, &48, &C0, &E8, &08
db &48, &D4, &E8, &08, &48, &D4, &C0, &08, &48, &FC, &C0, &08, &5C, &48, &C0, &08, &04, &E8, &84, &00, &04, &C0, &84, &00, &00, &48, &08, &00, &00, &04, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &54, &00, &00, &00, &56, &02, &00, &00, &A9, &A8, &00, &01, &FC, &A9, &00, &54, &56, &56, &00, &56, &FC, &FC, &02
db &FC, &FC, &A9, &A8, &A9, &FC, &FC, &A8, &56, &FC, &FC, &02, &54, &56, &56, &00, &01, &FC, &A9, &00, &00, &A9, &A8, &00, &00, &56, &02, &00, &00, &54, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &14, &B4, &B4, &00, &78, &B4, &78, &28, &F0, &B4, &3C, &A0, &3C, &78, &3C, &28, &B4, &3C, &F0, &A0, &78, &3C, &F0, &28
db &50, &3C, &F0, &00, &50, &3C, &F0, &00, &14, &B4, &B4, &00, &00, &B4, &A0, &00, &00, &B4, &A0, &00, &00, &78, &28, &00, &00, &50, &00, &00, &00, &50, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &10, &00, &00, &00, &64, &20, &00, &10, &CC, &98, &00, &64, &CC, &64, &20, &CC, &CC, &CC, &88, &CC, &CC, &98, &88
db &64, &CC, &CC, &88, &CC, &CC, &CC, &20, &98, &CC, &CC, &88, &CC, &CC, &CC, &88, &64, &64, &CC, &20, &10, &CC, &98, &00, &00, &64, &20, &00, &00, &10, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &05, &C3, &87, &00, &05, &87, &87, &00, &4B, &87, &C3, &0A, &4B, &4B, &C3, &0A, &C3, &4B, &C3, &82, &87, &C3, &C3, &82
db &87, &C3, &C3, &0A, &4B, &C3, &C3, &0A, &4B, &C3, &87, &82, &C3, &C3, &87, &82, &4B, &C3, &4B, &0A, &4B, &C3, &4B, &0A, &05, &87, &87, &00, &05, &C3, &87, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &11, &00, &00, &00, &11, &00, &00, &00, &9B, &8A, &00, &00, &9B, &8A, &00, &00, &33, &22, &00, &00, &33, &22, &00
db &45, &33, &67, &00, &45, &33, &67, &00, &11, &33, &33, &00, &11, &33, &33, &8A, &9B, &33, &33, &8A, &33, &33, &33, &22, &67, &CF, &CF, &22, &33, &33, &33, &22
db &00, &00, &00, &00, &00, &00, &00, &00, &51, &3F, &7B, &00, &B7, &3F, &3F, &A2, &3F, &3F, &B7, &2A, &3F, &3F, &7B, &2A, &3F, &3F, &7B, &2A, &3F, &3F, &3F, &2A
db &3F, &3F, &3F, &2A, &3F, &3F, &3F, &2A, &7B, &3F, &3F, &2A, &7B, &3F, &3F, &2A, &3F, &B7, &3F, &2A, &3F, &3F, &3F, &2A, &B7, &3F, &3F, &A2, &51, &3F, &7B, &00
;fruit
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &51, &88, &00, &00, &44, &A2, &00, &00, &FC, &00, &00, &01, &FC, &00, &00, &54, &A9, &00, &00, &56, &A9, &00
db &00, &FC, &A8, &00, &00, &FC, &A8, &00, &00, &FC, &A8, &00, &00, &FC, &A9, &00, &00, &56, &FC, &00, &00, &54, &FC, &A9, &00, &01, &FC, &FC, &A8, &00, &FC, &03
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &82, &00, &00, &41, &0A, &00, &00, &82, &82, &00, &05, &0A, &0A, &00, &41, &01, &00, &00, &0F, &41
db &00, &00, &82, &41, &00, &B2, &71, &41, &00, &60, &71, &30, &A2, &30, &B2, &90, &20, &30, &B2, &30, &20, &30, &B2, &30, &20, &B2, &B2, &30, &20, &00, &51, &30
db &A2, &00, &00, &00, &00, &00, &00, &00, &00, &00, &44, &00, &00, &00, &4E, &0A, &00, &00, &C3, &82, &00, &00, &C3, &82, &00, &00, &C3, &82, &00, &00, &C3, &87
db &00, &05, &C3, &87, &00, &4B, &C3, &C3, &0A, &4B, &C3, &C3, &0A, &C3, &C3, &D6, &82, &C3, &C3, &D6, &82, &C3, &C3, &D6, &82, &4B, &C3, &C3, &0A, &05, &C6, &87
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &A7, &C3, &0A, &00, &C9, &C3, &82, &00, &D8, &C3, &0A, &00, &78, &B4, &00, &00, &B4, &F0, &28, &00, &B4, &F0, &B4
db &00, &F0, &78, &F0, &00, &F0, &78, &F0, &28, &78, &B4, &F0, &28, &50, &B4, &F0, &A0, &14, &F0, &78, &A0, &00, &F0, &78, &A0, &00, &F0, &B4, &28, &00, &14, &F0
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &A3, &A2, &00, &51, &21, &71, &00, &B2, &98, &98, &A2, &E6, &CC, &CC, &A2, &64, &64, &64
db &20, &CC, &CC, &CC, &88, &98, &98, &98, &88, &CC, &CC, &CC, &88, &64, &64, &64, &20, &E6, &CC, &CC, &A2, &B2, &98, &98, &A2, &51, &64, &71, &00, &00, &F3, &A2
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &C3, &0A, &00, &00, &C3, &C3, &00, &05, &93, &9B, &8A, &41, &C7, &33, &22, &9B, &CB, &33, &22, &33, &63, &9B
db &8A, &33, &67, &67, &8A, &9B, &9B, &33, &8A, &67, &93, &33, &8A, &33, &67, &67, &22, &33, &9B, &9B, &22, &CF, &33, &33, &8A, &45, &33, &67, &00, &00, &9B, &8A
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &4B, &0A, &00, &05, &C3, &87, &00, &E3, &E9, &E9, &A2, &FC, &FC, &FC, &A8, &56, &56, &56, &02, &A9, &A9, &A9
db &A8, &FC, &FC, &FC, &A8, &56, &56, &56, &02, &A9, &A9, &A9, &A8, &FC, &FC, &FC, &02, &56, &56, &56, &02, &01, &A9, &A9, &00, &01, &FC, &A9, &00, &00, &03, &02, &00
;shapes
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &04, &00, &00, &00, &04, &00, &00, &00, &0C, &08, &00, &00, &48, &08, &00, &00, &48, &08, &00, &04, &48, &0C
db &00, &04, &C0, &84, &00, &04, &C0, &84, &00, &0C, &C0, &84, &08, &48, &C0, &C0, &08, &48, &C0, &C0, &08, &48, &C0, &C0, &08, &0C, &0C, &0C, &08, &0C, &0C, &0C
db &08, &00, &00, &00, &00, &00, &00, &00, &00, &F3, &F3, &F3, &A2, &F3, &F3, &F3, &A2, &B7, &3F, &3F, &A2, &B7, &3F, &3F, &A2, &B7, &3F, &3F, &A2, &B7, &3F, &3F
db &A2, &B7, &3F, &3F, &A2, &B7, &3F, &3F, &A2, &B7, &3F, &3F, &A2, &B7, &3F, &3F, &A2, &B7, &3F, &3F, &A2, &B7, &3F, &3F, &A2, &F3, &F3, &F3, &A2, &F3, &F3, &F3
db &A2, &00, &00, &00, &00, &00, &00, &00, &00, &00, &4B, &0A, &00, &00, &4B, &0A, &00, &00, &4B, &0A, &00, &00, &4B, &0A, &00, &0F, &4B, &0F, &0A, &0F, &4B, &0F
db &0A, &C3, &C3, &C3, &82, &C3, &C3, &C3, &82, &0F, &4B, &0F, &0A, &0F, &4B, &0F, &0A, &00, &4B, &0A, &00, &00, &4B, &0A, &00, &00, &4B, &0A, &00, &00, &4B, &0A
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &F3, &F3, &F3, &A2, &30, &30, &30, &20, &CC, &CC, &CC, &88, &64, &CC, &CC, &20, &E6, &CC, &CC, &A2, &E6, &CC, &CC
db &A2, &10, &CC, &98, &00, &51, &CC, &D9, &00, &51, &CC, &D9, &00, &00, &64, &20, &00, &00, &E6, &A2, &00, &00, &E6, &A2, &00, &00, &10, &00, &00, &00, &51, &00
db &00, &00, &00, &00, &00, &00, &14, &00, &00, &00, &14, &00, &00, &00, &78, &28, &00, &00, &78, &28, &00, &14, &F0, &B4, &00, &14, &F0, &B4, &00, &78, &F0, &F0
db &28, &78, &F0, &F0, &28, &14, &F0, &B4, &00, &14, &F0, &B4, &00, &00, &78, &28, &00, &00, &78, &28, &00, &00, &14, &00, &00, &00, &14, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &CF, &8A, &00, &45, &9B, &CF, &00, &CF, &33, &67, &8A, &CF, &33, &67, &8A, &9B, &33, &33, &8A, &9B, &33, &33
db &8A, &9B, &33, &33, &8A, &9B, &33, &33, &8A, &9B, &33, &33, &8A, &9B, &33, &33, &8A, &CF, &33, &67, &8A, &CF, &33, &67, &8A, &45, &9B, &CF, &00, &00, &CF, &8A
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &FC, &00, &54, &A8, &FC, &02, &56, &A8, &56, &02, &56, &02, &56, &A8, &FC, &02, &01, &A9, &A9, &00, &01, &FC, &A9
db &00, &00, &56, &02, &00, &00, &56, &02, &00, &01, &FC, &A9, &00, &01, &A9, &A9, &00, &56, &A8, &FC, &02, &56, &02, &56, &02, &FC, &02, &56, &A8, &FC, &00, &54, &A8

align &100
gfx_menufont:
db &00, &00, &00, &00, &00, &00, &00, &00, &18, &18, &18, &18, &18, &00, &18, &00
db &6C, &6C, &6C, &00, &00, &00, &00, &00, &36, &36, &7F, &36, &7F, &36, &36, &00
db &0C, &3F, &68, &3E, &0B, &7E, &18, &00, &60, &66, &0C, &18, &30, &66, &06, &00
db &38, &6C, &6C, &38, &6D, &66, &3B, &00, &0C, &18, &30, &00, &00, &00, &00, &00
db &0C, &18, &30, &30, &30, &18, &0C, &00, &30, &18, &0C, &0C, &0C, &18, &30, &00
db &00, &18, &7E, &3C, &7E, &18, &00, &00, &00, &18, &18, &7E, &18, &18, &00, &00
db &00, &00, &00, &00, &00, &18, &18, &30, &00, &00, &00, &7E, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &18, &18, &00, &00, &06, &0C, &18, &30, &60, &00, &00
db &3C, &66, &6E, &7E, &76, &66, &3C, &00, &18, &38, &18, &18, &18, &18, &7E, &00
db &3C, &66, &06, &0C, &18, &30, &7E, &00, &3C, &66, &06, &1C, &06, &66, &3C, &00
db &0C, &1C, &3C, &6C, &7E, &0C, &0C, &00, &7E, &60, &7C, &06, &06, &66, &3C, &00
db &1C, &30, &60, &7C, &66, &66, &3C, &00, &7E, &06, &0C, &18, &30, &30, &30, &00
db &3C, &66, &66, &3C, &66, &66, &3C, &00, &3C, &66, &66, &3E, &06, &0C, &38, &00
db &00, &00, &18, &18, &00, &18, &18, &00, &00, &00, &18, &18, &00, &18, &18, &30
db &18, &3C, &7E, &18, &18, &18, &18, &00, &00, &00, &7E, &00, &7E, &00, &00, &00
db &18, &18, &18, &18, &7E, &3C, &18, &00, &3C, &66, &0C, &18, &18, &00, &18, &00
db &3C, &66, &6E, &6A, &6E, &60, &3C, &00, &3C, &66, &66, &7E, &66, &66, &66, &00
db &7C, &66, &66, &7C, &66, &66, &7C, &00, &3C, &66, &60, &60, &60, &66, &3C, &00
db &78, &6C, &66, &66, &66, &6C, &78, &00, &7E, &60, &60, &7C, &60, &60, &7E, &00
db &7E, &60, &60, &7C, &60, &60, &60, &00, &3C, &66, &60, &6E, &66, &66, &3C, &00
db &66, &66, &66, &7E, &66, &66, &66, &00, &7E, &18, &18, &18, &18, &18, &7E, &00
db &3E, &0C, &0C, &0C, &0C, &6C, &38, &00, &66, &6C, &78, &70, &78, &6C, &66, &00
db &60, &60, &60, &60, &60, &60, &7E, &00, &63, &77, &7F, &6B, &6B, &63, &63, &00
db &66, &66, &76, &7E, &6E, &66, &66, &00, &3C, &66, &66, &66, &66, &66, &3C, &00
db &7C, &66, &66, &7C, &60, &60, &60, &00, &3C, &66, &66, &66, &6A, &6C, &36, &00
db &7C, &66, &66, &7C, &6C, &66, &66, &00, &3C, &66, &60, &3C, &06, &66, &3C, &00
db &7E, &18, &18, &18, &18, &18, &18, &00, &66, &66, &66, &66, &66, &66, &3C, &00
db &66, &66, &66, &66, &66, &3C, &18, &00, &63, &63, &6B, &6B, &7F, &77, &63, &00
db &66, &66, &3C, &18, &3C, &66, &66, &00, &66, &66, &66, &3C, &18, &18, &18, &00
db &7E, &06, &0C, &18, &30, &60, &7E, &00

gfx_labelGEM:
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &08, &00, &00, &00, &00, &00
db &00, &04, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &08, &00, &00, &00, &00, &00
db &00, &40, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &80, &00, &00, &00, &00, &00
db &00, &40, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &80, &00, &00, &00, &00, &00
db &00, &40, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &80, &00, &00, &00, &00, &00
db &04, &D5, &C4, &30, &30, &20, &00, &04, &00, &00, &00, &00, &00, &00, &48, &C0, &08, &00, &00, &00, &00
db &04, &C0, &C4, &CC, &CC, &20, &00, &04, &00, &00, &00, &00, &00, &00, &48, &EA, &08, &00, &00, &00, &00
db &00, &60, &CC, &CC, &CC, &98, &00, &40, &00, &00, &00, &00, &00, &00, &00, &85, &00, &00, &05, &0F, &00
db &10, &C8, &CC, &CC, &CC, &CC, &20, &40, &00, &00, &00, &00, &00, &00, &05, &C1, &0A, &00, &0F, &87, &0A
db &10, &C8, &CC, &CC, &CC, &CC, &20, &40, &00, &00, &00, &00, &00, &00, &05, &C1, &0A, &00, &4B, &C3, &0A
db &64, &CC, &CC, &CC, &CC, &CC, &CC, &55, &84, &00, &00, &00, &00, &00, &4B, &C3, &0A, &00, &4B, &C3, &0F
db &64, &CC, &CC, &98, &00, &64, &98, &40, &03, &03, &03, &03, &03, &00, &4B, &C3, &87, &05, &C3, &C3, &87
db &64, &CC, &CC, &20, &00, &44, &98, &42, &FC, &FC, &FC, &FC, &A9, &07, &C3, &C3, &87, &05, &C3, &C3, &87
db &64, &CC, &CC, &20, &00, &44, &98, &42, &FC, &FC, &FC, &FC, &FC, &07, &C3, &C3, &87, &05, &C3, &C3, &87
db &64, &CC, &CC, &00, &00, &44, &98, &56, &FC, &FC, &FC, &FC, &FC, &07, &C3, &C3, &C3, &4B, &C3, &C3, &87
db &64, &CC, &CC, &00, &00, &00, &00, &56, &FC, &FC, &FC, &FC, &FC, &07, &C3, &C3, &C3, &4B, &C3, &C3, &87
db &64, &CC, &CC, &00, &00, &00, &00, &56, &FC, &FC, &FC, &FC, &FC, &07, &C3, &C3, &C3, &4B, &C3, &C3, &87
db &64, &CC, &CC, &00, &00, &00, &00, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &C3, &C3, &C3, &C3, &87
db &64, &CC, &CC, &00, &00, &00, &00, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &C3, &C3, &C2, &C3, &87
db &64, &CC, &CC, &00, &00, &00, &00, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &C3, &C3, &C2, &C3, &87
db &64, &CC, &CC, &00, &10, &30, &00, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &C3, &C3, &C2, &C3, &87
db &64, &CC, &CC, &00, &44, &CC, &20, &56, &FC, &E8, &00, &00, &00, &05, &C3, &C3, &C3, &C3, &D5, &C1, &87
db &64, &CC, &CC, &00, &44, &CC, &20, &56, &FC, &E8, &00, &00, &00, &05, &C3, &C3, &C3, &C3, &C0, &C1, &87
db &64, &CC, &CC, &00, &44, &CC, &98, &56, &FC, &E8, &00, &00, &00, &05, &C3, &C3, &4B, &C3, &4A, &C3, &87
db &64, &CC, &CC, &00, &00, &44, &98, &56, &FC, &D5, &81, &03, &02, &05, &C3, &C3, &0F, &87, &40, &C3, &87
db &64, &CC, &CC, &00, &00, &44, &98, &56, &FC, &C0, &D4, &FC, &A9, &05, &C3, &C3, &0F, &87, &40, &C3, &87
db &64, &CC, &CC, &00, &00, &44, &98, &56, &FC, &E8, &FC, &FC, &FC, &07, &C3, &C3, &0A, &0A, &41, &C3, &87
db &64, &CC, &CC, &00, &00, &44, &98, &56, &FC, &E8, &FC, &FC, &FC, &07, &C3, &C3, &0A, &00, &41, &C3, &87
db &64, &CC, &CC, &00, &00, &44, &98, &56, &FC, &E8, &FC, &FC, &FC, &07, &C3, &C3, &0A, &00, &41, &C3, &87
db &64, &CC, &CC, &00, &00, &44, &98, &56, &FC, &FC, &FC, &FC, &FC, &07, &C3, &C3, &0A, &00, &41, &C3, &87
db &64, &CC, &CC, &20, &00, &40, &98, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &0A, &00, &41, &C3, &87
db &64, &CC, &CC, &98, &00, &40, &98, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &0A, &00, &41, &C3, &87
db &64, &CC, &CC, &98, &00, &40, &98, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &0A, &00, &41, &C3, &87
db &64, &CC, &CC, &CC, &CC, &C0, &C4, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &0A, &00, &41, &C3, &87
db &10, &CC, &CC, &CC, &CC, &C8, &20, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &0A, &00, &41, &C3, &87
db &10, &CC, &CC, &CC, &CC, &C8, &20, &56, &FC, &FC, &00, &00, &00, &05, &C3, &C3, &0A, &00, &41, &C3, &87
db &00, &64, &CC, &CC, &CC, &C8, &00, &56, &FC, &FC, &00, &00, &00, &05, &4B, &87, &0A, &00, &41, &C3, &0F
db &00, &10, &CC, &CC, &98, &20, &00, &56, &FC, &FC, &00, &00, &00, &00, &0F, &0F, &00, &00, &05, &0F, &0A
db &00, &10, &30, &30, &30, &20, &00, &56, &FC, &FC, &00, &00, &00, &00, &0F, &0F, &00, &00, &05, &0F, &0A
db &00, &00, &00, &00, &00, &00, &00, &56, &FC, &FC, &FC, &FC, &FC, &A8, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &56, &FC, &FC, &FC, &FC, &FC, &80, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &56, &FC, &FC, &FC, &FC, &FC, &80, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &03, &FC, &FC, &FC, &FC, &FC, &80, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &01, &03, &03, &03, &03, &E8, &EA, &08, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &01, &03, &03, &03, &03, &E8, &D5, &08, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &40, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &40, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &04, &00, &00, &00, &00, &00, &00, &00
gfx_labelJAM:
db &00, &00, &00, &00, &00, &00, &00, &00, &08, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &80, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &80, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &48, &EA, &CF, &CF, &CF, &00, &00, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &48, &C0, &33, &33, &67, &00, &00, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &45, &91, &33, &33, &33, &8A, &00, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &45, &91, &33, &33, &33, &8A, &00, &00, &00, &00, &00, &00, &00, &00
db &00, &3C, &3C, &3C, &3C, &3C, &00, &45, &33, &33, &33, &33, &8A, &00, &00, &00, &00, &00, &00, &00, &00
db &14, &F0, &F0, &F0, &F0, &3C, &00, &9B, &33, &CF, &9B, &33, &67, &00, &00, &00, &00, &00, &00, &00, &00
db &14, &F0, &F0, &F0, &F0, &B4, &28, &9B, &33, &8A, &11, &33, &67, &00, &00, &00, &00, &00, &00, &00, &00
db &14, &D0, &F0, &F0, &F0, &F0, &28, &33, &33, &8A, &11, &33, &67, &08, &00, &00, &00, &00, &00, &00, &00
db &14, &80, &14, &F0, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &80, &51, &00, &00, &00, &00, &00, &00
db &48, &C0, &1C, &F0, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &80, &B7, &A2, &00, &00, &00, &F3, &00
db &48, &EA, &1C, &78, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &62, &C0, &3F, &A2, &00, &00, &51, &3F, &A2
db &00, &80, &00, &78, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &62, &EA, &3F, &7B, &00, &00, &51, &3F, &A2
db &00, &80, &00, &14, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &D1, &3F, &7B, &00, &00, &B7, &3F, &A2
db &00, &08, &00, &14, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &3F, &3F, &7B, &00, &00, &B7, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &A2, &51, &3F, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &A2, &51, &3F, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &6D, &33, &33, &33, &33, &33, &67, &B7, &3F, &3F, &7B, &51, &3F, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &6D, &33, &33, &33, &33, &33, &67, &B7, &3F, &3F, &7B, &15, &3F, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &6D, &33, &33, &33, &33, &33, &67, &B7, &3F, &3F, &7B, &15, &3F, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &6D, &33, &33, &33, &33, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &6D, &33, &33, &CF, &9B, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &00, &00, &00, &14, &F0, &F0, &39, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &14, &28, &00, &14, &F0, &F0, &68, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &78, &B4, &00, &14, &F0, &F0, &68, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &F0, &B4, &00, &14, &F0, &F0, &C0, &91, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &F0, &B4, &00, &14, &F0, &F0, &D5, &91, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &F0, &B4, &00, &14, &F0, &F0, &68, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &F0, &B4, &00, &14, &F0, &F0, &68, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &F0, &B4, &00, &14, &F0, &F0, &39, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &3F, &3F, &3F, &3F, &7B
db &F0, &B4, &00, &3C, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &B7, &3F, &7B, &B7, &7B, &B7, &3F, &7B
db &F0, &B4, &00, &78, &F0, &F0, &6D, &33, &33, &8A, &11, &33, &67, &B7, &3F, &7B, &E2, &7B, &B7, &3F, &7B
db &F0, &B4, &00, &78, &F0, &B4, &45, &33, &33, &8A, &11, &33, &67, &B7, &3F, &7B, &40, &2A, &B7, &3F, &7B
db &78, &F0, &28, &F0, &F0, &B4, &45, &33, &33, &8A, &11, &33, &67, &B7, &3F, &3F, &D5, &84, &B7, &3F, &7B
db &3C, &F0, &F0, &F0, &F0, &28, &00, &9B, &33, &8A, &11, &33, &CF, &B7, &3F, &3F, &c0, &84, &B7, &3F, &7B
db &14, &F0, &F0, &F0, &F0, &28, &00, &9B, &33, &8A, &11, &33, &8A, &B7, &3F, &7B, &40, &2A, &B7, &3F, &7B
db &14, &F0, &F0, &F0, &B4, &00, &00, &45, &CF, &00, &45, &CF, &8A, &B7, &3F, &7B, &40, &2A, &B7, &3F, &7B
db &00, &3C, &3C, &3C, &3C, &00, &00, &00, &00, &00, &45, &CF, &00, &B7, &3F, &7B, &15, &A2, &B7, &3F, &7B
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &B7, &3F, &7B, &00, &00, &B7, &3F, &7B
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &B7, &3F, &7B, &00, &00, &B7, &3F, &7B
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &B7, &3F, &7B, &00, &00, &B7, &3F, &A2
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &B7, &3F, &7B, &00, &00, &51, &3F, &A2
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &51, &3F, &A2, &00, &00, &51, &3F, &A2
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &51, &3F, &A2, &00, &00, &00, &F3, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &F3, &00, &00, &00, &00, &00, &00



org &91C0

gfx_spriteexplosion1:
db &00, &00, &00, &00, &00, &00, &00, &00, &AA, &55, &00, &00, &00, &02, &45, &AA
db &55, &55, &28, &00, &00, &04, &55, &00, &28, &AA, &00, &00, &00, &45, &BA, &00
db &55, &40, &08, &00, &00, &48, &00, &00, &00, &45, &AA, &AA, &10, &AE, &00, &00
db &00, &55, &55, &00, &55, &00, &00, &00, &00, &55, &01, &AA, &AA, &00, &00, &00
;explosion2
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &55, &00, &00
db &00, &04, &14, &00, &55, &55, &DF, &00, &00, &AA, &00, &00, &20, &14, &BA, &00
db &55, &57, &8A, &00, &00, &55, &00, &00, &04, &AA, &AA, &00, &00, &55, &55, &00
db &55, &00, &00, &00, &00, &55, &04, &00, &00, &00, &00, &00, &00, &00, &00, &00
;explosion3
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &20, &04, &00, &00, &00, &00, &00
db &00, &55, &02, &00, &00, &8A, &00, &00, &00, &00, &10, &00, &00, &28, &00, &00
db &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00, &00

gfx_spritetiles:
ds &1C0, &00
gfx_spriteblacktile:
ds &40, &00

align &100
logic_invisibletopline:
 ds 8 ;This is the "invisible" top line where new tiles emerge from
logic_gametable:
 ds 64
logic_evaluationtable:
 ds 64	;Comes exactly 64 Bytes after GameTable (important offset!)
			;but is still avoiding a HiByte break (ends at &90)
logic_fallingtileslist:
	ds 72	;(80?) also still no HiByte break
gfx_fallingspritesaddresses:
	ds 128

gfx_starpositions: ds 64

	;==============
	;GAME VARIABLES
	;==============

	logic_gamescore: ds 3          ;3 Bytes in BCD -> 6 digits
	toggle_firstload: db 0   ;if the game is loaded for the first time, different stuff happens
	toggle_drawgamefieldnow: db 0 ;is set by the interrupts when CRT is past the field
	toggle_drawstars_readkeyboard: db 0 ;1-draw starfield 2-read keyboard (alternates)
	toggle_gamecursoractive: db 0 ;is the game cursor active i.e. should be drawn and keyboard should be read
	toggle_IR_DrawTimer: db 0 ;should the score be drawn?
	toggle_drawscore: db 0 ;should the score be drawn?
	toggle_menucursoractive: db 0
	logic_cursorposition: db 0
	logic_gameinfo:db 0
	logic_checkmovesindex: db 0
	logic_fallingcolumnsamount: db 0
	logic_gametimer: db 0
	logic_clockfortimer:db 0
	logic_tableanimationtilescounter: db 0
	gfx_tilesvisiblepart: db 0
	gfx_textpositionbackup: db 0,0
	kbd_oldinput: ds 2
	menu_highlighted: db 0
	menu_confirm: db 0
	hlstorage: db 0,0
