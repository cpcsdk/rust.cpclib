; bndbuild --direct -- rasm breakpoint.asm -o breakpoint_rasm -rasm
; bndbuild --direct -- basm breakpoint.asm  --remu breakpoint_basm.remu -o /dev/null

org 0

; on doit avoir 3 breakpoints
BREAKPOINT EXEC,ADDR=100
BREAKPOINT name="breakpoint1",condition="C3H<>22",addr=$
BREAKPOINT addr=2,mask=0x00ff