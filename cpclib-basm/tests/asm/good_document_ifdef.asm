    ; IFDEF/IFNDEF directive
    DEBUG = 1
    
    ifdef DEBUG
        db "Debug mode"
    endif
    
    ifndef RELEASE
        db "Not release"
    endif
    
    ret
