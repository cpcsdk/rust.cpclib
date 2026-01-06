    org $4000
    
    ; Code assembled at $4000
    ld a, 1
    
    ; Use RORG to relocate
    rorg $8000
    	ld b, 2
    
    rend
