

macro with_label
@my_label
	jp @my_label
endm



macro with_label2
@my_label
	ld hl, @my_label
endm


macro with_label3
@my_label
	ld hl, 20 + @my_label
endm

macro with_label4
@my_label = 5
	ld hl, 20 + @my_label
endm

macro with_label5
@my_label equ 5
	ld hl, 20 + @my_label
endm


macro with_label6
@my_label equ 5
	ld hl, 20 + @my_label
endm


macro with_arg arg
	ld hl, {arg}
endm

macro with_label7
@my_label equ 5
	with_arg({eval}@my_label) ; Here @my_label is local to with_label and cannot be given as it is to with_arg
endm





with_label(void)
with_label2(void)
with_label3(void)
with_label4(void)
with_label5(void)
with_label6(void)
with_label7(void)