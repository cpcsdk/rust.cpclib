    ; SKIP directive example
    ; Note: SKIP is ORGAMS-only and cannot be tested in standard basm
    ; This demonstrates the intended behavior if ORGAMS mode were enabled:
    ;
    ; org $4000
    ; db $AA          ; Write $AA at $4000
    ; skip 5          ; Skip 5 bytes ($4001-$4005)
    ; db $BB          ; Write $BB at $4006
    ; assert memory($4000) == $AA
    ; assert memory($4006) == $BB
    ;
    ; In standard basm, use DEFS/DS instead (though these fill with a value):
    org $4000
    db $AA          ; Write $AA at $4000
    defs 5, 0       ; Reserve 5 bytes filled with 0
    db $BB          ; Write $BB at $4006
    assert memory($4000) == $AA
    assert memory($4006) == $BB
