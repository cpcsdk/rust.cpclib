 include "inner://opcodes_first_byte.asm"

 org 0x4000

 db opcode_inc_l
 inc l

 assert memory(0x4000) == memory(0x4001)
