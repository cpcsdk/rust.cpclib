10 REM Mandelbrot Set
20 MODE 0:xmin=-2.5:xmax=1:ymin=-1.25:ymax=1.25
30 FOR py=0 TO 399
40 y0=ymin+(ymax-ymin)*py/399
50 FOR px=0 TO 639
60 x0=xmin+(xmax-xmin)*px/639
70 x=0:y=0:iteration=0:maxiteration=50
80 WHILE x*x+y*y<4 AND iteration<maxiteration
90 xtemp=x*x-y*y+x0
100 y=2*x*y+y0
110 x=xtemp
120 iteration=iteration+1
130 WEND
140 c=iteration MOD 4
150 PLOT px,py,c
160 NEXT px
170 NEXT py
