
; Action: Initialises the cassette manager
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt, and all the other registers are preserved
; Notes: Both read and write streams are closed; tape messages are switched on; the default speed is reselected
CAS_INITIALISE equ #BC65

; Action: Sets the speed at which the cassette manager saves programs
; Entry: HL holds the length of `half a zero' bit, and A contains the amount of precompensation
; Exit: AF and HL are corrupt
; Notes: The value in HL is the length of time that half a zero bit is written as; a one bit is twice the length of a zero bit; the default values (ie SPEED WRITE 0) are 333 microseconds (HL) and 25 microseconds (A) for SPEED WRITE 1, the values are given as 107 microseconds and 50 microseconds respectiveIy
CAS_SET_SPEED equ #BC68

; Action: Enables or disables the display of cassette handling messages
; Entry: To enable the messages then A must be 0, otherwise the messages are disabled
; Exit: AF is corrupt, and all other registers are preserved
CAS_NOISY equ #BC6B

; Action: Switches on the tape motor
; Entry: No entry conditions
; Exit: If the motor operates properly then Carry is true; if ESC was pressed then Carry is false; in either case, A contains the motor's previous state, tbe flags are corrupt, and all others are preserved
CAS_START_MOTOR equ #BC6E

; Action: Switches off the tape motor
; Entry: No entry conditions
; Exit: If the motor turns off then Carry is true; if ESC was pressed then Carry is false; in both cases, A holds tbe motor's previous state, the other flags are corrupt, all others are preserved
CAS_STOP_MOTOR equ #BC71

; Action: Resets the tape motor to its previous state
; Entry: A contains the previous state of the motor (eg from CAS START MOTOR or CAS STOP MOTOR)
; Exit: If the motor operates properly then Carry is true; if ESC was pressed then Carry is false; in all cases, A and the other flags are corrupt and all others are preserved
CAS_RESTORE_MOTOR equ #BC74

; Action: Opens an input buffer and reads the first block of the file
; Entry: B contains the length of the filename, HL contains the filename's address, and DE contains the address of the 2K buffer to use for reading the file
; Exit: If the file was opened successfully, then Carry is true, Zero is false, HL holds the address of a buffer contauling the file header data, DE holds the address of the destination for the file, BC holds the file length, and A holds the file type; if the read stream is already open then Carry and Zero are false, A contains an error nurnber (664/6128 only) and BC, DE and HL are corrupt; if ESC was pressed by the user, then Carry is false, Zero is true, A holds an error number (664/6128 only) and BC, DE and HL are corrupt; in all cases, IX and the other flags are corrupt, and the others are preserved
; Notes: A filename of zero length means `read the neXt file on the tape'; the stream remains open until it is closed by either CAS IN CLOSE or CAS IN ABANDON
; Disc: Similar to tape except that if there is no header on the file, then a fake header is put into memory by this routine
CAS_IN_OPEN equ #BC77

; Action: Closes an input file
; Entry: No entry conditions
; Exit: If the file was closed successfully, then Carry is true and A is corrupt; if the read stream was not open, then Carry is false, and A holds an error code (664/6128 only); in both cases, BC, DE, HL and the other flags are all corrupt
; Disc: All the above applies, but also if the file failed to close for any other reason, then Carry is false, Zero is true and A contains an error number; in all cases the drive motor is turned off immediately
CAS_IN_CLOSE equ #BC7A

; Action: Abandons an input file
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt, and all others are preserved
; Disc: All the above applies for the disc routine
CAS_IN_ABANDON equ #BC7D

; Action: Reads in a single byte from a file
; Entry: No entry conditions
; Exit: If a byte was read, then Carry is true, Zero is false, and A contains the byte read from the file; if the end of file was reached, then Carry and Zero are false, A contains an error number (664/6128 only) or is corrupt (for the 464); if ESC was pressed, then Carry is false, Zero is true, and A holds an error number (664/6128 only) or is corrupt (for the 464); in all cases, IX and the other flags are corrupt, and all others are preserved
; Disc: All the above applies for the disc routine
CAS_IN_CHAR equ #BC80

; Action: Reads an entire file directly into memory
; Entry: HL contains the address where the file is to be placed in RAM
; Exit: If the operation was successful, then Carry is true, Zero is false, HL contains the entry address and A is corrupt; if it was not open, then Carry and Zero are both false, HL is corrupt, and A holds an error code (664/6128) or is corrupt (464); if ESC was pressed, Carry is false, Zero is true, HL is corrupt, and A holds an error code (664/6128 only); in all cases, BC, DE and IX and the other flags are corrupt, and the others are preserved
; Notes: This routine cannot be used once CAS IN CHAR has been used
; Disc: All the above applies to the disc routine
CAS_IN_DIRECT equ #BC83

; Action: Puts the last byte read back into the input buffer so that it can be read again at a later time
; Entry: No entry conditions
; Exit: All registers are preserved
; Notes: The routine can only return the last byte read and at least one byte must have been read
; Disc: All the above applies to the disc routine
CAS_RETURN equ #BC86

; Action: Tests whether the end of file has been encountered
; Entry: No entry conditions
; Exit: If the end of file has been reached, then Carry and Zero are false, and A is corrupt; if the end of file has not been encountered, then Carry is true, Zero is false, and A is corrupt; if ESC was pressed then Carry is false, Zero is true and A contains an error number (664/6128 only); in all cases, IX and the other flags are corrupt, and all others are preserved
; Disc: All the above applies to the disc routine
CAS_TEST_EOF equ #BC89

; Action: Opens an output file
; Entry: B contains the length of the filename, HL contains the address of the filename, and DE holds the address of the 2K buffer to be used
; Exit: If the file was opened correctly, then Carry is true, Zero is false, HL holds the address of the buffer containing the file header data that will be written to each block, and A is corrupt; if the write stream is already open, then Carry and Zero are false, A holds an error nurnber (66~/6128) and HL is corrupt; if ESC was pressed then Carry is false, Zero is true, A holds an error number (664/6128) and HL is corrupt; in all cases, BC, DE, IX and the other flags are corrupt, and the others are preserved
; Notes: The buffer is used to store the contents of a file block before it is actually written to tape
; Disc: The same as for tape except that the filename must be present in its usual AMSDOS format
CAS_OUT_OPEN equ #BC8C

; Action: Closes an output file
; Entry: No entry conditions
; Exit: If the file was closed successfully, then Carry is true, Zero is false, and A is corrupt; if the write stream was not open, then Carry and Zero are false and A holds an error code (664/6128 only); if ESC was pressed then Carry is false, Zero is true, and A contains an error code (664/6128 only); in all cases, BC, DE, HL, IX and the other flags are all corrupt
; Notes: The last block of a file is written only when this routine is called; if writing the file is to be abandoned, then CAS OUT ABANDON should be used instead
; Disc: All the above applies to the disc routine
CAS_OUT_CLOSE equ #BC8F

; Action: Abandons an output file
; Entry: No entry conditions
; Exit: AF, BC, DE and HL are corrupt, and all others are preserved
; Notes: When using this routine, the current last block of the file is not written to the tape
; Disc: Similar to the tape routine; if more than 16K of a file has been written to the disc, then the first 16K of the file will exist on the disc with a file extension of .$$$ because each 16K section of the file requires a separate directory entry
CAS_OUT_ABANDON equ #BC92

; Action: Writes a single byte to a file
; Entry: A contains the byte to be written to the file output buffer
; Exit: If a byte was written to the buffer, then Carry is true, Zero is false, and A is corrupt; if the file was not open, then Carry and Zero are false, and A contains an error number (664/6128 only) or is corrupt (on the 464); if ESC was pressed, then Carry is false, Zero is true, and A contains an error number (664/6128 only) or it is corrupt (on the 464); in all cases, IX and the other flags are corrupt, and all others are preserved
; Notes: If the 2K buffer is full of data then it is written to the tape before the new character is placed in the buffer; it is important to call CAS OUT CLOSE when all the data has been sent to the file so that the last block is written to the tape
; Disc: All the above applies to the disc routine
CAS_OUT_CHAR equ #BC95

; Action: Writes an entire file directly to tape
; Entry: HL contains the address of the data which is to be written to tape, DE contains the length of this data, BC contains the e~ecution address, and A contains the file type
; Exit: If the operation was successful, then Carry is true, Zero is false, and A is corrupt; if the file was not open, Carry and Zero are false, A holds an error number (664/6128) or is corrupt (464); if ESC was pressed, then Carry is false, Zero is true, and A holds an error code (664/6128 only); in all cases BC, DE, HL, IX and the other flags are corrupt, and the others are preserved
; Notes: This routine cannot be used once CAS OUT CHAR has been used
; Disc: All the above applies to the disc routine
CAS_OUT_DIRECT equ #BC98

; Action: Creates a catalogue of all the files on the tape
; Entry: DE contains the address of the 2K buffer to be used to store the information
; Exit: If the operation was successful, then Carry is true, Zero is false, and A is corrupt; if the read stream is already being used, then Carry and Zero are false, and A holds an error code (664/6128 or is corrupt (for the 464); in all cases, BC, DE, HL, IX and the other flags are corrupt and all others are preserved
; Notes: This routine is only left when the ESC key is pressed (cassette only) and is identical to BASIC's CAT command
; Disc: All tbe above applies, except that a sorted list of files is displayed; system files are not listed by this routine
CAS_CATALOG equ #BC9B

; Action: Writes data to the tape in one long file (ie not in 2K blocks)
; Entry: HL contains the address of the data to be written to tape, DE contains the length of the data to be written, and A contains the sync character
; Exit: If the operation was successful, then Carry is true and A is corrupt; if an error occurred then Carry is false and A contains an error code; in both cases, BC, DE, HL and lX are corrupt, and all other registers are preserved
; Notes: For header records the sync character is &2C, and for data it is &16; this routine starts and stops the cassette motor and also tums off interrupts whilst writing data
CAS_WRITE equ #BC9E

; Action: Reads data from the tape in one long file (ie as originally written by CAS WRITE only)
; Entry: HL holds the address to place the file, DE holds the length of the data, and A holds the expected sync character
; Exit: If the operation was successful, then Carry is true and A is corrupt; if an error occurred then Carry is false and A contains an error code; in both cases, BC, DE, HL and IX are corrupt, and all other registers are preserved
; Notes: For header records the sync character is &2C, and for data it is &16; this routine starts and stops the cassette motor and turns off interrupts whilst reading data
CAS_READ equ #BCA1
