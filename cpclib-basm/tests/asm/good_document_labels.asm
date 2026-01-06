    ; Labels example
    org $4000
    
start:
    LD A, 5
    JP start     ; Label reference
    
    ret