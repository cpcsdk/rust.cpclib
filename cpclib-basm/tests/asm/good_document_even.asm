    ; Test EVEN directive
    ; EVEN aligns to the next even address (equivalent to ALIGN 2)
    
    org 0x4000
    
    ; Start at even address
    assert $ == 0x4000
    db 1         ; $ is now 0x4001 (odd)
    
    ; EVEN should align to 0x4002
    EVEN
    assert $ == 0x4002
    
    ; Already at even address
    db 2, 3      ; $ is now 0x4004 (even)
    EVEN
    assert $ == 0x4004    ; No change needed
    
    ; Test with odd address
    db 5         ; $ is now 0x4005 (odd)
    EVEN
    assert $ == 0x4006
    
    ; Test comparison with ALIGN 2
    org 0x5000
    db 1
    ALIGN 2
align_result = $
    
    org 0x5000
    db 1
    EVEN
even_result = $
    
    ; Both should give same result
    assert align_result == even_result
