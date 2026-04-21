; Test proximity labels functionality
; Proximity labels: _, _+, _-
; Inspired by rasm/spasm-ng proximity label system

        org 0x4000

; Test 1: Simple forward reference
        nop
        jr _+           ; Jump to next _ label
        nop
_       nop             ; Target of _+

; Test 2: Simple backward reference
_       nop             ; New _ label
        nop
        jr _-           ; Jump to previous _ label

; Test 3: Multiple proximity labels in sequence
_       ld a, 1         ; First _
        cp 1
        jr nz, _+       ; Jump to next _
        ld a, 2
_       cp 2            ; Second _
        jr nz, _+       ; Jump to next _
        ld a, 3
_       cp 3            ; Third _

; Test 4: Proximity labels in a loop
        ld b, 5
_       dec b           ; Loop start
        jr nz, _-       ; Jump back to loop start

; Test 5: Forward and backward in same block
        jr _+           ; Jump forward
        nop
        nop
_       nop             ; Landing point
        jr _-           ; Jump back to self (infinite loop pattern)

; Test 6: Proximity labels with normal underscored labels
_normal_label
        ld a, 0xFF
        jp _normal_label ; Normal label still works

; Test 7: Complex sequence with mixed references
_       ld hl, 0x8000   ; Label 1
        jr _+           ; Forward to label 2
        nop
_       ld de, 0x4000   ; Label 2
        jr _+           ; Forward to label 3
        nop
_       ld bc, 0x2000   ; Label 3
        ; Can reference backward
        djnz _-         ; Back to label 3

; Test 8: Proximity labels in nested structure
        ld b, 3
_       push bc         ; Outer loop start
        ld c, 2
_       pop hl          ; Inner loop start
        push hl
        dec c
        jr nz, _-       ; Back to inner loop start
        pop bc
        dec b
        jr nz, _-       ; Back to most recent _ (which is STILL the inner loop label!)

; Test 9: Using _ in expressions
_       nop
value   equ $ - _-      ; Should calculate distance from previous _

; Test 10: Verify normal labels with underscore prefix still work
_test_label_1
_test_label_2
        jp _test_label_1
        jp _test_label_2

        ret
