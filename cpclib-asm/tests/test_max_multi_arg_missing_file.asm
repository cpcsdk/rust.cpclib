; Test for multi-argument max() in equ
; This should work on both Linux and Windows

SECOND_FILE_CRTC0_LEN equ list_len(load("non_existent_file.asm")) ; This line should make crash the assembling
SECOND_FILE_CRTC1_LEN equ 20
SECOND_FILE_CRTC2_LEN equ 30
SECOND_FILE_CRTC3_LEN equ 40
SECOND_FILE_CRTC4_LEN equ 50

SECOND_FILE_MAX_LEN equ max(
    SECOND_FILE_CRTC0_LEN,
    SECOND_FILE_CRTC1_LEN,
    SECOND_FILE_CRTC2_LEN,
    SECOND_FILE_CRTC3_LEN,
    SECOND_FILE_CRTC4_LEN
)

; Use the value to force evaluation
org 0x8000
ld a, SECOND_FILE_MAX_LEN
assert SECOND_FILE_MAX_LEN = 50, "Max value should be 50"
