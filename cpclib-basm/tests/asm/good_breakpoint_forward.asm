	; BREAKPOINT directives naming a label defined further on
	org #4000
start
	breakpoint later
	breakpoint type=mem, access=write, addr=buffer
	ld a,1
later
	ret
buffer
	defb 0
