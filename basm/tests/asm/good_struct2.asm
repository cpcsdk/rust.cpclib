
  ;; Define a 3 fields structure
  struct point
xx    db 4
yy    db 5
zz    db 6
  endstruct

  assert point == 3
  assert point.xx == 0
  assert point.yy == 1
  assert point.zz == 2

 point 1, 2 , 3
 point ,,8
 point 9


; force values
; : after label name allows to disambiguate parser that does not try to check if label is a macro (less errors/faster)
my_point1: point 1, 2, 3

; use all default values
my_point2: point (void)

; use default at the end
my_point3: point 1

; use default at the beginning
my_point4: point ,,1


p1: point 1, 2 , 3
p2: point ,,8
p3: point 9




	struct triangle
p1 point 1, 2 , 3
p2 point ,,8
p3 point 9 ; third point
	endstruct

	assert triangle == 9
	assert triangle.p1 == 0
	assert triangle.p2 == 3
	assert triangle.p3 == 6


my_triangle2: triangle [1, 2, 3], [4, 5, 6], [7, 8 , 9]


 if 0


my_triangle1. triangle

my_triangle2: triangle [1, 2, 3], , [7, 8 , 9]
 endif
