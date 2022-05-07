
; Action: Turns the shift and caps locks on and off
; Entry: H contains the caps lock state, and L contains the shift lock state
; Exit: AF is corrupt, and all others are preserved
; Notes: In this routine, &00 means turned off, and &FF means turned on
KM_SET_LOCKS equ #BD3A

; Action: Empties the key buffer
; Entry: No entry conditions
; Exit: AF is corrupt, and all other registers are preserved
; Notes: This routine also discards any current expansion string
KM_FLUSH equ #BD3D

; Action: Gets the VDU and cursor state
; Entry: No entry conditions
; Exit: A contains the VDU and cursor state, the flags are corrupt, and all others are preserved
; Notes: The value in the A register is bit significant, as follows:if bit 0 is set, then the cursor is disabled, otherwise it is enabledif bit 1 is set, then the cursor is turned off, otherwise it is onif bit 7 is set, then the VDU is enabled, otherwise it is disabled
TXT_ASK_STATE equ #BD40

; Action: Sets the graphics VDU to its default mode
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt, and all other registers are preserved
; Notes: Sets the background to opaque, the first point of line is plotted, lines aren't dotted, and the write mode is force
GRA_DEFAULT equ #BD43

; Action: Sets the graphics background mode to either opaque or transparent
; Entry: A holds zero if opaque mode is wanted, or holds &FF to select transparent mode
; Exit: All registers are preserved
GRA_SET_BACK equ #BD46

; Action: Sets whether the first point of a line is plotted or not
; Entry: A holds zero if the first point is not to be plotted, or holds &FF if it is to be plotted
; Exit: All registers are preserved
GRA_SET_FIRST equ #BD49

; Action: Sets how the points in a line are plotted - ie defines whether a line is dotted or not
; Entry: A contains the line mask that will be used when drawing lines
; Exit: All registers are preserved
; Notes: The first point in the line corresponds to bit 7 of the line mask and after bit 0 the mask repeats; if a bit is set then that point will be plotted; the mask is always applied from left to right, or from bottom to top
GRA_SET_LINE_MASK equ #BD4C

; Action: Converts user coordinates into base coordinates
; Entry: DE contains the user X coordinate, and HL contains the user Y coordinate
; Exit: DE holds the base X coordinate, and HL holds the base Y coordinate, AF is corrupt, and all others are preserved
GRA_FROM_USER equ #BD4F

; Action: Fills an area of the screen starting from the current graphics position and extending until it reaches either the edge of the window or a pixel set to the PEN
; Entry: A holds a PEN to fill with, HL holds the address of the buffer, and DE holds the length of the buffer
; Exit: If the area was filled properly, then Carry is true; if the area was not filled, then Carry is false; in either case, A, BC, DE, HL and the other flags are corrupt, and all others are preserved
; Notes: The buffer is used to store complex areas to fill, which are remembered and filled when the basic shape has been done; each entry in the buffer uses seven bytes and so the more complex the shape the larger the buffer; if it runs out of space to store these complex areas, it will fill what it can and then return with Carry false
GRA_FILL equ #BD52

; Action: Sets the screen base and offset without telling the hardware
; Entry: A contains the screen base, and HL contains the screen offset
; Exit: A contains the masked screen base, and HL contains the masked screen offset, the flags are corrupt, and all other registers are preserved
SCR_SET_POSITION equ #BD55

; Action: Sets how ASCII characters will be translated before being sent to the printer
; Entry: HL contains the address of the table
; Exit: If the table is too long, then Carry is false (ie more than 20 entries); if the table is correctly set out, then Carry is true; in either case, A, BC, DE, HL and the other flags are corrupt, and all others are preserved
; Notes: The first byte in the table is the number of entries; each entry requires two bytes, as follows:byte 0 - the character to be translated byte 1 - the character that is to be sent to the printer If the character to be sent to the printer is &FF, then the character is ignored and nothing is sent
MC_PRINT_TRANSLATION equ #BD58
