10 REM Bouncing Ball with Gravity
20 MODE 1:INK 0,0:INK 1,24:BORDER 0
30 x=320:y=50
40 vx=5:vy=0
45 g=1
47 r=10
50 GOSUB 1000
60 x=x+vx:y=y+vy:vy=vy+g
70 IF x<=r OR x>=640-r THEN vx=-vx*0.9
80 IF y>=400-r THEN y=400-r:vy=-vy*0.9
90 GOSUB 1000
100 GOTO 60
1000 REM Draw ball
1010 FOR angle=0 TO 360 STEP 30
1020 px=x+r*COS(angle):py=y+r*SIN(angle)
1030 PLOT px,py,1
1040 NEXT angle
1050 RETURN
