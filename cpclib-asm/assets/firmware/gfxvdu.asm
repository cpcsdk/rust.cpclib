
; Action: Initialises the graphics VDU to its default set-up (ie its set-up when the computer is switched on)
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt, and all other registers are preserved
; Notes: Sets the graphics indirections to their defaults, sets the graphic paper to text pen 0 and the graphic pen to text pen 1, reset the graphics origin and move the graphics cursor to the bottom left of the screen, reset the graphics window and write mode to their defaults
GRA_INITIALISE equ #BBBA

; Action: Resets the graphics VDU
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt, and all others are preserved
; Notes: Resets the graphics indirections and the graphics write mode to their defaults
GRA_RESET equ #BBBD

; Action: Moves the graphics cursor to an absolute screen position
; Entry: DE contains the user X-coordinate and HL holds the user Y-coordinate
; Exit: AF, BC, DE and HL are corrupt, and all other registers are preserved
GRA_MOVE_ABSOLUTE equ #BBC0

; Action: Moves the graphics cursor to a point relative to its present screen position
; Entry: DE contains the X-distance to move and HL holds the Y-distance
; Exit: AF, BC, DE and HL are corrupt, and all others are preserved
GRA_MOVE_RELATIVE equ #BBC3

; Action: Gets the graphics cursor's current position
; Entry: No entry conditions
; Exit: DE holds the user X-coordirlate, HL holds the user Y-coordinate, AF is corrupt, and all others nre preserved
GRA_ASK_CURSOR equ #BBC6

; Action: Sets the graphics user origin's screen position
; Entry: DE contains the standard X-coordinate and HL holds the standard Y-coordinate
; Exit: AF, BC, DE and HL are corrupt, and all other registers are preserved
GRA_SET_ORIGIN equ #BBC9

; Action: Gets the graphics user origin's screen position
; Entry: No entry conditions
; Exit: DE contains the standard X-coordinate and HL holds the standard Y-coordinate, and all others are preserved
GRA_GET_ORIGIN equ #BBCC

; Action: Sets the left and right edges of the graphics window
; Entry: DE contains the standard X-coordinate of one edge and HL holds the standard X-coordinate of the other side
; Exit: AF, BC, DE and HL are corrupt, and all the other registers are preserved
; Notes: The default window covers the entire screen and is restored to its default when the mode is changed; used in conjunction with GRA WIN HEIGHT
GRA_WIN_WIDTH equ #BBCF

; Action: Sets the top and bottom edges of the graphics window
; Entry: DE contains the standard Y-coordinate of one side and HL holds the standard Y-coordinate of the other side
; Exit: AF, BC, DE and HL are corrupt, and all others are preserved
; Notes: See GRA WIN WIDTH for further details
GRA_WIN_HEIGHT equ #BBD2

; Action: Gets the left and right edges of the graphics window
; Entry: No entry conditions
; Exit: DE contains the standard X-coordinate of the left edge and HL contains the standard Y-coordinate of the right edge, AF is corrupt, and all other registers are preserved
GRA_GET_W_WIDTH equ #BBD5

; Action: Gets the top and bottom edges of the graphics window
; Entry: No entry conditions
; Exit: DE contains the standard Y-coordinate of the top edge and HL contains the standard Y-coordinate of the bottom edge, AF is corrupt, and all other registers are preserved
GRA_GET_W_HEIGHT equ #BBD8

; Action: Clears the graphics window to the graphics paper colour and moves the cursor back to the user origin
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt, and all other registers are preserved
GRA_CLEAR_WINDOW equ #BBDB

; Action: Sets the graphics PEN
; Entry: A contains the required text PEN number
; Exit: AF is corrupt, and all other registers are preserved
GRA_SET_PEN equ #BBDE

; Action: Gets the graphics PEN
; Entry: No entry conditions
; Exit: A contains the text PEN number, the flags are corrupt, and all other registers are preserved
GRA_GET_PEN equ #BBE1

; Action: Sets the graphics PAPER
; Entry: A contains the required text PEN number
; Exit: AF corrupt, and all others are preserved
GRA_SET_PAPER equ #BBE4

; Action: Gets the graphics PAPER
; Entry: No entry conditions
; Exit: A contains the text PEN number, the flags are corrupt, and all others are preserved
GRA_GET_PAPER equ #BBE7

; Action: Plots a point at an absolute user coordinate, using the GRA PLOT indirection
; Entry: DE contains the user X-coordinate and HL holds the user Y-coordinate
; Exit: AF, BC, DE and HL are corrupt, and all others are preserved
GRA_PLOT_ABSOLUTE equ #BBEA

; Action: Plots a point at a position relative to the current graphics cursor, using the GRA PLOT indirection
; Entry: DE contains the relative X-coordinate and HL contains the relative Y-coordinate
; Exit: AF, BC, DE and HL are corrupt, and all other registers are preserved
GRA_PLOT_RELATIVE equ #BBED

; Action: Moves to an absolute position, and tests the point there using the GRA TEST indirection
; Entry: DE contains the user X-coordinate and HL holds the user Y-coordinate for the point you wish to test
; Exit: A contains the pen at the point, and BC, DE, HL and flags are corrupt, and all others are preserved
GRA_TEST_ABSOLUTE equ #BBF0

; Action: Moves to a position relative to the current position, and tests the point there using the GRA TEST indirection
; Entry: DE contains the relative X-coordinate and HL contains the relative Y-coordinate
; Exit: A contains the pen at the point, and BC, DE, HL and flags are corrupt, and all others are preserved
GRA_TEST_RELATIVE equ #BBF3

; Action: Draws a line from the current graphics position to an absolute position, using GRA LINE
; Entry: DE contains the user X-coordinate and HL holds the user Y-coordinate of the end point
; Exit: AF, BC, DE and HL are corrupt, and all others are preserved
; Notes: The line will be plotted in the current graphics pen colour (may be masked to produce a dotted line on a 6128)
GRA_LlNE_ABSOLUTE equ #BBF6

; Action: Draws a line from the current graphics position to a relative screen position, using GRA LINE
; Entry: DE contains the relative X-coordinate and HL contains the relative Y-coordinate
; Exit: AF, BC, DE and HL are corrupt, and all other registers are preserved
; Notes: See GRA LINE ABSOLUTE above for details of how the line is plotted
GRA_LINE_RELATIVE equ #BBF9
