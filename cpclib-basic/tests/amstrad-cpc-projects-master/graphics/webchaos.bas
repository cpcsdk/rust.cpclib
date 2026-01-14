10 MODE 1
20 BORDER 0
30 INK 0,0:INK 1,6:INK 2,18:INK 3,26
40 CLS
50 DEG
60 x=320:y=200
70 angle=RND*360
80 length=20
90 WHILE INKEY$=""
100 GRAPHICS PEN (RND*2)+1
110 MOVE x,y
120 angle=angle+(RND*60-30)
130 length=length+(RND*10-5)
140 IF length<5 THEN length=5
150 x2=x+length*COS(angle)
160 y2=y+length*SIN(angle)
170 IF x2<0 OR x2>639 THEN angle=180-angle
180 IF y2<0 OR y2>399 THEN angle=-angle
190 x2=MIN(MAX(x2,0),639)
200 y2=MIN(MAX(y2,0),399)
210 DRAW x2,y2
220 x=x2:y=y2
230 WEND
240 CALL &BB18
