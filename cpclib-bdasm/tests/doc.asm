; to be assembled with basm doc.asm  --binary -o program.bin

    org 0x4000
LD A,0
LD BC,0x1234
JP 0x4000
