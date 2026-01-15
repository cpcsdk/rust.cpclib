10 REM Conway's Game of Life
20 MODE 1:DIM a(40,25),b(40,25)
30 FOR y=1 TO 24
40 FOR x=1 TO 39
50 r=INT(RND*10)
60 IF r>7 THEN a(x,y)=1
70 IF a(x,y) THEN LOCATE x,y:PRINT CHR$(143);
80 NEXT x:NEXT y
90 REM Main loop
100 FOR y=1 TO 24
110 FOR x=1 TO 39
120 n=a(x-1,y-1)+a(x,y-1)+a(x+1,y-1)+a(x-1,y)+a(x+1,y)+a(x-1,y+1)+a(x,y+1)+a(x+1,y+1)
130 IF a(x,y) THEN b(x,y)=(n=2 OR n=3) ELSE b(x,y)=(n=3)
140 NEXT x:NEXT y
150 FOR y=1 TO 24:FOR x=1 TO 39
160 a(x,y)=b(x,y)
170 LOCATE x,y
180 IF a(x,y) THEN PRINT CHR$(143);
190 NEXT x:NEXT y
200 IF INKEY$="" THEN GOTO 100
