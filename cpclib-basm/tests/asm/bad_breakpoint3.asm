
	org 0x1000

NAME="a"
	repeat 200
		NAME=string_push(NAME,"a")
	endr
	breakpoint name=NAME
