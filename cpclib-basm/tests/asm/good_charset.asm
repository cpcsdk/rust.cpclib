 org 0x100

 charset "abcdefghijklmnopqrstuvwxyz", 0
 charset "AB", 100
 db "aA"
 ASSERT memory(0x100) == 0x00
 ASSERT memory(0x101) == 100

 org 0x200
 charset
 db "aA"
 ASSERT memory(0x200) == 'a'
 ASSERT memory(0x201) == 'A' 
 

