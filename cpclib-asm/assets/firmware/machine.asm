
; Action: Loads a program into RAM and then executes it
; Entry: HL contains the address of the routine which is used to load the program
; Exit: Control is handed over to the program and so the routine is not returned from
; Notes: All events, sounds and interrupts are turned off, the firmware indirections are returned to their default settings, and the stack is reset; the routine to run the program should be in the central block of memory, and should obey the following exit conditions:if the program was loaded successfully, then Carry is true, and HL contains the prograrn entry point; if the program failed to load, then Carry is false, and HL is corrupt; in either case, A, BC, DE, IX, IY and the other flags are all corrupt Should the program fail to load, control is returned to the previous foreground program
MC_BOOT_PROGRAM equ #BD13

; Action: Runs a foreground program
; Entry: HL contains the entry point for the program, and C contains the ROM selection number
; Exit: Control is handed over to the prograrn and so the routine is not returned from
MC_START_PROGRAM equ #BD16

; Action: Waits until a frame flyback occurs
; Entry: No entry conditions
; Exit: All registers are preserved
; Notes: When the frame flyback occurs the screen is not being written to and so the screen c~n be manipulated during this period without any flickering or ghosting on the screen
MC_WAIT_FLYBACK equ #BD19

; Action: Sets the screen mode
; Entry: A contains the required mode
; Exit: AF is corrupt, and all other registers are preserved
; Notes: Although this routine changes the screen mode it does not inform the routines which write to the screen that the mode has been changed; therefore these routines will write to the screen as if the mode had not been changed; however as the hardware is now interpreting these signals differently, unusual effects may occur
MC_SET_MODE equ #BD1C

; Action: Sets the screen offset
; Entry: A contains the screen base, and HL contains the screen offset
; Exit: AF is corrupt, and all other registers are preserved
; Notes: As with MC SET MODE, this routine changes the hardware setting without telling the routines that write to the screen; therefore these routines may cause unpredictable effects if called; the default screen base is &C0
MC_SCREEN_OFFSET equ #BD1F

; Action: Sets all the PENs and the border to one colour, so making it seem as if the screen has been cleared
; Entry: DE contains the address of the ink vector
; Exit: AF is corrupt, and all other registers are preserved
; Notes: The ink vector takes the following form:byte 0 - holds the colour for the borderbyte 1 - holds the colour for all of the PENsThe values for the colours are all given as hardware values
MC_CLEAR_INKS equ #BD22

; Action: Sets the colours of all the PENs and the border
; Entry: DE contains the address of the ink vector
; Exit: AF is corrupt, and all other registers are preserved
; Notes: The ink vector takes the following form:byte 0 - holds the colour for the borderbyte 1 - holds the colour for PEN 0... byte 16 - holds the colour for PEN 15. The values for the colours are all given as hardware values; the routine sets all sixteen PEN's
MC_SET_INKS equ #BD25

; Action: Sets the MC WAIT PRINTER indirection to its original routine
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt, and all others are preserved
MC_RESET_PRINTER equ #BD28

; Action: Sends a character to the printer and detects if it is busy for too long (more than 0.4 seconds)
; Entry: A contains the character to be printed - only characters upto ASCII 127 can be printed
; Exit: If the character was sent properly, then Carry is true; if the printer was busy, then Carry is false; in either case, A and the other flags are corrupt, and all other registers are preserved
; Notes: This routine uses the MC WAIT PRINTER indirection
MC_PRINT_CHAR equ #BD2B

; Action: Tests to see if the printer is busy
; Entry: No entry conditions
; Exit: If the printer is busy, then Carry is true; if the printer is not busy, then Carry is false; in both cases, the other flags are corrupt, and all other registers are preserved
MC_BUSY_PRINTER equ #BD2E

; Action: Sends a character to the printer, which must not be busy
; Entry: A contains tlle character to be printed - only characters up to ASCII 127 can be printed
; Exit: Carry is true, A and the other flags are corrupt, and all other registers are preserved
MC_SEND_PRINTER equ #BD31

; Action: Sends data to a sound chip register
; Entry: A contains the register nurnber, and C contains the data to be sent
; Exit: AF and BC are corrupt, and all other registers are preserved
MC_SOUND_REGISTER equ #BD34
