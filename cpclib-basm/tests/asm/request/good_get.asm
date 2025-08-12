; http://mads.atari8.info/mads_eng.html

 .get 'file'                    ; load the file into a MADS array
 .get [5] 'file'                ; load the file into an array starting at index 5

 .get 'file',0,3                ; load the file into an array of size 3

 lda #.get[7]                   ; load the value of element 7 of the file array
 adres = .get[2]+.get[3]<<8     ; use bytes 2 and 3 of the file array as an address