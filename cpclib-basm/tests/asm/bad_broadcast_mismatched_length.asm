; Broadcasting two lists of different lengths is still a hard error, same
; as before this feature existed.
org 0x4000
db [1,2] + [1,2,3]
