// Tests for BASIC listings from AMSOFT_AMSTRAD_BASIC_Initiation_au_Basic_AMSTRAD_Partie1_SOFT411[OCR].pdf
// Each test parses a complete program with OCR error correction

mod common_book_tests;
use common_book_tests::test_basic_program;

/// Fix common OCR errors specific to this book
fn fix_ocr_errors(code: &str) -> String {
    code.replace("DRAM ", "DRAW ")
        .replace("DRAw ", "DRAW ")
        .replace("oRAw ", "DRAW ")
        .replace("DRAl›l ", "DRAW ")
        .replace("DRAH ", "DRAW ")
        .replace("DRAN ", "DRAW ")
        .replace("HOVE ", "MOVE ")
        .replace("Move ", "MOVE ")
        .replace("HODE ", "MODE ")
        .replace("REN ", "REM ")
        .replace("PR I NT", "PRINT")
        .replace("'", "'")
        .replace("■", "-")
        .replace("›", "")
        .replace("¿", "")
        .replace("l>", "D")
        .replace("t›", "D")
        .replace("MOVE100", "MOVE 100")
        .replace("PLOT×", "PLOT x")
        .replace("INPUT\"", "INPUT \"")
        .replace("PRINT\"", "PRINT \"")
}

/// Macro to generate test functions for this book
macro_rules! basic_test {
    ($name:ident, $program:expr, $desc:expr) => {
        #[test]
        #[ignore = "This test requires manual verification of the OCR correction. Please review the corrected code and expected output before enabling this test."]
        fn $name() {
            let fixed = fix_ocr_errors($program);
            test_basic_program(&fixed, $desc, stringify!($name));
        }
    };
}

#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_001,
    r##########"10 REM drawing a house (dessiner une maison)
20 MODE 0
30 CLS
40 REM ** start ** (début)
50 Border 12
60 INK 0,12:REM Yellow (jaune)
70 INK 1,3:REM red (rouge)
80 INK 2,6:REM bright red (rouge vif)
90 INK 3,9:REM green (vert)
100 PAPER 0
110 REM draw front (dessiner la façade)
120 MOVE100,50 (déplacement)
130 DRAM 100,250,1
140 DRAM 400,250
150 DRAM 400,50
160 DRAM 100,50
170 REM draw side (dessiner le coté)
180 MOVE 400,250
190 DRAM 600,250
200 DRAM 600,50
210 DRAM 400,50
220 DRAM 400,250
230 REM draw gabLe end (dessiner le pignon)
240 REM already at start point (déja au point de depart)
250 REM so no need for a MOVE (donc pas besoin de se deplacer)
260 DRAM 500,350
270 DRAM 600,250
280 DRAM 400,250
290 REM draw roof (dessiner Le toit)
300 REM only two Lines needed (seulement deux droites)
310 MOVE 100,250
320 DRAM 200,350
330 DRAM 500,350
340 REM draw door (dessiner La porte)
350 MOVE 225,50
360 DRAM 225,140,2
370 DRAM 275,140
380 DRAM 275,50 _
390 REM draw windows (dessiner les fenetres)
400 REM left hand bottom (fond partie gauche)
410 MOVE 120,70
420 DRAM 120,130,3
430 DRAM 180,130
440 DRAM 180,70
450 DRAM 120,70
460 REM left hand top (dessus partie gauche)
470 MOVE 120,170
480 DRAM 120,230
490 DRAM 180,230
500 DRAM 180,170
510 DRAM 120,170
520 REM right hand top (dessus partie droite)
530 MOVE 320,170
540 DRAU 320,230
550 DRAN 380,230
560 DRAH 380,170
570 DRAN 320,170
580 REM right hand bottom (fond partie droite)
590 MOVE 320,70
600 DRAM 320,130
610 DRAH 380,130
620 DRAH 380,70
630 DRAM 320,70 Test Peut-êtresouhaitez-vousreparcourirlechapitre avant de passer le test SAT5. Nous avons vu jusqu'apresentbeaucoupdechosestrèsimpor- tantes en peu de temps, mais Pheure est venue d'aborder la programmation proprement dite."##########,
    "pages 39, 40, 41"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_002,
    r##########"10 Rem 3D Bar Chart (histogramme)
20 REM by Dave Atherton
30 MODE1
40 BORDER1¿›:INK 0,14:INK1,0:INK 2,3:INK 3
50 b=50:REM bar size
60 LOCATE1,23:INPUT "Amount (1-290)" ;a
70 ×=100:PLOT ×,55,1
80 DRAM ×-b*2,55:l>RAu ×-b*2, a+55
90 DRM: ×-b,a+55+b:t›RAu ×+b,a+55+b
100 DRAN x,a+55:DRAH ×-b*2,a-l-55
110 Move ×+b,a+b+55:oRAw ×+b,b+55
120 oRAw ×,55:t›RAu x,55+a
130 LOCATE 1,23
140 PRINT"
150 LOCATE 1, 23: INPUT "Amount (1-290)" ;a
160 ×=260:PLOT ×,55,2
170 DRAH ×-b*2,55:oRAw ×-b*2,a+55
180 DRAN ×-b,a+55+b:DRAlrl ×+b,a+55+b
190 DRAM ×,a+55:oRAw ×-b*2,a+55
200 MOVE ×+b ,a+b+55:DRAN ×+b,b-|-55
210 oRAw ×,55:DRAw ×,55+a
220 LOCATE 1,23
230 PRINT"
240 LOCATE1,23:INPUT "Amount (1-290) ";a
250 x=420:PLOT ×,55,3
260 DRAM ×-b*2,55:DRAw × - b*2,a+55
270 DRAH x-b, a+55+b:DRAl›l ×+b,a-|-55-I-b
280 DR/-lu ×,a+55:oRAw ×-b*2,a+55
290 Move ×+b, a+b+55:nRAw ×+b,b+55
300 DRM: ×,55:1›RAw ×,55+a
310 LOCATE 1,23
320 PRINT"
330 LocATE1,23:1NPuT "Amount (1-290)";a
340 ×=580:PLOT ×,5S,1
350 t›RAw ×-a*2,55:oRAu ×-b*2,a+55
360 oRAw ×-b,a+55+b:oRAw ×+b,a+55+b
370 DRM: ×,a+55:oRAu ×-b*2,a+55
380 Move ×+b,a+b+55;oRAu ×+b, b+55
390 DRAH ×,55:DRAl›l ×,55+a"##########,
    "pages 49, 50"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_003,
    r##########"13 il LorsquevousavezrecoursàGOTO,lemot-clé Les machines font des fautes, les hommes -_ *.. --«UH ,_ IF peut vous étre trés utile. Il agit comme un commettent des erreurs. Nous avons déjà dit *In panneau indicateur pour guider le CPC464 que les ordinateurs ne pensent pas et que le lili4-.'ll . sQ r verslapartieduprogrammeparlaquelleildoit programmeurdoitétablirsespropres lignesde ft 1'. poursuivre. A supposer que vous vouliez que pensée; s'il se trompe, et cela peut trés bien ._' .LgI~l':.-'\_ ll. | l'ons'arréted'introduiredesnombressupérieurs arriverauxmeilleursd'entreeux,ilnepeuts'en cf' ,E | IJ'' - 1 1 '§*~*'1r* 3 à 295 dans le programme BARCHART, les rendrecomptequ'unefoisleprogrammepassé lignes suivantes se présenteront de la maniére dans Pappareil. Le CPC464 exécute les ordres ci-dessous: reçus méme si ceux-ci ne constituent pas toujours ce que le programmer avait prévu. 60INFUT "Amount (0290)”;a.
65 IF aj, 2.90GOTO 60 Le“de-bugging” estleprocessusquiconsisteà parcourir un programme en en corrigeant les _]usqu'àcequ'unnombreinférieurouégalà290 erreurs de logique ou de compréhension. Ne soit introduit, le CPC464 peut se diriger au voussentezpashonteuxsivousintroduisezun mauvais endroit à la moindre erreur de votre programmecontenant des bugs (erreurs sur le part. Nous voici arrivés à point nommé à un ROM).Biensur,lesprogrammeurschevronnés mot classique du jargon-ordinateur: rencontrentmoinssouventdesbugsdansleurs programmes, mais il ne s'agit là que d'une questiondeconnaissancesetdepratique. Vous vous rendrez bientot compte que les bugs se montrentdemoins en moins souventdans vos programmes. 55"##########,
    "page 57"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_004,
    r##########"55 print “Line 55 a= “;a Oui, nous voici de nouveau à la maison. Cette Vous pouvez également avoir recours à la fois-ci, elle aurait bien besoin d'une couche de commande STOP. Par exemple: peinture par-ci par-là. Cherchez la carte des couleursetchargezleprogrammeDECO. Ilne
55 stop s'agit pas d'un jeu mais plutôt d'une façon Le CPC464 interrompera le cours d'exécution amusanted'utilisertouteslescommandesapp- du programme à ce point et vous pouvez lui risesjusqu'ici. Voussouhaitezpeut-étremodifier ordonnerd'imprimerlavariableoulesvariables votre programme pour lui donner une touche qui vous interessant gràce aux commandes plus personnelle. Il vous suffit pour cela de d'impression directe. On revient au début du mettre en pratique les techniques étudiées en programme en faisant: début de chapitre. cont [ENTER] Sivous rencontrezdesmots-cléinconnusdans le listing du DECO présenté aux prochaines Cela signifie CONTinuer; n'oubliez pas que pages,nevousinquiétezpas: toutvousparaîtra celanemarchepassideslignessontajoutéesou plus clair trés bientôt. effacées aprés l'arrét (STOP) du programme. U ':[ E1 W-_ 57"##########,
    "page 59"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_005,
    r##########"10 REM deco
20 MODE 0
30 CLS
40 REM ** START **
50 BORDER 12 (bord 12)
60 INK 0,12:REM YELLOW (encre 0, rem jaune)
70 INK 3,3:REM red (rouge)
80 INK 6,6:REM bright red (rouge vif)
90 INK 9,9:REM green (vert)
100 PAPER 0 (papier)
110 REM draw front (dessin de La façade)
120 MOVE100,50 (Déplacement)
130 DRAN100,250,3
140 DRAH 400,250
150 DRAM 400,50
160 DRAl›l100,50
170 REM draw side (dessin du coté)
180 MOVE 400,250
190 DRM-l 600,250
200 DRM-l 600,50
210 DRAM! 400,50
220 DRAN 400,250
230 REM draw gabLe end (dessin du bout du pignon)
240 REM already at start point (déjà en position de départ)
250 REM so no need for a MOVE (donc pas besoin de déplacement)
260 DRAH 500,350
270 DRAH 600,250
280 DRM-l 400,250
290 REM draw roof (dessin du toit)
300 REM only two Lines needed (besoin de seulement deux Lignes) MovE100,250 310 oRAw200,350 320 oRAw 500,350 330
340 REM draw door (dessin dela porte)
350 MOVE 225,50
360 DRAM 225,140,
370 DRAM 275,140
380 DRAM 275,50
390 REM draw windows (dessin des fenetres)
400 REM Left hand bottom (partie en bas a gauche)
410 MOVE 120,70
420 DRAM l20,130,9
430 DRAM 180,130
440 DRAM 180,70
450 DRAM 120,70
460 REM Left hand top (partie en haut a gauche)
470 MOVE120,170
480 DRAM 120,230
490 DRAM 180,230
500 DRAM 180,170
510 DRAM 120,170
520 REM right hand top (partie en haut a droite)
530 MOVE 320,170
540 DRAM 320,230
550 DRAM 380,230
560 DRAM 380,170
570 DRAM 320,170
580 REM right hand bottom (partie en bas à droite)
590 MOVE 320,70
600 DRAM 320,130
610 DRAM 380,130
620 DRAM 380,70
630 DRAM 320,70
640 REM *** DECO ***
650 r$=CHR$ (18)
660 LOCATE 1,25
670 PRINT "Type a colour (1-15) "; (frappe d'une couleur)
680 FOR i=1 T015
690 INK i,i:NEXT i
700 LOCATE1,1:PRINT P$;
710 INPUT "Roof colour";r (couleur du toit)
720 FOR i=107 T0 399 STEP 2
730 MOVE i,252:DRAM i+96,349,r
740 NEXTi
750 LOCATE 1,1 :PRINT r'$;
760 INPUT ""gable end' ';g (bout du pignon)
770 FOR i=252 TO 346
780 MOVE i+154,i:DRAM 848-i,i,g
790 NEXTÎ
800 LOCATE 1,1 :PRINT r$;
810 INPUT "End waLL";e (bout du mur)
820 FOR i=52 TO 248 STEP 2
830 MOVE 404,i:DRAM 598,i,e
840 NEXTi
850 LOCATE1 1:PRINT r$'
860 INPUT "front";f (façade)
870 FOR i=52 TO 248 STEP 2
880 MOVE104,i:DRAM 398,i,f
890 NEXT i
900 LocAiE 1,1:PR1NT rs;
910 INPUT "Door";d (porte)
920 FOR i=52 T0 138
930 MOVE 229,i:DRAM 268,i,d
940 NEXT 'Î
950 LOCATE1,12PRINT r'$;
960 INPUT "Mindow";w (fenêtre)
970 FOR j=0 T0100 STEP 100
980 FOR i=70+j TO130+j
990 MOVE120,'Î:DRAM180,i,M
1000 MOVE 320,i :DRAM 380,i
1010 NEXT i
1020 NEXT j
1030 END Test Passez le SAT7 afin de vérifier votre compré- hension de cechapitre. Ne vous effrayez pas si vous voyez que vous avez besoin de consulter les pages précédentes avant de répondre aux questions. Bonnombredeprogrammeursutili- sant eux-mêmes un manuel de référence."##########,
    "pages 60, 61, 62, 63"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_006,
    r##########"10 REM mansion (maison)
20 MODE 0
30 CLS
40 REM ** start ** (debut)
50 BORDER 12 (bord)
60 INK 0,12:REM yellow (encre jaune)
70 INK 3,3:REM red (rouge)
80 INK 6,6:REM bright red (rouge vif)
90 INK 9,9:REM green (vert)
100 PAPERO (papier)
110 REM draw front (dessin dela façade)
120 MOVE100,50
130 DRAM 100,250,3 65
140 DRAM 400,250
150 DRAM 400,50
160 DRAM 100,50 _
170 REM draw side (dessin du coté)
180 MOVE 400,250
190 DRAM 600,250
200 DRAM 600,50
210 DRAM 400,50
220 DRAM 400,250
230 REM draw gable end (dessin du bout du pignon)
240 REM already at start point (déjà au point de départ)
250 REM so no need for a move (donc inutile de se déplacer)
260 DRAM 500,350
270 DRAM 600,250
280 DRAM 400,250
290 REM draw roof (dessin du toit)
300 REM only two lines needed (besoin de seulement deux lignes)
310 MOVE100,250
320 DRAM 200,350
330 DRAM 500,350
340 REM draw door (in red) (dessin dela porte, en rouge)
350 MOVE 225,50
360 DRAM 225,140,6
370 DRAM 275,140
380 DRAM 275,50
390 REM draw windows (in green) (dessin des fenêtres, en vert)
400 REM left hand bottom (partie en bas à gauche)
410 MOVE 120,70
420 DRAM 120,130,9
430 DRAM 180,130
440 DRAM 180,70
450 DRAM 120,70
460 REM left hand top (partie en haut a gauche)
470 MOVE120,170
480 DRAM 120,230
490 DRAM 180,230
500 DRAM 180,170
510 DRAM 120,170
520 REM right hand top (partie en haut a droite)
530 MOVE 320,170
540 DRAM 320,230
550 DRAM 380,230
560 DRAM 380,170
570 DRAM 320,170
580 REM right hand bottom (partie en bas a droite)
590 MOVE 320,70
600 DRAM 320,130
610 DRAM 380,130
620 DRAM 380,70
630 DRAM 320,70
635 STOP
640 REM fence (grille)
650 FOR F=0 T0 620 STEP 20
660 MOVE F,0:DRAM F,60,
670 NEXT F move c,4s=:›RAu 620,45 680"##########,
    "pages 67, 68, 69"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_007,
    r##########"71 U size=18 72[1 MOVE '30,78:GOSUB 900 73Cl MOVE 156,78:GOSUB 900 74Cl MOVE 130,103 G05 0090 U1'* 75U MOVE '56,103 GOS 0090 l4ü'?l 76\El MOVE 130,178 GOS-B 900 156,178 77\Tl MOVE GOS08900 78U « *W hi MOVE 130,203 GOS08900 79U MOVE 156,203 GOSlB900 80U MOVE 330,78:GOSUB 90\Pl bb
81 T4_'?T MOVE 356,78:GOSUB 900 82LT-Tl MOVE 330,103 GOS_.B 9100 83T U 1 MOVE 356,103 GOS-B 900 84\F1 MOVE 330,178 GOSJB 900 85U î MOVE 356,178 GOSlà;B 900"##########,
    "page 70"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_008,
    r##########"860 MOVE 330,203 GOSUB 900 87G MOVE 356,203 GOS-B 900 88U END (fin) 89Cl REM subroutine forsquare (sous-programmaticwipourcarre)
900 DRAMR 0,size,9 L»"##########,
    "page 70"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_009,
    r##########"91 iLl-Ill DRAMR size,0
92 L W i 0RAwR 0,-si ZE 931"* DRAMR -size, -uó 94O RETURN"##########,
    "page 70"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_010,
    r##########"74 Franchissez-le Refermez le portail Else (aut rement) autrement dirigez vous vers la Next (suivante) maison suivante."##########,
    "page 76"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_011,
    r##########"20 REMEU/Dave Atherton
30 MODE1
35 INKO U1 MU _0CA" :PRINT"SOUND DEMO" 50\_OCAf PRINT"1. Explosion""##########,
    "page 90"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_012,
    r##########"180 IF a=1'HEN GOS..B 280
190 IF a= 'HEN GOSLB 330
200 IFa== 'HEN GOS-B 38C
210 IF a: THEN GOSJB 431:
220 IF a: THEN GOSUB -P*œ
230 IF a=ONU1-l'-\U~l|'\J 'THEN GOSUB 531:
240 IF a=7 THEN GOS*...B 58C4,C14L
250 FOR J=O T0 'lUO0:NEXT
260 LOCATE 2U,19:PRINT""
270 GOTO 130 _ '\-'
280 REM Explosion
290 ENV1,11,-1,25
300 ENT 1,9,49,5,9,-10,15
310 SOUND 1,145,255,0,1,1,12
320 RETURN
330 REM Dog Bark (aboiement de chien)
340 ENV 1,4,7,1O
350 ENT 1,7,-8,3,6,24,2
360 SOUND 1,120,33,8,1,1,3
370 RETURN
380 REM Siren
390 ENV1,2,9,45
400 ENT1,2,9,45
410 SOUND 1,150,90,6,1,1
420 RETURN
430 REM Toilet FLush (chasse d'eau)
440 ENV 1,3,-2,85
450 ENT1,5,-1,51
460 SOUND 1,150,254,11,1,1,8
470 RETURN T8O REM Cuckoo (cocorico)
490 ENV 1,4,12,11 SDD ENT 1,5,12,8
510 SOUND 1,165,40,13,1,1
520 RETURN
530 REM Machine Gun
540 ENV1,21,-5,4
550 ENT1
560 SOUND 1,162,82,15,1,1,11
570 RETURN
580 REM Space Invader (anvahisseur de L'espace)
590 ENV 1,4,30,19
600 ENT 1,9,49,5,1,-10,26
610 SOUND 1,136,68,15,1,1,0
620 RETURN"##########,
    "pages 90, 91, 92"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_013,
    r##########"30 15 x6 l°ecran. Cependant ceci n'est pas parfait: quel- 3+2 9 quefoisle résultatd'unesuitedecalculsnesera 24_i_,4x3i 5×(2+s) pas absolument exact. Au lieu de 5,0, il peut Ces regles s'appliquént aux nombres autant vous arriver d'obtenir: qu'aux variables. Passez au programmé suivant 5,!ZI¿ïZï¿I¿IZIÔ1 et changez plusieurs fois la ligne 40 afin de pouvoircalculerlesautresoperationsci-dessus. ou bien: N'oubliezpasleurssymbolesenBasicetplacez les paranthéses au bon endroit. 4,99999999 ' Subtil,n'est-cepas? Heureusement,nousn'avons pas souvent besoin d'une telle precision. En"##########,
    "page 95"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_014,
    r##########"10 REM Garden
20 MODE 1
30 INK 0,0:BORDER 0
40 INK 1,26
50 Length=6
60 CLS
70 PRINT "GARDEN" (jardin)
80 PRINT "Length remaining: "Length; "metres" (longueur cultivable restante: Longueur mètres)
90 PRINT "Hhich Vegetable do you want to grow" (Quel légume voulez vous cultiver?)
100 PRINT
110 PRINT" width yield rows" (largeur) (rendement) (rangs)
120 PRINT"1. ONIONS 0.3m 9Kg ";onions
130 PRINT"2. CARROTS 0.3m 3Kg ";carrots
140 PRINT"3. POTATOES 50Kg ";potatoes
150 PRINT"4. CABBAGES 6m 8Kg ";cabbages
160 PRINT"5. BEANS EB 3OKg ";beans
170 PRINT"6. PARSNIPS C_)._z¢3..› 5m 1Kg ";parsnips
180 PRINT
190 PRINT "Enter a number between 1 and 6" (chiffre entre 1 et 6)
200 PRINT "or 7 to show total output" (ou 7 pour montrer la totalité dela production)
210 INPUT veg
220 IF veg<=0 oR ve-g>7 THEN GoTo190
230 IF veg=7 THEN GOTO 320
240 IF veg=1 THEN GOSUB 470
250 IF veg=2 THEN GOSUB 540
260 IF veg=3 THEN GOSUB 610
270 IF veg=4 THEN GOSUB 680
280 IF veg=5 THEN GOSUB 750
290 IF veg=6 THEN GOSUB 820
300 IÎ ______________________________________ Il
310 GOT060
320 REM summary (résumé)
340 PRINT "SUMMARY"
350 PRINT "GARDEN OUTPUT IN KILOS" (production totale en Kg)
370 PRINT
380 PRINT"Onions :";onions*9;"Kg"
390 PRINT"Carrots :"; carrots*3;"Kg"
400 PRINT"Potatoes :";potatoes*50;"Kg"
410 PRINT"Cabbages :";cabbages*8;"Kg"
420 PRINT"Beans :";beans*30;"Kg"
430 PRINT"Parsnips :";parsnips*11;"Kg
450 GOTO 450 460
470 REM Onions
480 PRINT"Onions"
490 rowwidth=0.3
500 GOSUB 890
510 onions=rows
520 RETURN 530
540 REM Carrots
550 PRINT"Carrots"
560 rowwidth=O.3:produce=3
570 GOSUB 890
580 carrots=rows
590 RETURN 600
610 REM Potatoes
620 PRINT"Potatoes"
630 rowwidth=1
640 GOSUB 890
650 pot atoes=rows
660 RETURN 670:
680 REM Cabbages (choux)
690 PRINT""Cabbages' '
700 rowwidth=0.ó (largeur du rang)
710 GOSUB 890
720 cabbages=rows
730 RETURN 740:
750 REM Beans
760 PRINT""Beans"
770 rowwidth=1
780 GOSUB 890
790 beans=rows
800 RETURN 810:
820 REM Parsnips
830 PRINT""Parsnips"
840 rowwidth=0.5
850 GOSUB 890
860 parsnips=rows
870 RETURN 880:
890 REM Details
900 INPUT ""How many rows do you want to plant' ';rows (combien de rangs souhaitez vous)
910 testlengthîlength-rows* rowwidth 9201F test Length<0 THEN PR1NT""n0 room' ' :Goto 900
930 length=testlength
950 RETURN O0"##########,
    "pages 99, 100, 101, 102"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_015,
    r##########"10 REM **BLACKJACK**
20 REM
30 REM **STARTUP**
40 MODE 1:BORDER 4
50 INK 0,17:INK1,0
60 LOCATE16,5:PRINT"BLACKJACK"
70 LOCATE 10,12
80 PRINT "Press a key to start" (frappez une touche pour commencer)
90 suits=cHR$(226›+cHR$<227›+cHRs(228›+cHR$<229>
100 card$="A23l+56789T.JQK"
110 C5$="5 card trickel win"
120 myace=0:yourace-=0
130 mygames=0:yourgames=0
140 HHILE INKEY$="":HEND 04
150 CLS
160 REM ** YOUR TURN ** (votre tour)
170 yourcards=0:yourace=0
180 yourhand=0
190 LocATE 20,1
200 PRINT"Games Me:";my games;
210 PRINT"You:";yourgames
220 y=20:×=5
230 yourcards=0
240 GOSUB 770
250 yourcards=yourcards+1
260 IF value=1 THEN yourace=yourace-l-1
270 yourhand=yourhand-Fvalue
280 GOSUB 830:×=×-F5
290 IF yourhand>21 THEN GOTO 690
300 GOSUB 930
310 oneacehand=yourhand
320 IF yourace>=1 THEN oneacehand=yourhand+10
350 IF yourcards=5 THEN 440
360 IF yourhand=21 THEN 440
370 IF oneacehand=21 THEN /+20
380 IF yourace=0 AND yourhand<=11 THEN 240
390 IF yourcards=i THEN 240
400 INPUT"bJant another card(Y/N)";q$ (nouvelle carte demandee (oui ou non)
410 IF UPPER$(q$)="Y" THEN GOTO 240
420 IF oneacehand<21 THEN yourhand_oneacehand
440 REM ** MY TURN ** (mon tour)
450 y=10:X=5 05
460 myhand=0:mycards=0:myace=O
470 GOSUB 770
480 mycards=mycards+1
490 IF value=1 THEN myace=myace+1
500 myhand=myhand-I-value
510 GOSUB 830:×=×+5
520 FOR delay=O TO1000:NEXT
530 IF myhand>21 GOTO 720
540 IF mycards=5 THEN GOSUB 930:PRINT C5$:GOTO 710
550 IF yourcards=5 THEN 470
560 mine!-\=m)/hand
570 IF myace>=1 AND myhand<12 THEN mineA=myhand+1O
600 IF myhand>=yourhand THEN 640
610 IF mineA>=yourhand THEN myhand=mineA:GOT0 640
630 GOTO 470
640 REM ** TEST RESULTS ** (résultats)
650 GOSUB 930
660 PRINT"I have";my hand;
670 PRINT"and you have";your hand
680 IF myhand<yourhand GOTO 730 ELSE GOTO 700
690 GOSUB 930:PRINT "You have bust!" (perdu!)
700 PRINT "I win" (je gagne)
710 mygames=mygames+1 :GOTO 140
720 GOSUB 930:PRINT "I have bust ! " (j 'ai perdu!)
730 PRINT "You win" (vous gagnez)
740 your*games=yourgames-l-1 :GOTO 140
750 END
760 REM ** GENERATE CARD ** O6
770 card=INT(RND*13)-I-1
780 sui1:=1NT <RNo*4)+1
790 value=card
800 IF value>10 THEN value=10
810 RETURN
820 REM ** PRINT CARD **
830 LOCATE ×,y
840 PRINT CHR$(2l›);" ";CHR$(24)
850 LOCATE ×,y+1
860 PRINT CHR$(24);" ";
870 PRINT MID$(card$,card,1);
880 PRINT MID$(suit$,suit,1);
890 PRINT" "; CHRS (24)
900 LOCATE ×,y+2
910 PRINT CHR$(24);" ";CHR$(24)
920 RETURN
930 LOCATE1,2¿|›:PRINT SPACE$(¿i0)
940 LOCATE1,24:RETURN O7"##########,
    "pages 106, 107, 108, 109"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_016,
    r##########"10 cr$=CHR$(13)
20 REM Simon
30 REM **** 1NsTRucT1oNs ****
40 MODE 1:BORDER 20:1NK 0,20:INK1,1
50 LOCATE16,2:PRINT CHR$(24);“Simon";CHR$(24)
60 PRINT:PRINT
70 PRINT "In this game, you_have to watch the"
80 PRINT "flashing circles and remember the"
90 PRINT "pattern. when the sequence ends you"
100 PRINT "must copy it out on the cursor keys"
110 PRINT "The sequence increases by one after"
120 PRINT "each correct attempt. PRINT"
130 PRINT "For example, a circle at the top of"
140 PRINT "the screen should be indicated by"
150 PRINT "the up cursor. The cursor keys are"
160 PRINT "above the numeric key pad, and are"
170 PRIN' "marked as foLlows:"PRINT i(`.ejeuconsisteaobserverlescerclesclignotentetalesmcmoriser.Alalindelaséquence,vousdciczlesrecopiersurlestouchesducurseur Achaqueloisque vousrcussisscz,lasequenceaugmente.Parexempleuncercleenhautdel`ecrandoitetreindiqueparlecurscursupi:rieur Lestouchesducurseursi:trouvent"##########,
    "page 110"
);
#[ignore]
basic_test!(
    test_amsoft_amstrad_basic_initiation_au_basic_amstrad_partie1_soft411_ocr__listing_017,
    r##########"108 au-dessusduclaviernumeriqueetsontsignalccsdelafaçonsuivante'i
180 PRINT TAB(20);CHR$(240)
190 PRINT TAB(19);CHR$(242);" ";CHR$(243)
200 PRINT TAB(20);CHR$(241)
210 LOCATE 7,22:PRINT"Press ENTER to continue" (appuyez sur ENTER pour continuer)
220 LOCATE 5,24:PRINT"there will be a short pause!" (attendez vous a une breve pausel)
230 NHILE INKEY$< cr$:lilEND
240 REM **** SET-UP ****
250 MODE 0 _
260 NINDOH 7,14,10,16 (fenetre)
270 b=17:f=3:REMBackground/Foreground (arriere-plan, premier plan)
280 BORDER b
290 INK 0,17
300 FOR i=1 TO15:INK i,b:NEXT
310 ×=320' =70:c=2;GOSUB 940
320 y=33O =1:GOSUB 940
330 ×=120: =200:c=3:GOSUB 940
340 x=520 ¢'I`<O`(=4 : GOSUB 940
350 INK 5,F:PEN 5
360 RANDOMIZE TIME (L'elément "hasard" intervient pour le facteur temps)
370 a$=" "
380 REM **** DISPLAY SEQUENCE **** (séquence affichage)
390 a$=a$+CHR$(RND*3+1)
400 FOR i=1 TO LEN(a$)
410 FOR j='l TO 200:NEXT
420 ×=ASC(MID$(a$,i,1))
430 INK ×,2*×+1
440 SOUND 1,10+×*100
450 FOR j=1 TO 200:NEXT 109
460 1NK×,b
470 NEXT
480 FOR i=1 TO100:NE×T
490 REM **** GET ANswER ****
500 FOR i=1 T0 LEN(a$)
510 wH1LE ks>"":k$==1NKEY$:wEND
520 FOR L=1 TO 2000:i<s=1Ni<i5Ys
530 1Fk$>""THEN560
540 NExT i<s=" " 550
560 k=ASC(k$)-239:IF k<10Rk>4 THEN52O
570 ×=ASC(MID$(a$,i,1))
580 IF k<>x GOTO 730:REM wrong
590 INK x,2*×+1
600 SOUND 1,10+×*1OO
610 FORj=fl TO80:NE×T
620 1Nk×,b
630 FOR j=1 To 20:NE×T
640 NEXT
650 REM*****R1GHT!**** (juste)
660 cLs:PR1NT"R1GHT!"
670 PR1NT:PR1NT:PR1NT" scoRE:"
680 PR1NT:PR1NT" ";LEN(A$)
690 FOR j=1 TO 600:NEXT
700 LOCATE1,1:PRINT"
710 GOTO 390
720 REM **** HRONG **** (faux)
730 SOUND 1,2000 O
740 CLS:PRINT "Wrong" 7
750 FOR j=1 TO 300:NE)(T
760 PRINT:PRINT "Sequence was:"
770 FOR i=1 TO LEN(a$)
780 ×=ASC(MID$(a$,i,1))
790 INK ×,2*×+1
800 SOUND 1,10+×*1OO
810 FOR j=1 TO 200:NEXT
820 INK ×,b
830 FOR J=1 TO 200:NEXT
840 NEXT
850 REM **** END & RESTART **** (fin et recommencement)
860 CLS
870 PRINT" You"
880 PRINT" scored "
890 PRINT:PRINT" ";LEN(a$) 900PRINT:PRINT" PRESS"
910 PRINT" ENTER"
920 wH1i_E 1Ni<EY$<>cR$:wENi>
930 GOTO 360
940 REM **** CIRCLES **** (cercles)
950 r=60
960 FOR i=-r TO r STEP 2
970 h=SQR(r*r-i*i)
980 MOVE x-h,i+y:DRAH ×+h,i+y,c
990 NEXT
1000 RETURN"##########,
    "pages 110, 111, 112, 113"
);
