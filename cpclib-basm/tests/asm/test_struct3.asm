  struct point
xx    db 4
yy    db 5
zz    db 6
  endstruct


  	struct triangle
p1 point 1, 2 , 3
p2 point ,,8
p3 point 9 ; third point
	endstruct

my_triangle2: triangle [1, 2, 3], [4, 5, 6], [7, 8 , 9]
