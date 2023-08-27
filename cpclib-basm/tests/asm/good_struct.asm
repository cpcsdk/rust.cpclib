
	struct color
r db 1
g db 2
b db 3
	endstruct

	struct point
x db 4
y db 5
	endstruct

col0:	color (void)
pt0:	point (void)

col1:	color 'a', 'b', 'c'
pt1:	point 'd', 'e'

	struct colored_point
col		color 10, 20, 30
pt		point 10, 20
	endstruct


	colored_point (void)