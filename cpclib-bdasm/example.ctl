; Control file example for bdasm disassembler
; Format: <directive> <parameters>
; 
; Available directives:
;   origin <hex_address>     - Set disassembly origin address
;   skip <decimal_count>     - Skip N bytes at start
;   data <hex_start>-<length> - Mark region as data (not code)
;   label <name>=<hex_addr>  - Define a label at address
;   cpcstring <hex_start>-<length> - Mark as CPC 7-bit string

; Set the disassembly origin
origin 4000

; Skip AMSDOS header if not auto-detected
; skip 128

; Define some labels
label main=4000
label loop=4010
label data_table=4050

; Mark data regions
data 4050-20

; Mark CPC strings (7-bit ASCII with bit 7 on last char)
cpcstring 4070-15
