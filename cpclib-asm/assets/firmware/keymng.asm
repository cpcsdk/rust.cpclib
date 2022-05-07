
; Action: Initialises the Key Manager and sets up everything as it is when the computer is first switched on; the key buffer is emptied, Shift and Caps lock are tumed off amd all the expansion and translation tables are reset to normal; also see the routine KM RESET below
; Entry: No entry conditions
; Exit: AF, BC, DE and HL corrupt, and all other registers are preserved
KM_INITIALISE equ #BB00

; Action: Resets the Key Manager; the key buffer is emptied and all current keys/characters are ignored
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt and all other registers are preserved
; Notes: See also KM INITIALISE above. On the 664 or 6128, the key buffer can also be cleared separately by calling the KM FLUSH routine
KM_RESET equ #BB03

; Action: Waits for the next character from the keyboard buffer
; Entry: No entry conditions
; Exit: Carry is true, A holds the character value, the other flags are corrupt, and all other registers are preserved
KM_WAIT_CHAR equ #BB06

; Action: Tests to see if a character is available from the keyboard buffer, but doesn't wait for one to become available
; Entry: No entry conditions
; Exit: If a character was available, then Carry is true, and A contains the character; otherwise Carry is false, and A is corrupt; in both cases, the other registers are preserved
KM_READ_CHAR equ #BB09

; Action: Saves a character for the next use of KM WAIT CHAR or KM READ CHAR
; Entry: A contains the ASCII code of the character to be put back
; Exit: All registers are preserved
KM_CHAR_RETURN equ #BB0C

; Action: Assigns a string to a key code
; Entry: B holds the key code; C holds the length of the string; HL contains the address of the string (must be in RAM)
; Exit: If it is OK, then Carry is true; otherwise Carry is false; in either case, A, BC, DE and HL are corrupt, and all other registers rlre preserved
KM_SET_EXPAND equ #BB0F

; Action: Reads a character from an expanded string of characters
; Entry: A holds an expansion token (ie a key code) and L holds the character position number (starts from 0)
; Exit: If it is OK, then Carry is true, and A holds the character; otherwise Carry is false, and A is corrupt; in either case, DE and flags are corrupt, and the other registers are preserved
KM_GET_EXPAND equ #BB12

; Action: Sets aside a buffer area for character expansion strings
; Entry: DE holds the address of the buffer and HL holds the length of the buffer
; Exit: If it is OK, then Carry is true; otherwise Carry is false; in either case, A, BC, DE and HL are corrupt
; Notes: The buffer must be in the central 32K of RAM and must be at least 49 bytes long
KM_EXP_BUFFER equ #BB15

; Action: Waits for a key to be pressed - this routine does not expand any expansion tokens
; Entry: No entry conditions
; Exit: Carry is true, A holds the character or expansion token, and all other registers are preserved
KM_WAIT_KEY equ #BB18

; Action: Tests whether a key is available from the keyboard
; Entry: No entry conditions
; Exit: If a key is available, then Carry is true, and A contains the character; otherwise Carry is false, and A is corrupt; in either case, the other registers are preserved
; Notes: Any expansion tokens are not expanded
KM_READ_KEY equ #BB1B

; Action: Tests if a particular key (or joystick direction or button) is pressed
; Entry: A contains the key/joystick nurnber
; Exit: If the requested key is pressed, then Zero is false; otherwise Zero is true for both, Carry is false A and HL are corrupt. C holds the Sbift and Control status and others are preserved
; Notes: After calling this, C will hold the state of shift and control - if bit 7 is set then Control was pressed, and if bit 5 is set then Shift was pressed
KM_TEST_KEY equ #BB1E

; Action: Gets the state of the Shift and Caps locks
; Entry: No entry conditions
; Exit: If L holds &FF then the shift lock is on, but if L holds &00 then the Shift lock is off; if H holds &FF then the caps lock is on, and if H holds &00 then the Caps lock is off; whatever the outcome, all the other registers are preserved
KM_GET_STATE equ #BB21

; Action: Reads the present state of any joysticks attached
; Entry: No entry conditions
; Exit: H and A contains the state of joystick 0, L holds that state of joystick 1, and all others are preserved
; Notes: The joystick states are bit significant and are as followsBit 0 - Up Bit1 - Down Bit2 - Left Bit3 - Right Bit4 - Fire2 Bit5 - Fire1 Bit6 - Spare Bit7 - Always zeroThe bits are set when the corresponding buttons or directions are operated
KM_GET_JOYSTICK equ #BB24

; Action: Sets the token or character that is assigned to a key when neither Shift nor Control are pressed
; Entry: A contains the key number and B contains the new token or character
; Exit: AF and HL are corrupt, and all other registers are preserved
; Notes: Special values for B are as follows&80 to &9F - these values correspond to the expansion tokens&FD - this causes the caps lock to toggle on and off&FE - this causes the shift lock to toggle on and off&FF - causes this key to be ignored
KM_SET_TRANSLATE equ #BB27

; Action: Finds out what token or character will be assigned to a key when neither Shift nor Control are pressed
; Entry: A contains the key number
; Exit: A contains the token/character that is assigned, HL and flags are corrupt, and all others are preserved
; Notes: See KM SET TRANSLATE for special values that can be returned
KM_GET_TRANSLATE equ #BB2A

; Action: Sets the token or character that will be assigned to a key when Shift is pressed as well
; Entry: A contains the key number and B contains the new token or character
; Exit: AF and HL are corrupt, and all others are preserved
; Notes: See KM SET TRANSLATE for special values that can be set
KM_SET_SHIFT equ #BB2D

; Action: Finds out what token/character will be assigned to a key when Shift is pressed as well
; Entry: A contains the key number
; Exit: A contains the token/character that is assigned, HL and flags are corrupt, and all others are preserved
; Notes: See KM SET TRANSLATE for special values that can be returned
KM_GET_SHIFT equ #BB30

; Action: Sets the token or character that will be assigned to a key when Control is pressed as well
; Entry: A contains the key number and B contains the new token/character
; Exit: AF and HL are corrupt, and all others are preserved
; Notes: See KM SET TRANSLATE for special values that can be set
KM_SET_CONTROL equ #BB33

; Action: Finds out what token or character will be assigned to a key when Control is pressed as well
; Entry: A contains the key number
; Exit: A contains the token/character that is assigned, HL and flags are corrupt and all others are preserved
; Notes: See KM SET TRANSLATE for special values that can be set
KM_GET_CONTROL equ #BB36

; Action: Sets whether a key may repeat or not
; Entry: A contains the key number B contains &00 if there is no repeat and &FF is it is to repeat
; Exit: AF, BC and HL are corrupt, and all others are preserved
KM_SET_REPEAT equ #BB39

; Action: Finds out whether a key is set to repeat or not
; Entry: A contains a key number
; Exit: If the key repeats, then Zero is false; if the key does not repeat, then Zero is true; in either case, A, HL and flags are corrupt, Carry is false, and all other registers are preserved
KM_GET_REPEAT equ #BB3C

; Action: Sets the time that elapses before the first repeat, and also set the repeat speed
; Entry: H contains the time before the first repeat, and L holds the time between repeats (repeat speed)
; Exit: AF is corrupt, and all others are preserved
; Notes: The values for the times are given in 1/5Oth seconds, and a value of 0 counts as 256
KM_SET_DELAY equ #BB3F

; Action: Finds out the time that elapses before the first repeat and also the repeat speed
; Entry: No entry conditions
; Exit: H contains the time before the first repeat, and L holds the time between repeats, and all others are preserved
KM_GET_DELAY equ #BB42

; Action: Arms the Break mechanism
; Entry: DE holds the address of the Break handling routine, C holds the ROM select address for this routine
; Exit: AF, BC, DE and HL are corrupt, and all the other registers are preserved
KM_ARM_BREAK equ #BB45

; Action: Disables the Break mechanism
; Entry: No entry conditions
; Exit: AF and HL are corrupt, and all the other registers are preserved
KM_DISARM_BREAK equ #BB48
