
; Action: Enables or disables disc error messages
; Entry: To enable messages, A holds &00; to disable messages, A holds &FF
; Exit: A holds the previous state, HL and the flags are corrupt, and all others are preserved
; Notes: Enabling and disabling the messages can also be achieved by poking &BE78 with &00 or &FF
BIOS_SET_MESSAGE equ #C033

; Action: Sets the parameters which effect the disc speed
; Entry: HL holds the address of the nine bytes which make up the parameter block
; Exit: AF, BC, DE and HL are corrupt, and all other registers are preserved
; Notes: The parameter block is arranged as followsbytes 0&1 - the motor on time in 20ms units; the default is &0032; the fastest is &0023bytes 2&3 - the motor off time in 20ms units; the default is &00FA; the fastest is &00C8byte 4 - the write off time in l0æs units; the default is &AF; should not be changedbyte 5 - the head settle time in 1ms units; the default is &0F; should not be changedbyte 6 - the step rate time in 1ms units; the default is &0C; the fastest is &0Abyte 7 - the head unload delay; the default is &01; should not be changedbyte 8 - a byte of &03 and this should be left unaltered
BIOS_SETUP_DISC equ #C036

; Action: Sets a format for a disc
; Entry: A holds the type of format that is to be selected
; Exit: AF, BC, DE and HL are corrupt, and all the other registers are preserved
; Notes: To select one of the normal disc formats, the following values should be put into the A registerData format - &C1System format - &41 - Used by CP/MIBM format - &01 - compatible with CP/M-86This routine sets the extended disc parameter block (XDPB) at &A890 to &A8A8 - to set other formats, the XDPB must be altered directly
BIOS_SELECT_FORMAT equ #C039

; Action: Reads a sector from a disc into memory
; Entry: HL holds the address in memory where the sector will be read to, E holds the drive number (&00 for drive A, and &01 for drive B), D holds the track number, and C holds the sector number
; Exit: If the sector was read properly, then Carry is true, A holds 0, and HL is preserved; if the read failed, then Carry is false, A holds an error number, and HL is corrupt; in either case, the other flags are corrupt, and all other registers are preserved
BIOS_READ_SECTOR equ #C03C

; Action: Writes a sector from memory onto disc
; Entry: HL holds the address of memory which will be written to the disc, E holds the drive number (&00 for drive A, and &01 for drive B), D holds the track number, and C holds the sector number
; Exit: If the sector was written properly, then Carry is true, A holds 0, and HL is preserved; if the write failed, then Carry is false, A holds an error number, and HL is corrupt; in either case, the other flags are corrupt, and all other registers are preserved
BIOS_WRITE_SECTOR equ #C03F

; Action: Formats a complete track, inserts sectors, and fills the track with bytes of &E5
; Entry: HL contains the address of the header information buffer which holds the header information blocks, E contains the drive number (&00 for drive A, and &01 for drive B), and D holds the track number
; Exit: if the formatting process was successful, then Carry is true, A holds 0, and HL is preserved; if the formatting process failed, then Carry is false, A holds an error number, and HL is corrupt; in either case, the other flags are corrupt, and all the other registers are preserved
; Notes: The header information block is laid out as followsbyte 0 - holds the track numberbyte 1 - holds the head number (set to zero)byte 2 - holds the sector numberbyte 3 - holds log2(sector size) -7 (usually either &02=512 bytes, or &03=1024 bytes).Header information blocks must be set up contiguously for every sector on the track, and in the same sequence that they are to be laid down (eg &C1, &C6, &C2, &C7, &C3, &C8, &C4, &C9, &C5)
BIOS_FORMAT_TRACK equ #C042

; Action: Moves the disc drive head to the specified track
; Entry: E holds the drive number (&00 for drive A, and &01 for drive B), and D holds the track number
; Exit: If the head was moved successfully, then Carry is true, A holds 0, and HL is preserved; if the move failed, then Carry is false, A holds an error number, and HL is corrupt; in both cases, the other flags are corrupt, and all other registers are preserved
; Notes: There is normally no need to call this routine as READ SECTOR, WRITE SECTOR and FORMAT TRACK automatically move the head to the correct position
BIOS_MOVE_TRACK equ #C045

; Action: Returns the status of the specified drive
; Entry: A holds the drive number (&00 for drive A, and &01 for drive B)
; Exit: If Carry is true, then A holds the status byte, and HL is preserved; if Carry is false, then A is corrupt, and HL holds the address of the byte before the status byte; in either case, the other flags are preserved, and all other registers are preserved
; Notes: The status byte indicates the drive's status as followsif bit 6 is set, then either the write protect is set or the disc is missingif bit 5 is set, then the drive is ready and the disc is fitted (whether the disc is formatted or not)if bit 4 is set, then the head is at track 0
BIOS_GET_STATUS equ #C048

; Action: Sets the number of times the operation is retried in the event of disc error
; Entry: A holds the number of retries required
; Exit: A holds the previous number of retries, HL and the flags are corrupt, and all others are preserved
; Notes: The default setting is &10, and the minimum setting is &01; the number of retries can also be altered by poking &BE66 with the required value
BIOS_SET_RETRY_COUNT equ #C04B
