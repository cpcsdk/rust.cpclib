    ; MAP directive
    map $4000
    
label1  #1      ; label1 = $4000, increment by 1
label2  #2      ; label2 = $4001, increment by 2
label3  #1      ; label3 = $4003, increment by 1
    
    db label1, label2, label3
    ret
