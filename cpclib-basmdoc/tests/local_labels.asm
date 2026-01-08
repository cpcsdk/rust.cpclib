;;; Test file for local label handling


;; This is a global label
;; It does some work
main_loop
    ld a, 5
    
;; This is a documented local label under main_loop
;; It decrements A
.loop
    dec a
    jr nz, .loop
    
    ret

;; Another global label
;; It has its own local labels
process_data
    ld b, 10
    
;; Local label for inner loop
;; This one has documentation
.inner
    dec b
    jr nz, .inner
    
    ret

; This is not documented (single ; comment)
.undocumented
    nop
