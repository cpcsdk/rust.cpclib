
	; Default values are not provided
	; We expect the user to define them all
	struct color
r db 
g db 
b db 
	endstruct

	struct point
x db 
y db 
	endstruct

; Here the double dots are mandatory (otherwhise col2 is considered to be a struct)
col: 	color 1, 2, 3
pt:		point 1, 2

	; Here we enforce the fact there is no default using (void)
	struct colored_point
col		color (void)
pt		point (void)
	endstruct

colpt: colored_point [4, 5, 6], [7, 8]