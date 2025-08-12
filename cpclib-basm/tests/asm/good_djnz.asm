    ; http://julien-nevo.com/disark/index.php/examples/
	org #1000
    ld hl,Label1
    ld b,4
Label1
    djnz Label1
    ret