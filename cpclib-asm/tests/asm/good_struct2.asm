
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


; force values
my_point1 point 1, 2, 3

; use all default values
my_point2 point

; use default at the end
my_point3 point 1

; use default at the beginning
my_point4 point ,,1


	struct triangle
p1 point 1, 2 , 3
p2 point ,,8
p3 point 9
	endstruct

	assert triangle == 9
	assert triangle.p1 == 0
	assert triangle.p2 == 3
	assert triangle.p3 == 6



my_triangle1 triangle
my_triangle2 triangle [1, 2, 3], , [7, 8 , 9]