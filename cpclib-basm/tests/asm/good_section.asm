; sarcasm inspiration https://www.ecstaticlyrics.com/electronics/Z80/sarcasm/

range $0080, $3FFF, code
range $4000, $7FFF, data

section code
  ld hl, message_1
  call print_message

section data
message_1: db "This is message #1.", $00

section code
  ld hl, message_2
  call print_message

section data
message_2: db "This is message #2.", $00

section code
  print_message: 
	ld a, (hl)
	or a
	ret z
	call 0xbb5a
	inc hl
	jr print_message

	assert section_start("data") ==  0x4000
	assert section_length("data") == 0x4000
	assert section_used("data") == 40