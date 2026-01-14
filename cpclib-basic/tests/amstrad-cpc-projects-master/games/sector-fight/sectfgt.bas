10 ' Sector Fight
20 RANDOMIZE TIME
30 ON ERROR GOTO 3430
40 ON BREAK GOSUB 3480
50 MODE 1:INK 0,0:INK 1,26:PAPER 0:PEN 1:BORDER 0
60 '
70 CLEAR INPUT:LOCATE 1,1:PRINT "Select mode (0-Mode0 1-Mode1)";
80 a$="":WHILE a$="":a$=INKEY$:WEND
90 IF a$<>"0" AND a$<>"1" THEN a$="1"
100 smd=VAL(a$):cols=20*(2^smd):rows=25:hcols=INT(cols/2):hrows=INT(rows/2)
110 CLEAR INPUT:LOCATE 1,3:PRINT"Auto Pause at End of Turn":PRINT"Default No (Y/N)";
120 a$="":WHILE a$="":a$=INKEY$:WEND
130 IF UPPER$(a$)="Y" THEN ps=1 ELSE ps=0
140 CLEAR INPUT:LOCATE 1,6:PRINT"1 CPU Match (default) 2 vs Human";
150 a$="":WHILE a$="":a$=INKEY$:WEND
160 IF a$<>"1" AND a$<>"2" THEN a$="1"
170 h=VAL(a$):IF h=1 THEN h=0 ELSE h=1:' h 0 means no human
180 '
190 ' Screen initialization and colors
200 ' cbg bg color, cl1,cl2 cpu colors, ctx text color
210 INK 0,0:INK 1,2:INK 2,6:INK 3,26:cbg=0:cl1=1:cl2=2:ctx=3:PAPER cbg:BORDER cbg:PEN ctx:MODE smd
220 ms$="Loading...":tmp=INT(LEN(ms$)/2):tmpx=MAX(1,hcols-tmp):LOCATE tmpx,hrows:PEN ctx:PRINT ms$
230 '
240 ' Setup blocks, cpu1 cpu2 empty block, highlight block
250 b1$=CHR$(207):b2$=CHR$(207):SYMBOL 240,0,0,60,60,60,60,0,0:eb$=CHR$(32):hb$=CHR$(240)
260 '
270 ' Initialize cpu/player stats
280 ' position, sum xy, avg xy, min and max xy
290 ' selected pos., last occupied pos.
300 id1=1:id2=2
310 ial=7:ist=0:ism=1:ivg=2:imn=3:imx=4:isl=5:ilt=6:icn=7
320 DIM st(2,ial,1):DIM st$(ial):RESTORE 3500:FOR i=0 TO ial:READ st$(i):NEXT
330 '
340 ' Battle probabilities array - btl():
350 ' 0-2 friendly block min,max,avg
360 ' 3-5 opposing min,max,avg
370 bsz=5:frn=0:frx=1:fra=2:opn=3:opx=4:opa=5
380 DIM btl(bsz)
390 btl(frn)=0.1:btl(frx)=0.2:btl(fra)=(btl(frn)+btl(frx))/2
400 btl(opn)=-0.2:btl(opx)=-0.1:btl(opa)=(btl(opn)+btl(opx))/2
410 '
420 ' Attacker and defense thresholds
430 attthres=0.3:defthres=0.3
440 '
450 ' Cumulative personality probabilities: pnprb()
460 ' 1 norm: 0.25 prob, 2 att: 0.25, 3 rnd: 0.25, 4 def: 0.25
470 psz=4:pnrm=0:patt=1:prnd=2:pdef=3:phmn=4
480 DIM pnprb(psz):pnprb(pnrm)=0.25:pnprb(patt)=pnprb(pnrm)+0.25:pnprb(prnd)=pnprb(patt)+0.25:pnprb(pdef)=pnprb(prnd)+0.25
490 '
500 ' Personality names pn$()
510 ' Normal, 1 Attacker, 2 Random, 3 Defender
520 DIM pn$(psz)
530 IF smd=0 THEN RESTORE 3520 ELSE RESTORE 3510
540 FOR i=0 TO psz:READ pn$(i):NEXT
550 '
560 ' Assign personalities based on personality probabilities: pnprb()
570 r=RND:IF r<pnprb(pnrm) THEN pn1=pnrm ELSE IF r<pnprb(patt) THEN pn1=patt ELSE IF r<pnprb(prnd) THEN pn1=prnd ELSE pn1=pdef
580 r=RND:IF r<pnprb(pnrm) THEN pn2=pnrm ELSE IF r<pnprb(patt) THEN pn2=patt ELSE IF r<pnprb(prnd) THEN pn2=prnd ELSE pn2=pdef
590 IF h=1 THEN pn2=phmn
600 '
610 ' Wait for key press
620 CLS:IF smd=0 THEN ms$="Press key to start" ELSE ms$="Press any key to start":GOSUB 2980:CLS
630 '
640 ' print personalities
650 IF h=0 THEN id1$="CPU 1":id2$="CPU 2" ELSE id1$="CPU":id2$="You"
660 c1$=id1$+": "+pn$(pn1):c2$=id2$+": "+pn$(pn2)
670 c1x=MAX(LEN(c1$),LEN(c2$))+2:c1y=1:c2x=c1x:c2y=rows' aligned x location to print status on cpu rows
680 sx=1:sy=2:' location of status line
690 LOCATE 1,1:PEN cl1:PRINT c1$
700 LOCATE 1,rows:PEN cl2:PRINT c2$
710 '
720 ' Initialize grid and starting positions
730 LOCATE sx,sy:PAPER ctx:PEN cbg:PRINT STRING$(cols," "):LOCATE sx,sy:PRINT"Initializing...":PAPER cbg:PEN ctx
740 GOSUB 3100:DIM grd(gw,gh):grd(0,0)=-1:' setup grid dimensions
750 GOSUB 3020:' draw grid border
760 GOSUB 3170:' define and draw starting positions and stats
770 '
780 'lists of valid moves for player 1 and 2
790 'element 0 of bls store the counter of valid moves e.g. bls(0,0,0) counter of potential valid moves for player 1 while bls(1,0,0) for player 2
800 blmax=(gw+gh)*2:DIM bls(1,blmax,1)' REM bls(0) stores list for player id1, bls(2) for player id2
810 '
820 DIM vm(8,1):' array to store all potential 8 valid moves around a given block, first element at pos 0 stores the count of valid moves e.g.e vm(0,0)=5
830 '
840 ' Initialize valid block lists for players
850 tmpid=id1:tmpopp=id2:GOSUB 2050
860 IF h=0 THEN tmpid=id2:tmpopp=id1:GOSUB 2050:'if it is a cpu vs cpu match initialize cpu 2 list has well
870 '
880 ' clear initialization message
890 LOCATE sx,sy:PRINT STRING$(cols," ")
900 '
910 c1=st(id1,icn,0):c2=st(id2,icn,0)
920 GOSUB 2930:' print block counts
930 '
940 ' Game LOOP
950 turn=1:trn=0
960 WHILE c1+c2<gw*gh
970 '
980 ' CPU turn
990 bx=0:by=0:tx=0:ty=0:trn=trn+1:trs=0
1000 IF turn=1 THEN id=id1:opp=id2:cpuclr=cl1:oppclr=cl2:b$=b1$:pn=pn1:clx=c1x:cly=c1y
1010 IF turn=2 THEN id=id2:opp=id1:cpuclr=cl2:oppclr=cl1:b$=b2$:pn=pn2:clx=c2x:cly=c2y
1020 LOCATE sx,sy:PRINT STRING$(cols," ")
1030 c1=st(id1,icn,0):c2=st(id2,icn,0):prg=ROUND((c1+c2)/(gwh)*100,2)
1040 PEN ctx:LOCATE sx,sy:PRINT "Turn";:PEN cpuclr:PRINT trn;:PEN ctx:PRINT STR$(prg);"%";
1050 PEN cpuclr:mst$="...":LOCATE clx,cly:PRINT mst$;
1060 '
1070 'Process cpu action based on personality
1080 act=0:ON pn+1 GOSUB 1410,1450,1490,1680,1720
1090 c1=st(id1,icn,0):c2=st(id2,icn,0)
1100 LOCATE clx,cly:PRINT SPC(LEN(mst$));
1110 ' act 0 is no move found, 1 is move, 2 fight won, 3 fight loss
1120 IF act=0 THEN GOTO 1280:'no valid move found, end game
1130 ' print move or fg results to screen
1140 tmpid=id:'used by highlight routine to decide color
1150 IF act=1 THEN hx=tx:hy=ty:GOSUB 2760:SOUND 1,200,20,15:GOSUB 2760:' move highlight, play sound highlight
1160 IF act=2 THEN hx=tx:hy=ty:GOSUB 2760:SOUND 1,142,20,15:SOUND 1,95,20,15:GOSUB 2760:' fight won highlight, play sound highlight
1170 IF act=3 THEN hx=tx:hy=ty:GOSUB 2760:SOUND 1,95,20,15:SOUND 1,125,20,15:GOSUB 2760:' fight lost highlight, play sound highlight
1180 IF act=1 OR act=2 THEN LOCATE ofx+tx,ofy+ty:PEN cpuclr:PRINT b$;
1190 GOSUB 2930:' print block counts
1200 'auto pause only for cpu moves
1210 IF id=id2 AND h=1 THEN 1230
1220 IF ps=1 THEN a$="":FOR i=sx TO cols:LOCATE i,sy:a$=a$+COPYCHR$(#0):NEXT:PAPER cpuclr:PEN ctx:LOCATE sx,sy:PRINT a$:PAPER cbg:PEN cpuclr:CLEAR INPUT:CALL &BB18
1230 IF c1+c2>=gwh OR c1=0 OR c2=0 THEN GOTO 1320
1240 IF turn=1 THEN turn=2 ELSE turn=1
1250 trs=1
1260 WEND
1270 '
1280 ' error: no valid move found
1290 IF smd<>0 THEN ms$="Error: no valid move found" ELSE ms$="Err: no move found"
1300 GOSUB 2980:GOTO 1370:' print error message and exit
1310 '
1320 ' end game
1330 IF smd<>0 THEN ms$="Game Over: "+id1$+":"+STR$(c1)+" "+id2$+":"+STR$(c2) ELSE ms$="Game Over:"+MID$(STR$(c1),2)+"/"+MID$(STR$(c2),2)
1340 GOSUB 2980
1350 IF c1>c2 THEN ms$="CPU 1 wins!" ELSE IF c1<c2 THEN ms$="CPU 2 wins!" ELSE ms$="Draw!"
1360 GOSUB 2980
1370 CLS:ms$="Play again? (Y:N)":LOCATE hcols-INT(LEN(ms$)/2),hrows:INPUT"Play Again? (Y:N)",a$
1380 IF UPPER$(a$)="Y" THEN RUN ELSE GOTO 3480
1390 END
1400 '
1410 ' CPU Normal
1420 GOSUB 1490
1430 RETURN
1440 '
1450 ' CPU Attacker
1460 GOSUB 1490
1470 RETURN
1480 '
1490 ' CPU Random
1500 ids=id-1:bx=0:by=0:tx=0:ty=0:tmpid=id:tmpopp=opp
1510 tmp=bls(ids,0,0):IF tmp<1 THEN act=0:RETURN:' no valid move found, we should never reach this state normally
1520 r=INT(RND*tmp)+1:tmpx=bls(ids,r,0):tmpy=bls(ids,r,1)
1530 'verify that the block is still valid, else refresh the valid block list
1540 tmp=0:GOSUB 2590
1550 IF tmp=0 THEN tmpid=id:GOSUB 2050 ELSE 1590:'if move invalid tmp=0 then refresh list
1560 'retry getting valid block after list has been refreshed
1570 tmp=bls(ids,0,0):IF tmp<1 THEN act=0:RETURN:' no valid move found, we should never reach this state normally
1580 r=INT(RND*tmp)+1:tmpx=bls(ids,r,0):tmpy=bls(ids,r,1)
1590 ' We found a random valid block next we need to find a valid random target
1600 tmpopp=opp:GOSUB 2210:'populate valid moves
1610 tmp=vm(0,0):IF tmp<1 THEN act=0:RETURN:' no valid target found, we should never normally reach this state
1620 r=INT(RND*tmp)+1:tx=vm(r,0):ty=vm(r,1)
1630 bx=tmpx:by=tmpy
1640 ' block and target selected, resolve action
1650 GOSUB 1930
1660 RETURN
1670 '
1680 ' CPU Defender
1690 GOSUB 1490
1700 RETURN
1710 '
1720 ' Human
1730 hx=st(id,isl,0):hy=st(id,isl,1):tmpx=hx:tmpy=hy:' Initialize cursor at last selected position
1740 GOSUB 2830:' Highlight cursor
1750 WHILE 1:CLEAR INPUT
1760 a$=INKEY$:IF a$="" THEN GOTO 1760
1770 tmpx=hx:tmpy=hy
1780 IF a$=CHR$(240) THEN hy=hy-1 ELSE IF a$=CHR$(241) THEN hy=hy+1 ELSE IF a$=CHR$(242) THEN hx=hx-1 ELSE IF a$=CHR$(243) THEN hx=hx+1 ELSE IF a$=CHR$(32) THEN 1810 ELSE GOTO 1750
1790 IF hx<1 THEN hx=1 ELSE IF hx>gw THEN hx=gw ELSE IF hy<1 THEN hy=1 ELSE IF hy>gh THEN hy=gh
1800 GOSUB 2830:GOTO 1750:'Highlight cursor and WEND
1810 IF bx=0 THEN 1820 ELSE 1840
1820 IF grd(hx,hy)=id THEN bx=hx:by=hy:SOUND 1,0,2,15,0,1:ENT -1,1,100,1:GOTO 1750:'valid move bx by locked
1830 IF grd(hx,hy)=0 OR grd(hx,hy)=opp THEN SOUND 1,300,10,10:SOUND 1,400,10,10:GOTO 1750:'invalid move need to select bx by first
1840 'bx by already locked
1850 IF bx=hx AND by=hy THEN SOUND 1,0,2,15,0,1:ENT -1,1,100,1:bx=0:by=0:GOTO 1750:'selected bx by again so unselecting it and set bx by to 0
1860 IF ABS(hx-bx)>1 OR ABS(hy-by)>1 THEN SOUND 1,300,10,10:SOUND 1,400,10,10:bx=0:by=0:GOTO 1750:'non adjacent to bx by selection, invalid resetting bx by to 0
1870 IF grd(hx,hy)<>0 AND grd(hx,hy)<>opp THEN SOUND 1,300,10,10:SOUND 1,400,10,10:bx=0:by=0:GOTO 1750:'adjacent selection not valid resetting
1880 tx=hx:ty=hy:GOSUB 2830:GOTO 1900:' valid move found proceed to action handler
1890 WEND
1900 GOSUB 1930:'action handler
1910 RETURN
1920 '
1930 ' action handler
1940 st(id,ilt,0)=tx:st(id,ilt,1)=ty:st(id,isl,0)=bx:st(id,isl,1)=by
1950 IF grd(tx,ty)=0 THEN act=1 ELSE GOSUB 2420:'if target is opp, resolve fight
1960 ' update stats on move or won fight
1970 IF act=1 OR act=2 THEN grd(tx,ty)=id:GOSUB 2320 ELSE RETURN
1980 'check if new occupied block is valid and if yes (tmp=1) add it to the list
1990 tmp=0:ids=id-1:tmp=0:tmpid=id:tmpx=tx:tmpy=ty:GOSUB 2590
2000 IF tmp=1 AND bls(ids,0,0)+1<=blmax THEN tmp=bls(ids,0,0)+1:bls(ids,0,0)=tmp:bls(ids,tmp,0)=tx:bls(ids,tmp,1)=ty
2010 'if a fight was won at tx,ty then we need to remove opponent's block from his valid list
2020 IF act=2 THEN tmpid=opp:GOSUB 2680
2030 RETURN
2040 '
2050 ' Populate bls list with all valid moves for id
2060 tmp=0:ids=tmpid-1
2070 minx=st(tmpid,imn,0):miny=st(tmpid,imn,1):maxx=st(tmpid,imx,0):maxy=st(tmpid,imx,1)
2080 FOR x=minx TO maxx:FOR y=miny TO maxy
2090 SOUND 1,1000,2,15:hx=x:hy=y:GOSUB 2760:' highlight grid scanning
2100 IF grd(x,y)<>tmpid THEN GOTO 2180
2110 FOR dx=-1 TO 1:FOR dy=-1 TO 1
2120 IF dx=0 AND dy=0 THEN GOTO 2170
2130 nx=x+dx:ny=y+dy
2140 IF nx<1 OR nx>gw OR ny<1 OR ny>gh THEN GOTO 2170
2150 IF grd(nx,ny)=0 OR grd(nx,ny)=tmpopp THEN tmp=tmp+1 ELSE 2170
2160 bls(ids,0,0)=tmp:bls(ids,tmp,0)=x:bls(ids,tmp,1)=y:GOTO 2180:' valid block found move to next
2170 NEXT:NEXT
2180 NEXT:NEXT
2190 RETURN
2200 '
2210 ' populate valid moves for tmpopp and block at tmpx,tmpy
2220 tmp=0
2230 FOR dx=-1 TO 1:FOR dy=-1 TO 1
2240 IF dx=0 AND dy=0 THEN GOTO 2280
2250 nx=tmpx+dx:ny=tmpy+dy
2260 IF nx<1 OR nx>gw OR ny<1 OR ny>gh THEN 2280
2270 IF grd(nx,ny)=0 OR grd(nx,ny)=tmpopp THEN tmp=tmp+1:vm(tmp,0)=nx:vm(tmp,1)=ny ELSE 2280
2280 NEXT:NEXT
2290 vm(0,0)=tmp
2300 RETURN
2310 '
2320 ' update stats after move, or won fight
2330 tmp=st(id,icn,0)+1:st(id,icn,0)=tmp:st(id,ism,0)=st(id,ism,0)+tx:st(id,ism,1)=st(id,ism,1)+ty:st(id,ivg,0)=INT(st(id,ism,0)/tmp):st(id,ivg,1)=INT(st(id,ism,1)/tmp)
2340 st(id,imn,0)=MIN(st(id,imn,0),tx):st(id,imn,1)=MIN(st(id,imn,1),ty):st(id,imx,0)=MAX(st(id,imx,0),tx):st(id,imx,1)=MAX(st(id,imx,1),ty)
2350 IF act<>2 THEN RETURN
2360 'there was a fight and opp lost a block
2370 IF st(opp,icn,0)-1<1 THEN st(opp,icn,0)=0:RETURN
2380 tmp=st(opp,icn,0)-1:st(opp,ism,0)=st(opp,ism,0)-tx:st(opp,ism,1)=st(opp,ism,1)-ty:st(opp,ivg,0)=INT(st(opp,ism,0)/tmp):st(opp,ivg,1)=INT(st(opp,ism,1)/tmp):st(opp,icn,0)=tmp
2390 GOSUB 2500:' recalculate min max x y if needed due to opp's lost block
2400 RETURN
2410 '
2420 ' resolve fg
2430 pfg=0.35:FOR dx=-1 TO 1:FOR dy=-1 TO 1:IF dx=0 AND dy=0 THEN 2460
2440 nx=tx+dx:ny=ty+dy:IF nx<1 OR nx>gw OR ny<1 OR ny>gh THEN 2460
2450 IF grd(nx,ny)=id THEN pfg=pfg+btl(frn)+RND*(btl(frx)-btl(frn)) ELSE IF grd(nx,ny)=opp THEN pfg=pfg+btl(opn)+RND*(btl(opx)-btl(opn))
2460 NEXT:NEXT
2470 IF RND<pfg THEN act=2 ELSE act=3
2480 RETURN
2490 '
2500 ' recalculate minx maxx miny maxy
2510 IF st(opp,imn,0)=tx OR st(opp,imn,1)=ty OR st(opp,imx,0)=tx OR st(opp,imx,1)=ty THEN 2520 ELSE RETURN
2520 minx=st(opp,imn,0):maxx=st(opp,imx,0):miny=st(opp,imn,1):maxy=st(opp,imx,1)
2530 FOR i=minx TO maxx:FOR j=miny TO maxy:IF grd(i,j)=opp THEN st(opp,imn,0)=i:GOTO 2540 ELSE NEXT:NEXT
2540 FOR i=maxx TO minx STEP -1:FOR j=miny TO maxy:IF grd(i,j)=opp THEN st(opp,imx,0)=i:GOTO 2550 ELSE NEXT:NEXT
2550 FOR j=miny TO maxy:FOR i=minx TO maxx:IF grd(i,j)=opp THEN st(opp,imn,1)=j:GOTO 2560 ELSE NEXT:NEXT
2560 FOR j=maxy TO miny STEP -1:FOR i=minx TO maxx:IF grd(i,j)=opp THEN st(opp,imx,1)=j:GOTO 2570 ELSE NEXT:NEXT
2570 RETURN
2580 '
2590 ' cpu only check if block at tmpx tmpy has at least one valid move
2600 tmp=0:IF tmpid=id2 AND h=1 THEN RETURN
2610 IF grd(tmpx,tmpy)<>tmpid THEN RETURN
2620 FOR dx=-1 TO 1:FOR dy=-1 TO 1:IF dx=0 AND dy=0 THEN GOTO 2650
2630 nx=tmpx+dx:ny=tmpy+dy:IF nx<1 OR nx>gw OR ny<1 OR ny>gh THEN 2650
2640 IF grd(nx,ny)=0 OR grd(nx,ny)=tmpopp THEN tmp=1:RETURN ELSE 2650
2650 NEXT:NEXT
2660 RETURN
2670 '
2680 ' cpu only: remove block from list at tmpx,tmpy
2690 IF tmpid=id2 AND h=1 THEN RETURN
2700 ids=tmpid-1:tmp=bls(ids,0,0):IF tmp<1 THEN RETURN
2710 FOR i=1 TO tmp:IF bls(ids,i,0)=tmpx AND bls(ids,i,1)=tmpy THEN 2720 ELSE NEXT
2720 FOR j=i TO tmp-1:bls(ids,j,0)=bls(ids,j+1,0):bls(ids,j,1)=bls(ids,j+1,1):NEXT
2730 bls(ids,0,0)=tmp-1
2740 RETURN
2750 '
2760 ' highlight
2770 IF hx<1 OR hy<1 OR hx>gw OR hy>gh THEN RETURN
2780 LOCATE ofx+hx,ofy+hy:IF tmpid=id1 THEN PEN cl1:PRINT hb$ ELSE PEN cl2:PRINT hb$
2790 IF grd(hx,hy)=id1 THEN PEN cl1:a$=b1$ ELSE IF grd(hx,hy)=id2 THEN PEN cl2:a$=b2$ ELSE a$=eb$
2800 LOCATE ofx+hx,ofy+hy:PRINT a$
2810 RETURN
2820 '
2830 'highlight human cursor
2840 'restore previous position
2850 IF grd(tmpx,tmpy)=id1 THEN PEN cl1:tmp$=b1$ ELSE IF grd(tmpx,tmpy)=id2 THEN PEN cl2:tmp$=b2$ ELSE tmp$=eb$
2860 LOCATE ofx+tmpx,ofy+tmpy:PRINT tmp$
2870 IF bx>0 THEN PEN cl2 ELSE PEN ctx
2880 IF grd(hx,hy)=id1 THEN PAPER cl1 ELSE IF grd(hx,hy)=id2 THEN PAPER cl2 ELSE PAPER cbg
2890 LOCATE ofx+hx,ofy+hy:PRINT hb$
2900 PAPER cbg:PEN cpuclr
2910 RETURN
2920 '
2930 ' print block counts
2940 PEN cl1:LOCATE cols-3,c1y:PRINT USING "####";c1;
2950 PEN cl2:LOCATE cols-3,c2y:PRINT USING "####";c2;
2960 RETURN
2970 '
2980 ' print centered message
2990 tmp=INT(LEN(ms$)/2):tmpx=MAX(1,hcols-tmp):LOCATE tmpx,hrows:PEN ctx:PRINT ms$:tmp=LEN(ms$)
3000 CLEAR INPUT:CALL &BB18:LOCATE tmpx,hrows:PRINT SPC(tmp)
3010 RETURN
3020 ' draw grid border
3030 ' draw top and bottom horizontal
3040 PEN ctx:LOCATE ofx,ofy:PRINT CHR$(150)+STRING$(gw,CHR$(154))+CHR$(156)
3050 LOCATE ofx,ofy+gh+1:PRINT CHR$(147)+STRING$(gw,CHR$(154))+CHR$(153)
3060 ' draw verticals
3070 a$=CHR$(149):FOR i=ofy+1 TO gh+ofy:LOCATE ofx,i:PRINT a$;:LOCATE ofx+gw+1,i:PRINT a$:NEXT
3080 RETURN
3090 '
3100 ' Define grid size
3110 gw=MAX(INT(cols/3),INT(RND*cols)+2):gw=MIN(gw,cols-2):hgw=INT(gw/2):IF gw MOD 2=0 THEN gw=gw-1' Ensure gw is odd, -2 cols for vertical grid lines
3120 gh=MAX(INT(rows/3),INT(RND*rows)+2):gh=MIN(gh,rows-5):hgh=INT(gh/2):IF gh MOD 2=0 THEN gh=gh-1' Ensure gh is odd, -5 rows for 2 cpu rows, 1 status line and 2 grid rows
3130 gwh=gw*gh
3140 ofx=INT((cols-gw)/2):ofy=INT((rows-gh)/2)+1:' locate offsets: ofy+1 row to leave a blank line for the status line
3150 RETURN
3160 '
3170 ' Define starting positions
3180 st1x=INT(gw/2)+1:st1y=1:st2x=INT(gw/2)+1:st2y=gh
3190 FOR i=0 TO ial:st(id1,i,0)=st1x:st(id1,i,1)=st1y:st(id2,i,0)=st2x:st(id2,i,1)=st2y:NEXT
3200 c1=1:st(id1,icn,0)=c1:c2=1:st(id2,icn,0)=c2
3210 grd(st1x,st1y)=id1:grd(st2x,st2y)=id2:
3220 LOCATE  ofx+st1x,ofy+st1y:PEN cl1:PRINT b1$:LOCATE  ofx+st2x,ofy+st2y:PEN cl2:PRINT b2$
3230 p=0.1:sb=MAX(1,INT((gwh/2)*p)):' starting blocks formula
3240 ' randomly select starting blocks for reach player
3250 WHILE c1<sb OR c2<sb
3260 ' Player 1 and 2 block selection
3270 FOR i=id1 TO id2
3280 tmp=st(i,icn,0)
3290 IF tmp>=sb THEN 3380
3310 'id1 has blocks from y=1 to y=gh/2-1, e.g. if gh=11 id1 y ranges from 1-5. Similarly for id2 is from 7-11.
3300 IF i=id1 THEN tmpx=INT(RND*gw)+1:tmpy=INT(RND*INT(gh/2))+1 ELSE tmpx=INT(RND*gw)+1:tmpy=INT(RND*(gh-INT(gh/2)-1))+INT(gh/2)+2
3320 IF grd(tmpx,tmpy)<>0 THEN 3380
3330 grd(tmpx,tmpy)=i:tmp=tmp+1:st(i,icn,0)=tmp:st(i,ism,0)=st(i,ism,0)+tmpx:st(i,ism,1)=st(i,ism,1)+tmpy
3340 st(i,ivg,0)=INT(st(i,ism,0)/tmp):st(i,ivg,1)=INT(st(i,ism,1)/tmp):st(i,imn,0)=MIN(st(i,imn,0),tmpx):st(i,imn,1)=MIN(st(i,imn,1),tmpy)
3350 st(i,imx,0)=MAX(st(i,imx,0),tmpx):st(i,imx,1)=MAX(st(i,imx,1),tmpy)
3360 IF i=id1 THEN cpuclr=cl1:a$=b1$ ELSE cpuclr=cl2:a$=b2$
3370 SOUND 1,1500,2,10:LOCATE ofx+tmpx,ofy+tmpy:PEN cpuclr:PRINT a$
3380 NEXT
3390 c1=st(id1,icn,0):c2=st(id2,icn,0)
3400 WEND
3410 RETURN
3420 '
3430 ' error handling
3440 ms$="Error with code"+STR$(ERR)+" at line"+STR$(ERL):GOSUB 2980
3450 MODE 2:INK 0,1:INK 1,26:PEN 1:PAPER 0:BORDER 1:LOCATE 1,1:PRINT ms$
3460 IF ERR=5 THEN PRINT"Improper Argument"
3470 END
3480 INK 0,1:INK 1,26:PAPER 0:PEN 1:BORDER 1:MODE 2:END
3490 ' DATA
3500 DATA "start","sum","avg","min","max","sel","last","count":' sel is last selected position, last is last occupied position
3510 DATA "Normal","Attacker","Random","Defender","Human":' Personality names
3520 DATA "Nrm","Att","Rnd","Def","Hmn":' Shorthands for Personality names
