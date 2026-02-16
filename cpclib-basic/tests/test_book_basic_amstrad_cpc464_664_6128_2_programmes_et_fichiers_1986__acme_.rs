// Tests for BASIC listings from BASIC_AMSTRAD_CPC464-664-6128_2-Programmes_et_fichiers(1986)(acme).pdf
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

basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_001,
    r##########"10 INK 1,2 couleur 2(bleu vif) dans case couleur 1
20 INK 2,6 couleur 6(rouge vif) dans case couleur 2 En MODE 2 seules les cases 0 et 1 sont utilisables. En MODE 1 seules les cases 0,1,2,3 sont utilisables. En MODE 0 les 16 cases sont utilisables. Table de correspondance des couleurs à la mise sous tension : Mode 2 Mode 1 Mode 0 Couleurs"##########,
    "page 10"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_002,
    r##########"10 1 20 14 10 turquoise 24 jaune
11 24 6 16 11 bleu ciel 25 jaune pastel
12 1 1 18 12 jaune 26 blanc brillant
13 24 24 22 13 blanc
14 1 20 1,24
15 24 6 16,11 En MODE 2 par exemple, si vous changez la couleur de la case 0, la couleur de la case"##########,
    "page 10"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_003,
    r##########"10 MODE 1
20 INK 0,26 ' blanc dans case 0
30 INK 1,0 ' noir dans case 1
40 INK 2,6 ' rouge dans case 2
50 PLOT 100,100,1 ’ point 100,100 avec stylo l(noir)
60 PLOT 120,120,2 ' point 120,120 avec stylo 2(rouge) L’origine de l’écran est en bas à gauche. X doit être compris entre 0 et 639. Y doit être compris entre 0 et 399. Les coordonnées spécifiées sont INDÉPENDANTES de la résolution choisie par MODE. Seule la taille du point varie. Y A 399 100___ 100,100"##########,
    "page 11"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_004,
    r##########"10 MODE 1
20 INK 0,1 bleu dans case 0
30 INK 1,24 jaune dans case 1
40 PLOT 100,100,1 point 100,100
50 DRAW 200,200,1 droite entre 100,100 et 200,200 A Y 200,200 100,100 ► X"##########,
    "page 11"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_005,
    r##########"10 I BASIC AMSTRAD PLOTR DX,DY,case couleur DRAWR DX,DY,case couleur Ces instructions spécifient des déplacements relatifs au point courant.
10 MODE 1
20 INK 0 ,26 blanc dans case 0
30 INK 1 ,0 noir dans case 1
40 INK 2 rouge dans case 2
50 PLOT 100,100,1 point 100,100
60 DRAWR 50,50,1 droite entre 100,100 et 150,150
70 DRAWR 50,0,2 droite entre 150,150 et 200,150 La première droite est dessinée avec le stylo 1 (noir), et la seconde avec le stylo 2(rouge). Si vous aviez utilisé le stylo numéro 4, les droites n’auraient pas été tracées. En MODE 1, le stylo 4 est en réalité le stylo 0. MOVE X,Y MOVER DX,DY Positionne le curseur graphique sans dessiner. Le CPC 664 dispose du paramètre “stylo”. TAG TAGOFF TAG spécifie que l’écriture du texte doit se faire avec le curseur graphique. TAGOFF annule TAG ; l’affichage du texte se fait avec le curseur texte (positionné par LOCATE X,Y). Pour empêcher l’affichage des caractères CHR$(10) et CHR$(13), l’instruction PRINT doit être suivie d'un point-virgule."##########,
    "page 12"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_006,
    r##########"10 MODE 1
20 TAG
30 MOVE 100,100 : PR I NT "AAAAAAA"; affiche en 100,100
40 TAGOFF
50 LOCATE 1,1:PRINT "BBBBBBBB" avec curseur texte"##########,
    "page 12"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_007,
    r##########"10 ' AFFICHAGE AVEC TAG
20 '
30 MODE 1
40 INK 0,26:INK 1,0
50 DIM X(100),Y(100)
60 AI="AMSTRAD..........AMSTRAD...............................
70 R=100
80 XC=200:YC=200
90 FOR 1=1 T0 30
100 A=PI*I/30*2
110 X ( I ) =R*COS(A)* 1.5 + XC
120 Y(I)=R*SI N(A)+YC
130 NEXT I
140 '-------ROTATION
150 FOR 1=1 TO 30
160 MOVE X(I),Y(I)
170 TAG: PRINT MI DI(Al, I , 1 );
180 NEXT I
190 AI = RIGHTI(Al, 1)+LEFTI(Al, LEN ( Al)-1 )
200 GOTO 150 A M S tr ° I> ORIGIN X,Y, gauche, droite, haut, bas Redéfinit l’origine pour le curseur graphique et une fenêtre graphique si les paramètres “gauche”, “droite”, “haut” et “bas” sont spécifiés."##########,
    "page 13"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_008,
    r##########"10 MODE 1
20 ORIGIN 100,100 ' origine en 100,100
30 PLOT 1,1,1
40 DRAW 100,100 ' 100,100 a partir de la nouvelle origine"##########,
    "page 13"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_009,
    r##########"10 MODE 1:INK 0,26:INK 1,0: INK 2,2
20 ORIGIN 200,100,200,350,100,250 ' -fenetre graphique
25 CLG 2
30 PLOT 1,1,1
40 DRAW 50,50,1"##########,
    "page 13"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_010,
    r##########"10 MODE 1
20 PLOT 100,100,1
30 PLOT 200,200,2
40 PRINT TEST(100,100)
50 PRINT TEST (200,200) RUN 1 2 Le programme ci-dessous recopie un écran sur imprimante EPSON."##########,
    "page 14"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_011,
    r##########"2000 --------------- RECOPIE D'ECRAN SUR IMPRIMANTE EPSON (mode 1)
2004 WIDTH 255
2005 PRINT #8,CHR$(27);"3";CHR$(10)
2010 DEFINT A-Z
2020 DIM LG(320)
2030 LN=399 ligne
2040 FOR LG=1 TO 30
2050 FOR X = 0 TO 319
2060 LG(X)=O
2070 FOR P=0 TO 6
2080 IF TEST(X*2,LN-P*2)>0 THEN LG(X)= LG(X)+(2A(6-P))
2090 NEXT P
2100 NEXT X
2110 PRINT #8,CHR$(27);"K";CHR$ (64);CHR$ ( 1 ) ;
2120 FOR Z = 0 TO 319:PRINT #8 , CHR$ ( LG ( Z)); : NEXT Z
2130 PRINT #8
2140 LN=LN-14
2150 NEXT LG
2160 PRINT #8,CHR$(15)
2170 '-------------------------------------------- pour MODE 2
2180 ' 2080 IF TEST(x ,LN-P*2)>0 THEN ............
2190 ' Prevu pour cable AMSTRAD (adapter 2005 pour autres cables) WINDOW #numéro fenêtre,gauche,droite,haut,bas Définit une fenêtre pour le texte. Dans l’exemple ci-dessous, l’écriture du texte dans la fenêtre 1 se fait en blanc sur fond rouge."##########,
    "page 14"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_012,
    r##########"20 MODE 1
30 INK 0,26
40 INK 1,0
50 INK 2,6
60 INK 3,26
70 '-------fenetre 1
80 WINDOW #1,20,40,1,12 ' fenetre col 20 et 40-ligne 1 et 12
90 PAPER #1,2:CLS #1
100 PEN #1,3 ' stylo 3 pour fenetre 1
110 PRINT #1,"Fenetre 1 "
120 PRINT #1,"Ecriture blanche"
130 PRINT #1 , "sur fond rouge"
140 '---------------fenetre 2 (dans fenetre 1)
150 WINDOW #2,30,38,6,11
160 PAPER #2,0:CLS #2
170 PRINT #7."fenetre 2" Une fenêtre définie à l’intérieur d’une autre n’est pas protégée contre une écriture dans la fenêtre où elle est incluse. La fenêtre par défaut “#0” recouvre tout l’écran. Naturellement, les bornes de cette fenêtre peuvent être changées. INSTRUCTIONS SPÉCIFIQUES AUX CPC 664 ET 6128 Sur CPC 664 et 6128, MOVE possède un paramètre “stylo”. Les instructions PLOT, DRAW et MOVE disposent d'un paramètre “opération” supplé­ mentaire. Ce paramètre égal à 0, 1,2 ou 3 permet de combiner un tracé existant avec un nouveau tracé en utilisant une opération XOR,AND,OR. Par exemple, si vous tracez une droite sur une droite existante avec l’opération XOR(OU exclusif), l'ancien tracé disparaît (0=nul 1=XOR 2=AND 3=OR). PEN possède un paramètre “mode transparent” qui annule l’effet de PAPER."##########,
    "pages 14, 15"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_013,
    r##########"10 MODE 1: INK 0,26:INK 1,0:INK 2,6
20 PAPER 0:PEN 1
30 PRINT "AAAAA"
35 - —
40 PAPER 2
50 PEN 1 ' essayer avec PEN 1,1
60 PRINT "BBBBBB"
70 PAPER 0 FILL stylo Remplit une figure fermée avec le stylo spécifié. Le curseur doit être positionné avec le stylo qui a servi à tracer la figure. En revanche, le stylo spécifié dans FILL peut être différent."##########,
    "page 15"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_014,
    r##########"10 '-----------------FIFLILLL
20 MODE 1
30 CE=1 ' Couleur écriture
40 INK 0,26:INK 1,0:INK 2,20
50 PLOT 100,100,CE
60 DRAWR 50,0,CE
70 DRAWR 0,50,CE
80 DRAWR -50,0,CE
90 DRAWR 0,-50,CE"##########,
    "page 15"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_015,
    r##########"14 I BASIC AMSTRAD
100 '----------------------------------remplissage carre
110 MOVER 10,10,CE ' Positionnement curseur
120 C=1 ' Stylo remplissage
130 FILL C
140 '
150 ' La couleur de remplissage peut etre differente de la couleur du contour
160 ' ex:C=2 MASK masque,premier point Le tracé des droites est effectué avec le pointillé défini en binaire par “masque”. Si “premier point” est égal à 1, celui-ci apparaît."##########,
    "page 16"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_016,
    r##########"10 MODE 1
20 MASK U10101010
30 PLOT 100,100
40 DRAW 200,200,1
50 MASK &X1111000
60 DRAW 300,200,1 COPYCHR$ (#no fenêtre) Donne le caractère affiché sous le curseur."##########,
    "page 16"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_017,
    r##########"10 MODE 1
20 LOCATE 10,10:PRINT "ABC"
30 LOCATE 10, 10:Xt= COPYCHR$(»O)
40 LOCATE 1,20J PR I NT X$ Sur CPC 464, le programme suivant donne le contenu des 8 octets représentant un caractère."##########,
    "page 16"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_018,
    r##########"10 ' COPYCHRt SUR CPC464
20 MODE -2
30 X = 1
40 Y=1
50 LOCATE X,Y:PRINT "ABODE"
60 1
70 FOR A H = 4 9152 + X -1 + ( Y -1 ) ♦ 80 T0 65534 STEP 2048
80 M=PEEK(AM)
90 PRINT BIN*(M,8)
100 NEXT AM ABCDE 00011000 00111100 01100110 01100110 01111110 01100110 01100110 00000000 Ready"##########,
    "page 16"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_019,
    r##########"10 MODE 1
20 INK 0,26:INK 1,0:INK 2,14
30 GRAPHICS PAPER 2
40 MASK &X11110000
50 PLOT 100,100:DRAW 200,200,1 GRAPHICS PEN stylo,mode transparent Définit le stylo utilisé par défaut pour les instructions graphiques. Si “mode transparent” est égal à 1, l’effet de GRAPHICS PAPER est annulé. FRAME Synchronise l’affichage des points d’un caractère affiché avec TAG. CURSOR mode Lorsque INKEY$ est utilisé, le curseur est apparent pour CURSOR 1 et invisible pour CURSOR 0. EFFACEMENT D’UN DESSIN Pour effacer un dessin il suffit de tracer le même dessin en utilisant le stylo 0."##########,
    "page 17"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_020,
    r##########"10 '-------------effacement d'un dessin
20 MODE 1:PAPER 0:PEN 1
30 INK 0,26 ' blanc dans case 0
40 INK 1,0 ' noir dans case 1
50 '--------- droite noire sur fond blanc
60 PLOT 100,100,1
70 DRAW 200,200,1 ' droite avec case 1"##########,
    "page 17"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_021,
    r##########"90 FOR TP=1 T0 2000:NEXT TP ' temporisation
100 '----- effacement
110 PLOT 100,100,0
120 DRAW 200,200,0 ' droite avec case 0 CHANGEMENT DE LA COULEUR D’UN DESSIN Pour changer la couleur d’un dessin il suffit de modifier par INK la couleur de la case (stylo) qui a servi à tracer le dessin. On peut également de la même façon faire dispa­ raître momentanément un dessin sans l’effacer."##########,
    "page 17"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_022,
    r##########"10 '-------comment changer la couleur d'un dessin
20 MODE IsPAPER 0:PEN 1
30 INK 0,26 ' blanc dans case 0
40 INK 1,0 ' noir dans case 1 —►"##########,
    "page 17"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_023,
    r##########"16 I BASIC AMSTRAD
50 '------- carre noir sur blanc
60 PLOT 100,100,1
70 DRAWR 100,0,1
80 DRAWR 0,100,1
90 DRAWR -100,0,1
100 DRAWR 0,-100,1
110 '-------
120 FOR TP=1 TO 2000:NEXT TP ' temporisation
130 '---------------le carre devient bleu
140 INK 1,2 ' bleu dans case 1
150 FOR tp=l TO 2000:NEXT tp
160 '---------------le carre devient invisible
170 INK 1,26 ' blanc dans case 1
180 FOR tp=l TO 2000:NEXT tp
190 '---------'■-------le revoi 1 a
200 INK 1,6 ' rouge dans case 1 Pièges : Lorsque vous utilisez un programme la table de correspondance des couleurs est dans l’état où le programme précédent l’a laissée. Il est donc prudent d’initialiser la table des couleurs en début de programme ou bien d’appuyer sur “CTRL /SHIFT/ESC/”. Le programme ci-dessous trace, en MODE 1, trois droites avec les stylos 1,2 et 3. En MODE 2, la seconde droite n’apparaît pas puisque le stylo 2 est en réalité le stylo 0."##########,
    "page 18"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_024,
    r##########"10 '-----------PIEGE NO 1
20 MODE 1
30 INK 0,26 blanc dans case 0
32 INK 1,0 ' noir dans case 1
34 INK 2,2 ' bleu dans case 2
40 '---------droite noire sur fond blanc
50 PLOT 100,100,1
60 DRAW 200,200,1
70 '-----------droite bleue sur fond blanc
80 DRAW 300,200,2
90 '-------------droite rouge sur fond blanc
100 DRAW 300,300,3
110 '--­
120 ' essayez ce programme en MODE 2
130 ' la droite trace avec le stylo 2 n'apparait pas:
140 ' En mode 2,1e stylo 2 est le meme que 0. Le programme ci-dessous utilise le stylo 2 pour écrire en MODE 1. En passant en MODE 2, le texte n’apparaît plus puisque PEN 2 est équivalent à PEN 0. Il suffit de frapper PEN 1 pour faire apparaître le texte."##########,
    "page 18"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_025,
    r##########"10 '------------------- PIEGE N0 2 2.0 MODE 1
30 INK 0,26:INK 1,0: INK 2,2
40 PEN 1
50 PRINT "J ’ECRIS EN NOIR"
60 PEN 2
70 PRINT "J 'ECRIS EN BLEU"
80 '
90 PRINT "ATTENTION,TOUT VA DISPARAITRE"
100 PRINT "POUR FAIRE APPARA ITRE,FRAPPE Z :PEN 1"
110 FOR TP=1 TO 4000:NEXT TP ' TEMPORISATION
120 MODE 2
130 '
140 ' EN MODE 2 ,'PEN 2' EST EQUIVALENT A 'PEN 0' Ci-dessous, nous plaçons dans la case 1 la couleur de la case O pour rendre invisible une droite ; le texte disparaît en même temps. Pour le faire apparaître à nouveau, frapper INK 1,0."##########,
    "pages 18, 19"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_026,
    r##########"10 •-------------PIEGE NO 3
20 MODE 1
30 INK 0,26:INK 1,0
40 '---------------droite noire sur -fond blanc
50 PLOT 100,100,1
60 DRAWR 100,0,1
70 '-------
80 PRINT "Frapper : INK 1,0"
90 FOR TP=1 TO 2000:NEXT TP ' temporisation
100 '---------------droite invisible
110 INK 1,26
120 '
130 ' -frapper en mode direct: INK 1,0 Nous présentons ici quelques exemples de dessins. Le programme ci-dessous trace un drapeau français. Nous initialisons d’abord la table des couleurs puis nous traçons trois rectangles."##########,
    "page 19"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_027,
    r##########"10 ' DRAPEAU FRANÇAIS 20 30
40 1MODE 1 •
50 INK 0,21 ' vert dans case 0
60 INK 1,2 ’ bleu dans case 1
70 INK 2,26 ' blanc dans case 2
80 INK 6,3 ' rouge dans case 3 90
100 XA=100:YA=100 ' coordonnées depart
110 H=150 :L=100 ' h auteur/1ongueur rectangle
120 CE=1 ' bleu
130 GOSUB 220
140 CE = 2 ' blanc
150 XA=XA+L
160 GOSUB 220
170 CE=3 ' rouge
180 XA=XA+L
190 GOSUB 220
200 END
210 ------------- .---------- Trace d'un rectangle
220 FOR Y = YA TO YA + H
230 PLOT XA,Y:DRAWR L,O,CE
240 NEXT Y
250 RETURN Le dessin de soleil ci-dessous est obtenu en traçant un cercle plein puis des rayons aléatoires."##########,
    "pages 19, 20"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_028,
    r##########"10 ------------------------------------SOLEIL
20 MODE 1
30 INK 0,26:INK 1,6:PAPER O:PEN 1
40 R=40 ' rayon
50 XC=200:YC=200 ' centre
60 GOSUB 160
70 '---------RAYONS .. ’i I//
80 FOR A = 0 TO 2*PI STEP PI/20
90 R1=R + RND(1)*R
100 X=XC+R1*COS (A) " ~q|M
110 Y = YC + R1*SIN(A) ■-■■■"T’ill
120 PLOT XC,YC, 1:DRAW X,Y,1
130 NEXT A
140 END
150 '--------------------------------------CERCLE
160 FOR A=0 TO 2*PI+0.1 STEP 1/R
170 X=XC+R«COS(A)
180 Y=YC+R*SIN(A)
190 PLOT XC,YC:DRAW X,Y,1
200 NEXT A
210 RETURN Pour tracer des cercles rapidement nous calculons les valeurs des sinus et cosinus dans des tables C() et S()."##########,
    "page 20"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_029,
    r##########"10 ’ TRACE DE CERCLE RAPIDE
20 '
30 MODE 1
40 INK 0,26:INK 1,6
50 PRINT "JE CALCULE LES COSINUS ET SINUS"
60 '
70 NP=20 ' NOMBRE DE POINTS
80 DIM CI360),S(360)
90 FOR J=0 TO 360 STEP 360/NP
100 A=2*PI*J/360
110 C(J)=COS(A): S(J)=SIN(A)
120 NEXT J
130 ' —
140 CE=1
150 FOR N=1 TO 10
160 XC=1OO+RND(1)*300
170 YC=1OO+RND(1)*200
180 R=40+RND( 1 )*30
190 GOSUB 230
200 NEXT N
210 END
220 '---------------CERCLE
230 PLOT XC+R,YC, 1
240 FOR A=O TO 360 STEP 360/NP
250 X=XC+R*C(A)
260 Y=YC+R*S(A)
270 DRAW X,Y,CE
280 NEXT A
290 RETURN JE CALCULE LES COSTHUS ET SINUS R ead y CARACTÈRES ACCENTUÉS _____________________________ Les caractères accentués ne sont pas prévus sur l’AMSTRAD. Ils peuvent être ajoutés en redéfinissant des caractères avec l’instruction SYMBOL. Si les caractères accentués doivent être édités sur imprimante, il faut utiliser les codes ASCII adaptés à l’imprimante. Par exemple, pour une imprimante DPMI, le code de “à accent” est 64. On redéfinit donc le caractère de code ASCII 64. En revanche, les touches au clavier peuvent être choisies. Ci-dessous nous utilisons la touche “1 ” du clavier numérique pour frapper le “à accent”."##########,
    "pages 20, 21"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_030,
    r##########"10 =============================== ACCENTS
30 ' redefinition du caractère 'A'
40 ' 1 clavier numérique
50 '
55 SYMBOL AFTER 64
60 SYMBOL 64,96,16,120,12,124,204,118
70 KEY 129,CHR$(64) Ci-dessous nous utilisons “,CTRUA” pour frapper “à accent”."##########,
    "page 21"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_031,
    r##########"10 '=============================== ACCENTS
30 ' redefinition du caractère 'A'
40 ' CTRL A --> a accentue
50 '
55 SYMBOL AFTER 64
60 SYMBOL 64,96,16,120,12,124,204,118
70 KEY DEF 69,1,97,65,64"##########,
    "page 21"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_032,
    r##########"10 ==================== accents
20 SYMBOL AFTER 64
30 '-----7 e accent grave
40 SYMBOL 125,96,16,60,102,126,96,60
50 KEY 135,CHR$(125)
60 '-------0 accent aigu
70 SYMBOL 123,6,8,60,102,126,96,60
80 KEY 128 , CHRÎ(123)
90 '-------1 a accent grave
100 SYMBOL 64,96,16,120,12,124,204,118
110 KEY 129,CHR$(64)
120 '-----4 c cedille
130 SYMBOL 92,0,0,60,102,96,62,8,24
140 KEY 132,CHR$(92) L’exemple ci-dessous inverse les caractères “A” et “Q” au clavier."##########,
    "page 22"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_033,
    r##########"10 KEY DEF 67,1,97,65
20 KEY DEF 69,1,113,82 Le programme ci-dessous permet de dessiner un caractère et donne les valeurs déci­ males à spécifier dans l’instruction SYMBOL."##########,
    "page 22"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_034,
    r##########"10 -----------------GENERATEUR DE CARACTERES 8X8
20 MODE 1:PAPER O:PEN 1 : I NK 0,26:INK 1,0
30 DIM T(9,9)
40 SYMBOL AFTER 134
50 SYMBOL 134,255,129,129,129,129,129,129,255 carre
60 FOR Y=1 TO 8:F0R X=1 TO 8
70 LOCATE X,Y:PRINT CHR$(134)
80 NEXT X:NEXT Y
90 LOCATE 1,12:PRINT "Fléchés pour déplacer"
100 LOCATE 1,13:PRINT "A:allumer E:effacer V:valeurs dec."
110 LOCATE 1 ,15:PRINT "F:fin"
120 X=5:Y=5
130 CS=1
140 '---------------------CURSEUR CLIGNOTANT
150 C$=INKEY$:IF LEN(C$)<>0 THEN 210
160 LOCATE X,Y:IF T(X,Y)=1 THEN PRINT CHR$(143) ELSE PRINT CHR$( 231)
170 FOR TP=1 TO 40:NEXT TP
180 LOCATE X,Y:PRINT CHRK134)
190 GOTO 150
200 -
210 LOCATE X,Y: IF T(X,Y)=1 THEN PRINT CHR$(143) ELSE PRINT CHRt ( 134)
220 '
230 C$=UPPER$(Ct)
240 C=ASC(Ct)
250 IF C=242 THEN IF X>1 THEN X=X-1 ,
260 IF C = 243 THEN IF X<8 THEN X = X +1
270 IF C = 241 THEN IF Y<8 THEN Y = Y+1
280 IF C = 240 THEN IF Y>1 THEN Y = Y-1
290 IF C$="A" THEN LOCATE X,Y: PRINT CHRt(143): T(X,Y)=1
300 IF C$="E" THEN LOCATE X,Y: PRINT CHR$(134):T(X,Y)=0
310 IF C$="V" THEN 350
320 IF C$="F" THEN END
330 GOTO 150
340 '---------------------------------------- calcul valeurs décimales
350 FOR L=1 TO 8 '8 lignes
360 ND=O ' valeur décimale
370 FOR X = 1 TO 8 ' 1 caractère
380 A = 0 :IF T(X,L)=1 THEN A=1
390 ND = ND + A*2A(8-X ) ' A=Fleche haut
400 NEXT X
410 LOCATE lO.LsPRINT ND SPC(l)
420 D(L)=ND
430 NEXT L
440 SYMBOL 135 , D ( 1 ) , D ( 2 ) , D ( 3 ) , D ( 4 ) , D ( 5 ) , D ( 6 ) , D ( 7 ) , D ( 8 )
450 LOCATE 15,2:PRINT CHRI(135)
460 X=5:Y=5
470 GOTO 150 F 1 c c li c s pour déplacer A:allurier E : effacer Uivaleurs dec. F : F i iii ANIMATION ___________________________________________ Pour animer des figures on utilise généralement des caractères redéfinis. En affichant un caractère dans différentes positions de l’écran, on obtient un effet d’animation. Pour simuler un vol de papillon nous le représentons alternativement dans deux posi­ tions. Chaque position est représentée par 16x16 points, soit quatre caractères gra­ phiques. CHR$(8), CHR$(10) et CHR$(11 ) permettent de déplacer le curseur à gauche, en bas et vers le haut. Ainsi le papillon est affiché avec une seule instruction PRINT."##########,
    "pages 22, 23"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_035,
    r##########"10 ' VOL DE PAPILLON
20 '
30 MODE 1:INK 0,26:INK 1,6
40 C=145 ' 1ER CARACTERE A MODIFIER
50 SYMBOL AFTER C
60 '------------------- POSITION 1
70 SYMBOL C,0,0,0, 127,63,31 , 15,63
80 SYMBOL C+l ,0,0,0,248,240,226,196,248
90 SYMBOL C + 2,15,31,63,127,0,0,0,0
100 SYMBOL C+3,196,226,240,248,0,0,0,0
110 P1$ = CHR$ (C) + CHR$ (C+l ) + CHRÎ- (10)+CHR$(8)+CHRS(8)+CHR$(C+2)+CHR$ (C+3)+CHRI(11)
120 LOCATE 10,2:PRINT L$
130 '---------------POSITION 2
140 SYMBOL C + 4,0,0,0,0,0,31,15,63
150 SYMBOL C + 5,0,0,0,0,0,226,196,248
160 SYMBOL C + 6, 15,31 ,0,0,0,0,0,0
170 SYMBOL C + 7, 1 96,226,0,0,0,0,0,0
180 P2$=CHR$(C+4)+CHRÎ(C+5)+CHR$(10)+CHR$(8)+CHR$(8)+CHR$(C+6)+CH R$ (C + 7)+CHRI(11)
190 '
200 ---------------CHAINE EFFACEMENT
210 EF$=CHR$(32)+CHRI(10)+CHR$(8)+CHR$(32)+CHR$(11)
220 '-------AVANCE PAPILLON
230 Y=10
240 FOR X = 1 TO 24
250 LOCATE X,Y:PRINT EF$;P1$
260 FOR TP=1 TO 100:NEXT TP
270 LOCATE X,Y:PRINT EF$;P2$
280 FOR TP=1 TO 80:NEXT TP
290 NEXT X
300 LOCATE X ,Y:PRINT EF$;EF$
310 INK 1 , I NT(RND(1)*16)
320 GOTO 240 Positionl Voici deux exemples de dessins définis par quatre caractères graphiques (16x16 points)."##########,
    "page 24"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_036,
    r##########"10 '-------soleil
20 '
30 MODE 1: INK 0,1:INK 1 ,24
40 C = 1 4 5 ' 1ER CARACTERE A MODIFIER
50 SYMBOL AFTER C
60 SYMBOL C,1,65,33,17,11,7,239,31
70 SYMBOL C+l,0,8,16,32,192,192,224,254
80 SYMBOL C+2,15,7,27,34,66,2,2,0
90 SYMBOL C+3,224,192,160,16,8,0,0,0
100 LI=CHRI(C)+CHR$(C+l)+CHR*(10)+CHRI(8)+CHR$(8)+CHR*(C+2)+CHR$( C+3)+CHR$(11)
110 LOCATE 10,2:PRINT L$"##########,
    "page 24"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_037,
    r##########"10 '-----------bateau
20 '
30 MODE 1 : INK 0,1:INK 1,24
40 0=145 ' 1ER CARACTERE A MODIFIER
50 SYMBOL AFTER C
60 SYMBOL C,0,0,0,0,2,2,6,6
70 SYMBOL C+l,0,0,0,0,128,128,192,224
80 SYMBOL C + 2,14,30,62,0,255, 127,63,0
90 SYMBOL C + 3,240,248,252,0,255,254,252,0
100 L$ = CHR$ (C) +CHRKC+1 ) + CHR$ (10)+CHRI(8)+CHR$(8)+CHRI(C+2)+CHR$( C+3)+ CHR$(11)
110 LOCATE 10,2:PRINT L$ Pour réaliser une animation sans caractères graphiques nous dessinons une figure dans différentes positions et n’en “démasquons” qu’une seule à la fois."##########,
    "page 25"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_038,
    r##########"10 ' ROTATION ROUE 20
30 On représente successivement une roue
40 dans 3 positions pour donner une
50 impression d'animation.
60 Attention! si vous interrompez le programme , Frappez en
70 ' mode direct: INK 1,0
80 '
90 CF=26 ' Fond
100 CE = 6 ' écriture.
110 MODE 1:PAPER O:PEN 1 !..
120 INK O,CF:INK 1,CF
130 INK 2, CF:INK 3, CF —
140 XA=200:YA=20Q ' centre
150 R= 150 ' rayon "û-
160 GOTO 180
170 '-------------dessin positions 1,2,3’
180 NR=30 ' Nombre de rayons
190 V=200 ' vitesse
200 DC=O:S=2:GOSUB 330
210 DC=(2*PI)/(3*NR):S=3:GOSUB 330
220 DC=DC*2:S=1:GOSUB 330
230 '-------------rotation roue
240 INK 1,CF : INK 3,CE ' position 1
250 FOR TP=1 TO V:NEXT TP
260 INK 2,CE:INK 3,CF ' posi ti on 2
270 FOR TP=1 TO V:NEXT TP ■y
280 INK 2,CF:INK 1,CE ' positi on
290 FOR TP=1 TO V:NEXT TP
300 V=V-2:IF V=0 THEN END
310 GOTO 240
320 ■-------------------------RAYONS
330 FOR P=1 TO NR
340 A=2»PI*P/NR+DC
350 DX = R*COS(A):DY = R*SIN (A)
360 PLOT XA+DX*O.3,YA+DY*0.3,S:DRAW XA+DX,YA+DY,S
370 NEXT P
380 RETURN"##########,
    "page 25"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_039,
    r##########"10 ■-----------------------AGRANDISSEMENT D’ UNE FIGURE
20 MODE 1
30 CF = O ’ -fond
40 CE = 26 ' écriture
50 INK O,CF
60 '
70 XC=200:YC=200 ' ORIGINE
80 R=10
90 '-------
100 S T Y L 0 = 1 : INK 1,CF :GOSUB 260 ' 1ER FIGURE
110 '
120 INK 1 ,CE:INK 2,CF DEMASQ 1ERE/MASQUAGE 2EME
130 '
140 STYL0=0:R=R-4:XC=XC+2: YC=YC+2: GOSUB 260 'EFFACEMENT 2ME
150 '
160 STYL0=2:INK 2,CF:R=R+8 :XC=XC-4:YC=YC-4: GOSUB 260 ' 2EME FIG URE
170 '
180 INK 1,CF: INK 2,CE MASQUAGE 1 ER,DEMASQUAGE 2EME
190 '
200 STYLO=O:R=R-4:XC=XC+2: YC=YC+2: GOSUB 260 ' EFFACEMENT 1ER
210 '
220 R=R+8:XC=XC-4:YC=YC-4
230 IF R>150 THEN INK 1,CE:END
240 GOTO 100
250 - ------- FIGURE
260 PLOT XC, YC,STYLO
270 DRAWR R, 0,STYLO
280 DRAWR 0, R,STYLO
290 DRAWR -R ,0,STYLO
300 DRAWR 0, -R,STYLO
310 RETURN Un éclair est dessiné en mode masque. Nous le démasquons momentanément et en même temps nous changeons la couleur de fond."##########,
    "page 26"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_040,
    r##########"10 ' ECLAIR 20
30 MODE 1
40 PAPER 0:PEN 1:BORDER 1
50 INK 0,1: INK 1,26:INK 2,0:INK 3,0
60 GOSUB 250 ' OPTIONNEL
70 DESSIN ECLAIR
80 PLOT 200,300,2
90 DRAW 234,340,2
100 DRAW 232,314,2
110 DRAW 280,362,2
120 —
130 INK 2,6 ' ECLAIR APPARENT
140 FOR TP=1 TO 100:NEXT TP
150 INK 0,2:B0RDER 2 160
170 FOR TP=1 T0 100:NEXT TP
180 —
190 INK 2,1 " ECLAIR MASQUE
200 INK 0,1:BORDER 1
210 -
220 FOR TP=1 T0 3000:NEXT TP
230 GOTO 130
240 ----------------------------- IMMEUBLE
250 C=125 ' caractère a modifier
260 C$=CHR$(C)
270 SYMBOL AFTER C
280 SYMBOL 0,255,129,129,129,129,129,129,255 290
300 XB = 1 :YB = 23
310 H=15 :L=7
320 PEN 3
330 FOR Y=YB TO YB-H STEP-1
340 FOR X = XB TO XB+L
350 LOCATE X,Y:PRINT 0$
360 NEXT X
370 NEXT Y
380 PEN 1
390 RETURN
400 ■--------------------------- sur écran Vert;
410 ' 50 ink 0,l:ink 1,26:ink 3,20
415 ' 130 ink 2,24
420 ' 150 ink 0,12:border 12 DESSINATEUR _________________________________________ Le dessinateur présenté permet de choisir un caractère parmi ceux affichés en haut de l’écran à l'aide d’un curseur. Lorsque le caractère est choisi vous réalisez un dessin en déplaçant le curseur avec les quatre flèches. “L” permet de se déplacer sans écriture, “E” d’effacer une partie des dessins. Pour dessiner à nouveau, utiliser “B”. Fléchés pour déplacer P :prend B : Jb-a i sser L:leuer E:effacer Coule uic s : 1,2,3 TJ a IN"##########,
    "pages 26, 27"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_041,
    r##########"10 '---------DESSIN (CPC464 ET CPC664) 20CE=l:CF=0 ' ecriture/Fond
30 MODE 1
40 INK 0,26:INK 1,0:INK 2,2:PAPER O:PEN 1
50 CC=139 ' debut caractères graphiques
60 SYMBOL AFTER CC
70 SYMBOL 00,255,129,129,129,129,129,129,255
80 SYMBOL CC+1,8,8,28,127,28,34,65,0
90 SYMBOL CC + 2,255,129,129,255,0,0,0,0
100 SYMBOL CC + 3,255,129,129,255,129,129,129,255
110 X=10:Y=10 ' ccordonnees curseur
120 '
130 XM=1:YM = 1 ' af-fichage caractères de base
140 LOCATE XM,YM:FOR 1=00 TO CC+18:PRINT CHR$(I) SPC ( 1 ) .-NEXT I
150 CP$ = CHR$(00):L = 1 : LOCATE X,Y:PRINT CP$
160 '
170 LOCATE 1,20:PRINT "Fléchés pour déplacer"
180 LOCATE 1,21:PRINT "P:prend"
190 LOCATE 1,22:PRINT "Bîbaisser L:lever E:effacer F:fin"
200 LOCATE 1,23:PRINT "Cou 1eurs:1,2,3" 2io '------------------------- CURSEUR
220 LOCATE X,Y
230 XG= (X-l )*16+8:YG = 399-(Y-l )*16-8
240 T=TEST(XG,YG):TB=TEST(XG+2,YG)
250 C$=INKEY$:IF LEN(C$)<>0 THEN 290
260 PLOT XG , YG,CE : PLOT XG + 2 , YG,CE:PLOT XG , YG,CF : PLOT XG + 2,YG,CF
270 GOTO 250
280 '---------
290 PLOT XG,YG,T:PLOT XG + 2,YG,TB
300 C$=UPPER$(C$)
310 C=ASC(C$)
320 IF C=242 THEN IF X>1 THEN X = X-1
330 IF C=243 THEN IF X<38 THEN X = X + 1
340 IF C=240 THEN IF Y>1 THEN Y = Y-1
350 IF C = 241 THEN IF Y<24 THEN Y=Y+1
360 IF C$>="1" AND C$< = "3" THEN CE = VAL(C$): PEN CE
370 IF C$="P" THEN GOSUB 470
380 IF C$="B" THEN L=1
390 IF C$="L" THEN L=0
400 IF C$="E“ THEN L=2
410 IF C$="F" THEN END
420 IF L=1 THEN IF Y>YM+1 THEN LOCATE X,Y:PRINT CPI
430 IF L = 2 THEN IF Y>YM+1 THEN LOCATE X,Y:PRINT CHR$(32)
440 LOCATE 1 ,16: PR I NT c$
450 GOTO 220
460 '---------------ON PREND
470 IF Y = YM THEN CP$ = CHRI(CC+1 NT ( ( X-XM)/2))
480 LOCATE 1,18:PRINT CP$:PRINT CHR$(7);
490 RETURN
500 ■------------------------------------------------------
510 ' Pour CPC664:
520 ' Ajouter : 115 CURSOR 1
530 ' Supprimer: 230 240 260"##########,
    "page 28"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_042,
    r##########"10 '---------SOURIS (CPC664)
20 CE=1:CF=O ' ecriture/fond
30 MODE 2
40 INK 0,26:INK 1,O:PAPER 0:PEN 1
50 X=10:Y=10 ' coordonnées curseur
60 CURSOR 1
70 LOCATE 1,5:PRINT "DEPLACEZ LE CURSEUR AVEC LES FLECHES"
80 LOCATE 1,6:PRINT "’PRENEZ' UNE LETTRE EN APPUYANT SUR ’P'"
90 LOCATE 1,7:PRINT "DEPLACEZ LE CURSEUR PUIS 'POSEZ' LA LETTRE EN APPUYANT SUR 'D'H
100 LOCATE 1,24 : PR I NT "F: fin"
110 ■----------------------------------------CURSEUR
120 LOCATE X,Y
130 C$=INKEY$:IF LEN(CI)<>0 THEN 160
140 GOTO 130 150
160 C=ASC(CI)
170 C$=UPPER$(C*)
180 IF C = 242 THEN IF X>1 THEN X = X-1
190 IF C = 243 THEN IF X< 79 THEN X = X + 1
200 IF C = 240 THEN IF Y>1 THEN Y = Y-1
210 IF C = 241 THEN IF Y<25 THEN Y = Y+1
220 IF C*="P" THEN GOSUB 270
230 IF C*="D" THEN GOSUB 320
240 IF C*=“F" THEN END
250 GOTO 120
260 '---------------ON PREND
270 CP$=COPYCHR$(#0)
280 PRINT CHR$(32)
290 LOCATE 1,20 : PR INT CP*
300 RETURN
310 ■-------------------0N p0SE
320 IF COPYCHR*(#0)<>CHR*(32) THEN RETURN
330 PRINT CP*
340 RETURN DEPLACEZ LE EUR AVEC LES FLECHES 'PRENEZ' UNE LETTRE EN APPUYANT SUR 'P' DEPLACEZ LE CURSEUR PUIS 'POSEZ' LA LETTRE EN APPUYANT SUR 'D' C U R S"##########,
    "page 29"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_043,
    r##########"10 '---------SOURIS (CP464+CPC664)
20 CE=1:CF=O ' eeccrrii ttuurree//ffond
30 MODE 2
40 INK 0,26:INK 1,0:INK 2,2: INK 3,6:PAPER O:PEN 1
50 X=10:Y=10 ' coordonnées curseur
60 '
70 LOCATE 1,5:PRINT "DEPLACEZ LE CURSEUR AVEC LES FLECHES"
80 LOCATE 1,6:PRINT "'PRENEZ' UNE LETTRE EN APPUYANT SUR 'P'"
90 LOCATE 1 ,7:PRINT "DEPLACEZ LE CURSEUR PUIS 'POSEZ' LA LETTRE EN APPUYANT SUR 'D' "
100 LOCATE 1,24 : PR I NT "F:fin"
110 '------------------- curseur clignotant
120 LOCATE X,Y
130 GOSUB 470 ' Sauvegarde caractère sous le curseur
140 '
150 LOCATE X,Y
160 C$=INKEY$:IF LEN(C$)<>0 THEN 210
170 PRINT CHR$(143)
180 GOSUB 530
190 GOTO 150
200 '
210 GOSUB 530 ' Restitution caractère sous curseur
220 C$=UPPER$(C*)
230 C=ASC(C$)
240 IF C=242 THEN IF X>1 THEN X=X-1
250 IF C=243 THEN IF X<78 THEN X=X+1
260 IF C=240 THEN IF Y>1 THEN Y = Y-1
270 IF C=241 THEN IF Y<25 THEN Y=Y+1
280 IF C$="P" THEN GOSUB 330
290 IF C$="D" THEN GOSUB 410
300 IF C$="F" THEN END
310 GOTO 120
320 '---------------ON PREND
330 N=1
340 FOR A=49152+X-l+(Y-l)*80 TO 65534 STEP 2048
350 XP(N)=PEEK(A):N=N+1
360 NEXT A
370 PRINT CHR$(32)
380 PRINT CHR$(7) ;
390 RETURN
400 '-------------------ON POSE
410 N=1
420 FOR A=49152+X-l+(Y-l)*80 TO 65534 STEP 2048
430 POKE A , XP(N):N = N+1
440 NEXT A
450 RETURN
460 '------------------------- sauvegarde caractère sous curseur
470 N=1
480 FOR A=49152+X-l+(Y-l)*80 TO 65534 STEP 2048
490 X(N)=PEEK<A):N=N+1
500 NEXT A
510 RETURN
520 '--------------------------- restitution caractère _ _
530 N=1
540 FOR A=49152+X-1+(Y-1)*80 TO 65534 STEP 2048
550 POKE A,X(N):N=N+1
560 NEXT A
570 RETURN STOCKAGE D’UN DESSIN DANS UN FICHIER Nous réalisons un dessin à l’aide de segments de droites. Les coordonnées des points sont stockées dans un fichier séquentiel sur cassette ou disquette."##########,
    "pages 30, 31"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_044,
    r##########"10 '--------------- STOCKAGE D'UN DESSIN DANS UN FICHIER 20CE=l:CF=0 ' ecriture/fond
30 MODE 1
40 INK 0,0:INK 1,26:INK 2,2:INK 3,6:PAPER 0:PEN 1
50 LOCATE l,20:PRINT "PREMIER PO I NT :f1eches puis 'P"’
60 LOCATE 1,21:PRINT "AUTRES POINTS: fléchés puis 'D'"
70 LOCATE 1,22:PRINT "1,2,3 : cauleurs "
80 LOCATE 1,23:PRINT "F.-FIN"
90 X=200:Y=200
100 INPUT "NOM FICHIER ";NF$
110 OPENOUT NFS
120 '---------------------------------- CURSEUR CLIGNOTANT s------------■.
130 T=TEST(X,Y) \-----------\
140 ' fï------------/
150 C$=INKEYI:IF LEN(C$)<>0 THEN 200
160 PLOT X,Y,CE ,
170 PLOT X , Y, CF L____—l'"''
180 GOTO 150
190 '
200 PLOT X,Y,T
210 C=ASC(C$)
220 CI=UPPERI(C$)
230 IF C=242 THEN X = X-2
240 IF C=243 THEN X=X+2
250 IF C=240 THEN Y=Y+2
260 IF C=241 THEN Y = Y-2
270 LOCATE 1,17:PRINT Cl
280 IF CI="P" THEN PLOT X,Y,CE:GOSUB 340 : XA = X : YA = Y
290 IF CI="D" THEN PLOT X A,YA,CE :GOSUB 340:DRAW X,Y,CE : YA = Y : XA=X
300 IF CI>="0" AND Cl<="3" THEN CE=VAL(Cl):GOSUB 340
310 IF CI="F" THEN CLOSEOUT:END
320 GOTO 130
330 '----------------------- STOCKAGE FICHIER
340 PRINT #9,Cl
350 PRINT #9 ,X
360 PRINT #9,Y
370 RETURN"##########,
    "page 31"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_045,
    r##########"10 ■-----------------LECTURE FICHIER DESSIN
20 MODE 1
30 CF=O:CE=1 ' fond/ecriture
40 INK O,O:INK 1,26:INK 2,2: INK 3,6
50 '
60 INPUT "NOM FICHIER ";NF$
70 OPENIN NF*
80 '
90 '
100 IF EOF THEN CLOSEIN:END
110 INPUT #9,C*-,X,Y
120 IF C*="P" THEN PLOT X,Y,CE
130 IF C*="D" THEN DRAW X, Y,CE
140 IF C*>="0" AND C*<="3" THEN CE=VAL(C*)
150 GOTO 100 DESSINATEUR ET COMPOSITION DE DESSINS Nous reprenons le dessinateur présenté dans le tome 1, page 127, mais cette fois nous enregistrons toutes les commandes et les coordonnées des points dans des tables CM$(),X(),Y() etCE(). CM$() X() Y() CE() P 81 371 1 D 81 315 1 D 47 315 1 D 81 371 1 F 77 343 1 Cette méthode présente plusieurs avantages : □ Elle permet d’exécuter à nouveau un dessin en supprimant une ou plusieurs commandes. □ La sauvegarde du dessin est moins encombrante et plus rapide qu’une sauvegarde binaire. □ Le dessin réalisé peut être agrandi. □ Nous pouvons ajouter plusieurs dessins. Le dessin se fait dans une fenêtre en haut à gauche de l’écran. Par exemple, pour tracer une droite, positionnez le curseur avec les flèches puis frappez “P”. Ensuite déplacez le curseur et frappez “D”. Pour tracer un rectangle, frappez “P” puis déplacez le curseur vers le sommet opposé et frappez “R”. Lorsque le dessin est réalisé, vous le sauvegardez en frappant Vous pouvez ensuite l’appeler avec “$”. Il est affiché dans la fenêtre. Pour l’afficher dans un autre endroit de l’écran, utiliser “%”. Auparavant, positionnez le curseur à l’endroit où doit être affiché le dessin. “+” et permettent d’augmenter et de diminuer la vitesse de déplacement du curseur."##########,
    "page 32"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_046,
    r##########"10 ' COMPOSITION DE DESSINS
20 '
30 GOSUB 1400 ' Affichage menu
40 DIM X(200),Y(200),CM$(200),CE(200)
50 DIM XX (200),YY(200)
60 CE=1:CF=O ' couleur écriture et fond
70 INK 0,26:INK 1,O:PAPER CF:PEN CE ' fond blanc/encre noire
80 INK 2,2:INK 3,6
90 RPT = 3
100 SPEED KEY 20,3 ' repetition plus rapide
110 MODE 1
120 PLOT 1 , 399,1 :DRAWR 200,0:DRAWR 0,-150:DRAWR -200,0
130 WINDOW #1,27,40,1,3
140 WINDOW #2,1,12,1,9
150 PRINT #2,"Des de base"
160 XA=100:YA=350 ' point precedent
170 XB=XA:YB=YA:X=XA+20:Y=YA
180 LOCATE 1,24:PRINT "P:point D:droite R:rect Czcercle Ystgle"
190 LOCATE 1,25:PRINT " cou 1 : 0,1 , 2,3 F:peindre #:sauv $:lect"
200 '----------------------------------curseur clignotant
210 T=TEST(X,Y)
220 '
230 C$=INKEY$: IF LEN(C$)<>0 THEN 270
240 PLOT X,Y,CE:PLOT X,Y,CF
250 GOTO 230
260 '
270 PLOT X,Y,T
280 C = ASC(CI)
290 IF C=242 THEN IF X>2 THEN X=X-2:G0T0 210
300 IF C=243 THEN IF X<600 THEN X=X+2:G0T0 210
310 IF C=240 THEN IF YC397 THEN Y=Y+2:G0T0 210
320 IF C=241 THEN IF Y>2 THEN Y=Y-2:G0T0 210
330 Ct=UPPERI(Ct):LOCATE 1,21:PRINT C#
340 IF C$=" + " THEN IF RPT>1 THEN RPT = RPT-1 : SPEED KEY 20,RPT
350 IF Cl="-" THEN IF RPT<6 THEN RPT = RPT+1 : SPEED KEY 20,RPT
360 IF C$="!" THEN TT = ABS(TT-1):GOSUB 770-.G0T0 210
370 IF TT=1 THEN TAG:PLOT X,Y,1:PRINT C$;:X=X+16:TAGOFF:GOSUB 77 OîGOTO 210
380 IF C$="P" THEN PLOT X,Y,CE : XB = X A :YB = YA : XA = X : YA = Y :GOSUB 770
390 IF C$="D“ THEN PLOT X A,YA,CE : DRAW X,Y,CE : XB = X A :YB = YA : YA = Y : XA = X: 1GOSUB 770
400 IF C$="C" THEN GOSUB 600:G0SUB 770
410 IF C$="R" THEN GOSUB 550:XA=X:YA=Y:GOSUB 770
420 IF C$="Y" THEN GOSUB 680 : XB = XA :YB = YA : XA = X:YA = Y :GOSUB 770
430 IF C$="G" THEN AC=CE:CE=O:GOSUB 550:CE = AC:GOSUB 770 ' gomme
440 IF C$>="0" AND C$<="9" THEN CE=VAL(C$)
450 IF C$="F" THEN MOVE X, Y,CE:FILL CE:GOSUB 770 ' CPC664
460 IF C$="S" THEN SAVE "DES",B,&C000,&4000
470 IF C$="L" THEN LOAD "DES"
480 IF C$="#" THEN GOSUB 800 ' sauvegarde dessin de base
490 IF Ct="$" THEN GOSUB 920 ' lecture dessin de base
500 IF Ct="7. " THEN GOSUB 1040 ' affichage dessin base avec ech
510 IF Ct="&" THEN IF P>1 THEN P=P-1:CLS #2:G0SUB 1220
520 IF Ci="A" THEN END
530 GOTO 210
540 '------------------------rectangle plein
550 FOR Y1=YA TO Y STEP SGN(Y-YA)
560 PLOT X A,Y1,CE : DRAW X,Y1,CE
570 NEXT Y1
580 RETURN
590 '------------------------------------------Cercle plein
600 R=SQR((XA-X)A2+(YA-Y)A2):R2=RA2 ' A=fleche haut
610 FOR DX=-R TO R
620 DY = SQR(R2-(DXA2) ) ' A=fleche haut
630 PLOT XA + DX,YA + DY,CE : DRAW XA + DX,YA-DY,CE
640 NEXT DX
650 RETURN
660 '----------------------------------------- triangle plein
670 ' valider 2 points avec 'P' puis un troisième avec 'Y'
680 D = SQR ( (YB-YA)A2+(XB-XA)A2)
690 IF D = O THEN RETURN
700 CX=(XA-XB)/D:CY=(YA-YB)/D
710 FOR DD = O TO D STEP 0.5
720 X3=XB+DD*CX:Y3=YB+DD*CY
730 PLOT X,Y,CE:DRAW X3,Y3,CE
740 NEXT DD
750 RETURN
760 '---------------------------- MAJ TABLES
770 P = P+1: X(P)= X:Y(P)= Y:CM»(P)=C$:CE(P)=CE
780 XX (P)=X:YY(P)=Y:RETURN
790 '----------------------------------------- SAUVEGARDE COMMANDES
800 INPUT #1,"Nom";NF»
810 OPENOUT NF$
820 PRINT #9,P
830 FOR J=1 TO P
840 PRINT #9,XX(J)
850 PRINT #9,YY(J)
860 PRINT #9,CM$(J)
870 PRINT #9,CE(J)
880 NEXT J
890 CLOSEOUT
900 RETURN
910 ■----------------------------------------AFFICHAGE DESSIN BASE
920 INPUT #l,"Nom ";NF$
930 CLS #2
940 GOSUB 1140
950 SX=X:SY=Y
960 X=l:Y=399:ECH=1
970 FOR J = 1 TO P
980 X (J)=XX(J)»ECH + X: Y(J) = (YY(J)-400)»ECH +Y
990 NEXT J
1000 GOSUB 1220
1010 X=SX:Y=SY
1020 RETURN
1030 ’---------------------------------- DESSIN AVEC ECHELLE
1040 INPUT #1 , "Echel1e";ECH
1050 IF ECH<=0 THEN ECH=1
1060 FOR J=1 TO P
1070 X ( J)=XX(J)*ECH + X:Y(J) = (YY(J)-400)»ECH + Y
1080 NEXT J
1090 SX=X:SY=Y
1100 GOSUB 1220
1110 X = SX:Y = SY: FOR J = 1 TO P : X ( J)=XX(J): Y(J)=YY(J): NE X T J
1120 RETURN
1130 '---------------------------------LECTURE DESSIN BASE
1140 OPENIN NF$
1150 INPUT #9,P
1160 FOR J = 1 TO P
1170 INPUT #9,XX(J),YY(J),CM$(J),CE ( J)
1180 NEXT J
1190 CLOSEIN
1200 RETURN
1210 '-------------------------------------- REDESSINE
1220 FOR J=1 TO P
1230 X = X ( J):Y = Y ( J>: CE = CE(J)
1240 IF CM$(J)="!" THEN TT = ABS ( TT-1): GOTO 1340
1250 IF TT=1 THEN TAG:PLOT X,Y,CE:PRINT CM$(J); :X = X +16:TAGOFF: G OTO 1340
1260 IF CM$(J)="P" THEN PLOT X , Y , CE : X B= X A : YB=Y A : X A = X : Y A«= Y
1270 IF CM$(J)="D" THEN PLOT X A,YA,CE:DRAW X,Y,CE :XB = XA:YB = YA: XA = X:YA = Y
1280 IF CMt(J)="C" THEN GOSUB 600
1290 IF CM$(J)="R" THEN GOSUB 550:XB=XA:YB=YA:XA=X:YA=Y
1300 IF CM$(J)="Y" THEN GOSUB 680:XB=XA:YB=YA:XA=X:YA=Y
1310 IF CM$(J)="F" THEN MOVE X, Y,CE:FILL CE ' CPC664
1320 IF CM»(J)="!" THEN TT=ABS(TT-1)
1330 IF CM$(J)="G" THEN AC=CE:CE=O:GOSUB 550:CE=AC
1340 NEXT J
1350 RETURN
1360 '
1370 XB=XA:YB=YA:YA=Y:XA=X
1380 RETURN
1390 '-------------------------MENU (OPTIONNEL)
1400 MODE 2:PAPER 0:PEN 1 : INK 0,26:INK 1,0
1410 PRINT "Permet de dessiner des -figures de base dans une fene tre"
1420 PRINT "en haut a gauche"
1430 PRINT "Fleches,P=premier point,D=droite avec point preceden t,C = cercle“
1440 PRINT "R = rectangle,Y = triang1e,F = peindre (CPC664) "
1450 PRINT
1460 PRINT "EX: -DROITE: Fléchés puis 'P'.Ensuite Fléchés puis ' D'.
1470 PRINT " -CERCLE: 'P' pour centre.Ensuite Fléchés et 'C' h
1480 PRINT "Couleurs: 0,1,2,3 (avec 0,1e curseur apparait seulem ent sur surFace coloriée)"
1490 PRINT "G gomme un rectangle"
1500 PRINT “+ et - changent la vitesse du curseur"
1510 PRINT "Je annule la derniere commande"
1520 PRINT "! Fait passer du mode graphique au texte et inverse ment "
1530 PRINT:PRINT "SAUVEGARDE DESSIN DE BASE: '#' "
1540 PRINT "LECTURE DESSIN DE BASE: '$'
1550 PRINT "'7.' permet de les aFFicher avec une echelle"
1560 PRINT "dans diFFerents endroits de l'écran grace au curseur II
1570 PRINT "L'ensemble du dessin compose peut etre sauvegarde pa r 'S' et lu par ' L ' "
1580 PRINT:PRINT "A: arret programme"
1590 PRINT:INPUT "APPUYER SUR ENTER"jX$
1600 RETURN
1610 '----------
1620 ' Pour disque ajouter: 25 ON ERROR GOTO 1800
1630 ' 1800 IF DERR=146 THEN RESUME 920"##########,
    "pages 34, 35, 36"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_047,
    r##########"10 ' APMOT APPRENTISSAGE DE MOTS 20
30 1MODE lîINK 0,0: INK 1,26
40 NM0T =5 ' nombre de mots 50
60 ------- BOITE
70 DATA 16,354, 70,354, 70,310
80 :DATA 16,310, 16,354, 36,374
90 DATA 86,374, 70,354, 70,310
100 DATA 88,332, 88,374, 76,394
110 DATA 26,394, 36,374, 999,999
120 DATA BOITE 130 -___
140 MAISON
150 DATA 12,364, 90,364, 76,384
160 DATA 28,384, 12,364, 20,364
170 DATA 20,318, 78,318, 78,364
180 DATA 0,0
190 DATA 66,342, 48,342, 48,318
200 DATA 66,318, 66,342
210 DATA 0,0
220 DATA 28,352, 40,352, 40,338
230 DATA 28,338, 28,352, 28,346
240 DATA 40,346, 40,338, 36,338
250 DATA 36,352
260 DATA 999,999
270 DATA MAISON
280 '____ — -- BOUGIE
290 DATA 42,358, 42,286, 66,286
300 DATA 66,358, 42,358, 52,358
310 DATA 48,366, 46,374, 46,384
320 DATA 52,390, 56,380, 56,374
330 DATA 54,366, 52,360
340 DATA 999,999
350 DATA BOUGIE
360 BATEAU
370 DATA 38,382, 16,342, 38,342
380 DATA 38,382
390 DATA 0,0
400 DATA 44,382, 44,342, 70,342
410 DATA 44,382
420 DATA 0,0
430 DATA 6,338, 98,338, 86,324
440 DATA 16,324, 16,324, 6,338
450 DATA 999,999
460 DATA BATEAU 470 -____— ETOILE
480 DATA 12,366, 38,366, 48,392
490 DATA 64,366, 92,366, 74,344
500 DATA 88,320, 52,330, 28,310
510 DATA 32,342, 12,364
520 DATA 999,999
530 DATA ETOILE
540 ========= LECTURE DESSIN AU HASARD
550 P=I NT(RND(1)*NM0T): IF P = AP THEN 550
560 AP=P:RESTORE
570 IF P = 0 THEN 630
580 FOR J=1 TO P
590 READ X,Y:IF X = 999 THEN 600 ELSE 590
600 READ MOT*
610 NEXT J
620 -----------------AFFICHAGE
630 MODE 1
640 READ XA,YA
650 PLOT XA,YA,1 660
670 READ X,Y : IF X = 999 THEN 730
680 IF X=0 THEN READ X,Y:PLOT X,Y,1:GOTO 700
690 DRAW X , Y , 1
700 XA=X:YA=Y
710 GOTO 670
720 '--------- REPONSE
730 READ MOT*
740 LOCATE 1,2O:INPUT "Réponse (ou FIN) ";R*
750 R* = UPPER*(R*): IF R*="FIN" THEN END
760 IF R* = UPPER*(MOT* ) THEN PRINT "OK":GOTO 830 770
780 GOSUB 860
790 ■
800 IF R = LEN(MOT*)- 1 THEN PRINT "LA BONNE ORTHOGRAPHE ESTMOT*: GOTO 830
810 PRINT "LA BONNE REPONSE EST:";MOT* 820
830 FOR TP=1 TO 1000:NEXT TP
840 GOTO 550
850 '--------------------- VOISINAGE DE LA REPONSE
860 R=0 ' nombre de bonnes lettres
870 FOR K=1 TO LEN (R*) „
880 C$=MID$(R$,K,1)
890 FOR J=1 TO LEN(MOTl)
900 IF C$ = MID$(MOT$, J , 1 ) THEN R = R+1:GOTO 920
910 NEXT J
920 NEXT K
930 RETURN COMPOSITION DE PAYSAGE AVEC ANIMATION A l’aide de trois figures de base (arbre, maison, locomotive), vous composez un pay­ sage. Le choix des figures se fait à l’aide d’un curseur que vous déplacez avec les quatre flèches. En frappant “P”, vous “prenez” une des trois figures de base (BEEP signale que la figure a été choisie). “D” permet de “déposer” la figure choisie. La couleur se choisit en frappant 1,2,3. Pour animer une figure, placer le curseur devant elle puis frapper “+” ou (une ou plusieurs fois). Une figure animée peut également être stoppée. S$() contient les dessins. X() et Y() contiennent les coordonnées des dessins. V() contient les vitesses. Exemple : Placez le curseur sur le centre de la locomotive et appuyez sur “P” puis déplacez le curseur et appuyez sur “D”. Pour animer la figure, placez le curseur devant la locomo­ tive et appuyez sur “+” ou Pour “accrocher” une figure derrière une autre, placez le curseur derrière la première figure et appuyez sur “A”. Fie cl» es j? u is Pzprendre I> : cl epo s er A : a c oro + : a van c er — : r e o u 1 er F : F in Couleur-s : 1,2,3"##########,
    "pages 37, 38, 39"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_048,
    r##########"10 ■-----------------COMPOSITION DE PAYSAGE AVEC ANIMATION
20 MODE 1
30 CE =1 :CF = 0 ' ecriture/fond
40 PAPER O:PEN 1
50 INK 0,0:INK 1,26:INK 2,2:INK 3,6
60 DIM X(32) , Y ( 32) , V(32) ' coordonnées/vitesse
70 DIM S$(32)
80 NS = 3 ' nombre de figures de base
90 NN = NS ' nombre total de figures
100 '
110 LOCATE 1,23:PRINT "Fléchés puis P:prendre D:deposer Azaccroc her"
120 LOCATE 1 ,24:PRINT " + :avancer -:reculer F:fin"
130 LOCATE 1,25:PRINT "Couleurs: 1,2,3"
140 EF$ = CHR$(32)+CHR$(10)+CHR$(8)+CHRI(32)+CHR$( 11) ' effacement
150 '--------------------------- locomotive
160 C = 145 ' 1er caractère a modifier
170 SYMBOL AFTER C
180 SYMBOL C,0,0,6,0,16,0,48,48
190 SYMBOL C+l ,0,0,0,0,254,198,254,254
200 SYMBOL C + 2 , 1 27 , 1 27 , 1 27 , 1 27,255,255,24,24
210 SYMBOL C+3,254,254,254,254,255,255,24,24
220 S$ ( 1 )=CHR$(C)+CHR$(C+l)+CHR$(10)+CHR$(8)+CHR$(8)+CHR$(C + 2)+C HR$(C + 3)+ CHR*(11)
230 maison
240 SYMBOL C+ 4, 12, 12,63, 127,255,64,95,85
250 SYMBOL c+5,0,0,248,252,255,2,2,2
260 SYMBOL c+6,95,85,85,85,95,64,64,127
270 SYMBOL c+ 7, 122,7 4,74, 106,7 4,74,74,254
280 S$(2)= CHR$(C + 4)+CHR$(C + 5)+ CHR$(10)+ CHR$(8)+ CHR$(8)+CHR$(C+6) + CHR$ (C + 7)+CHR$(11)
290 '---------------------arbre
300 SYMBOL C + 8 , 1 , 3,7,7 , 15 , 15 ,15,31
310 SYMBOL C+9,0,128,192,192,224,224,240,240
320 SYMBOL C+10,63,63,63,3,3 ,3,3,15
330 SYMBOL C+1 1,248,248,248 , 0,0,0,0,192
340 S$(3)=CHR$ (C + 8)+ CHR$(C + 9)+ CHR$(10)+ CHR$(8)+ CHR$(8)+ CHR$(C + 10 )+ CHR$(C+l 1)+ CHR$(11)
350 '---------------------AFFICHAGE FIGURES DE BASE
360 FOR S=1 TO NS
370 X(S)=1 : Y(S)=S*3
380 LOCATE X(S) ,Y ( S): PR I NT S$(S)
390 NEXT S
400 ‘
410 X=64zY=384
420 '--------------- Curseur clignotant
430 T=TEST(X,Y):TB=TEST(X+2,Y)
440 '
450 C$=INKEY$zIF LEN(C$)<>0 THEN 510
460 PLOT X + 2,Y,CE
470 PLOT X ,Y,CE:FOR TP = 1 TO 10:NEXT TP:PLOT X, Y,CF
480 GOSUB 870
490 GOTO 450 500
510 PLOT X ,Y,T:PLOT X+2,Y,TB
520 CI=UPPERI(Cl)
530 C=ASC(CI)
540 IF C=242 THEN IF X>15 THEN X=X-16:GOTO 430
550 IF C=243 THEN IF XC580 THEN X=X+16:GOTO 43
560 IF C = 240 THEN IF Y<384 THEN Y=Y+16:G0T0 430
570 IF C=241 THEN IF Y>15 THEN Y=Y-16:G0T0 430
580 IF CI="F" THEN END
590 IF CI>"0" AND Cl<"4" THEN CE = VAL(Cl): PEN CE
600 IF CI="P" THEN GOSUB 670
610 IF CI="D" THEN GOSUB 780
620 IF CI="A" THEN GOSUB 1080
630 IF Cl="+" THEN V=1:GOSUB 980
640 IF Cl="-" THEN V=2:G0SUB 980
650 GOTO 430
660 -----------------------0N PREND
670 88 = 0
680 XC=I NT(X/16)+1 :YC=I NT((400-Y)/16)+1
690 FOR 8=1 TO NS
700 IF XC>=X(8) AND XC<=X(S)+2 AND YC>=Y(S) AND YC<=Y(S)+2 THEN 730
710 NEXT 8
720 RETURN
730 88 = 8
740 PRINT CHRI(7);
750 LOCATE 1,21:PRINT 81(88)
760 RETURN
770 -------------------ON P0SE
780 IF 88=0 THEN RETURN
790 IF NN>30 THEN RETURN
800 XC=I NT(X/16)+1 :YC = INT((400-Y)/ 16 ) +1
810 IF XC<4 THEN RETURN
820 NN = NN+1 : SI(NN)=81(88)
830 LOCATE XC,YC:PRINT S$(NN)
840 Y(NN)=YC:X(NN)=XC
850 RETURN
860 '-----------------AVANCE
870 IF NN=NS THEN RETURN
880 FOR S=NS+1 TO NN
890 IF V(S)=O THEN 950
900 X ( 8)= X(8)-V(8)
910 IF X (8)<6 THEN LOCATE X ( 8) , Y (8): PR I NT EFI; EFI ; EFI;EFI; EFI: X(8)=34
920 IF X(8)>34 THEN LOCATE X(8),Y(8): PR I NT EFI;EFI; EFI ; EFI:X(8)=6
930 IF V(S)>0 THEN LOCATE X(8),Y(8): PR I NT SI(8); EFI ;EFI
940 IF V(SX0 THEN LOCATE X(8)-1,Y(8) : PR I NT EFI;EFI;SI(8);EFI
950 NEXT 8
960 RETURN
970 '--------------REGLAGE VITESSE
980 XC=I NT(X/16)+1 :YC=I NT(( 400-Y)/16)+1
990 FOR S=NS+1 TO NN
1000 IF YC>=Y(S) AND YCOY (8) +1 THEN 1040
1010 NEXT S
1020 RETURN
1030 '
1040 IF V=1 THEN IF XC>3 THEN V(S)=V(S)+0.5:V = 0
1050 IF V = 2 THEN IF XC>3 THEN V (S)=V(S)-0.5:V = 0
1060 RETURN
1070 '---------------------------ACCROCHAGE
1080 XC=INT(X/16)+1:YC=INT((400-Y)/16)+1
1090 '
1100 FOR S=NS+1 TO NN
1110 IF YCOY(S) THEN 1130
1120 IF XC-2=X(S) THEN S$ (S)=S$(S)+S$(SS): LOCATE XC,YC:PRINT S$(SS): RETURN
1130 NEXT S
1140 RETURN MINI INTERPRÉTEUR LOGO La plupart des langages proposent des ordres graphiques où sont spécifiées les coor­ données X et Y des droites à tracer. Le langage LOGO dispose d’ordres graphiques originaux : AVANCE distance Trace une droite d’une longueur égale à la distance spécifiée. ROTATION angle Change la direction du tracé en lui ajoutant l’angle spécifié. Un curseur indique à chaque instant la direction courante. En frappant “ROTATION 90” vous voyez le curseur tourner de 90 degrés. EX : AUANCE 40 ROTAT 90 EINI pour stopper CMDE ? avance 70 CMDE ? rotat 9O CMDE ? avance 70 CMDE ? x-otat 90 CMDE 7 avance 70 CMDE 7 x'otat 90 CMDE o Le tracé de la droite en fonction de l’angle courant se fait ainsi : DX=distanceXCOS(angle) DY=distanceXSI N(angle)"##########,
    "pages 40, 41, 42"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_049,
    r##########"10 SIMULATEUR LOGO 20
30 MODE 1
40 INK 0,26:INK 1 ,0: INK 2,2:INK 3,6
50 DIM GDI(100): X0 = 450 :YC=150: 0X = 1 :CY = 1 : CY = O : DI ST = 20
60 DIM (5,20) ' table fonctions
70 XX=XC:YY=YC ' mémoire centre
80 CE=1 ' écriture _
90 CF = 0 ' -fond
100 '
110 WINDOW #0,1,21,1,23
120 WINDOW #1,35,39,1,3
130 PRINT "EX: AVANCE 40
140 PRINT " ROTAT 90"
150 PRINT
160 PRINT "FINI pour stopperPR I NT
170 '-------
180 L =1 ' pointeur ligne de commande
190 '
200 DS=1:GOSUB 1080 ' appel curseur apparent
210 INPUT "CMDE ";XS
220 XS=UPPERS(XS)
230 DS=O:GOSUB 1080 ' e-f-facement curseur
240 IF XS="" THEN 200
250 '
260 IF LEFTS(XS,4)="DEF I" THEN GOSUB 930:G0T0 200
270 FOR K=1 TO 5
280 IF XS=FS(K,O) THEN GOSUB 1020:G0T0 380
290 NEXT K
300 '
310 IF LEFTS( XS , 4)="REPE" THEN B = B+1
320 '
330 IF LEFTS(X$,4)="ENCO" THEN B = B-1
340 '
350 CDS(L)=XS ' stockage commande dans CDS ( )
360 L = L + 1
370 '
380 IF B>0 THEN 200
390 '
400 NC=L-1 ' nombre de commandes
410 PL=1
420 '
430 LGS=CDS(PL)
440 IF PL?>NC THEN 180
450 CS=LEFTS(LGS,4)
460 IF CS="REPE" THEN GOSUB 670:G0T0 430
470 IF CS="ENCO" THEN GOSUB 710:G0T0 430
480 IF CS="AVAN" THEN GOSUB 810
490 IF CS="ROTA" THEN GOSUB 740
500 IF CS="DIST" THEN GOSUB 630: IF VOO THEN DIST=V
510 IF CS="DIS+" THEN GOSUB 630: IF VOO THEN DIST = DIST + V
520 IF CS="LEVE" THEN LV = 1:LOCATE #1,1,1:PRINT #1 ,C$
530 IF CS="BAIS" THEN LV=O:LOCATE #1,1,1:PRINT #1,CS
540 IF CS = "COUL" THEN GOSUB 630: IF V>0 THEN CE = V
550 IF CS="EFFA" THEN CLG
560 IF CS="COLO" THEN GOSUB 630: IF VOO THEN XC = V
570 IF CS="LIGN" THEN GOSUB 630: IF VOO THEN YC = V
580 IF CS="CENT" THEN XC=XX:YC=YY
590 IF CS="FINI" THEN END
600 PL=PL+1:IF PL>NC THEN 180
610 GOTO 430
620 '-------------recherche espace
630 P= INSTR(LGt,CHRt(32))
640 IF P = 0 THEN V = 0:RETURN
650 V = VAL(RIGHT!(LGt,LEN(LGt)-P)): RETURN
660 '---------------répété
670 GOSUB 630
680 PR=PR+1:RP(PR)=PL+1:NB(PR)=V
690 PL = PL+1 : RETURN
700 '--------------------------------------ENCORE
710 NB(PR)=NB(PR)- 1 : IF NB(PR)>0 THEN PL = RP(PR):RETURN
720 PR = PR-1:PL = PL + 1: RETURN
730 '-------------------------------------- ROTATION ANGLE
740 GOSUB 630
750 ANG=ANG+V
760 IF ANG>=360 THEN ANG=ANG-360
770 AR=ANG/360*PI*2
780 CX=COS(AR):CY=SIN(AR)
790 RETURN
800 ■---------------------------------------- TRACE DROITE
810 GOSUB 630
820 IF P=0 THEN 840
830 DIST=V
840 DX=DIST*CX:DY=DIST*CY
850 IF XC+DXCl OR XC+DX>640 THEN RETURN
860 IF YC+DY<1 OR YC+DY>399 THEN RETURN
870 IF LV=1 THEN 900
880 '
890 PLOT XC,YC,CE : DRAWR DX,DY,CE
900 X C = X C + D X : Y C = Y C + D Y
910 RETURN
920 '----------------------------------STOCKAGE FONCTION DANS Ft ( , )
930 NF=NF+1
940 Ft(NF,0)=RIGHTt(Xt,LEN(X !)-5)
950 FOR P=1 TO 20
960 INPUT "CD FCT ";Ft
970 Ft(NF,P)=UPPERt(Ft)
980 IF Ft(NF,P)="F I NF" THEN RETURN
990 NEXT P
1000 STOP
1010 '------------------------------ INSERTION FONCTION
1020 P=1
1030 '
1040 IF Ft (K,P)="FINF" THEN RETURN
1050 CDt(L)=Ft(K,P)
1060 P = P+1:L = L + 1: GOTO 1040
1070 '---------------------------------- CURSEUR
1080 DX=CX*6:DY=CY*6
1090 IF DS=1 THEN CL = CE:X = XC + DX:Y = YC + DY : IF TEST(X,Y)<>0 THEN CL = O
1100 IF DS = O THEN CL = CF:X=XC+DX:Y = YC + DY: IF TEST(X,Y)=0 THEN CL = CE
1110 PLOT XC , YC,CL : PLOT XC + DX , YC + DY,CL :DRAWR DX,DY,CL
1120 RETURN
1130 '---------------------
1140 ex: AVANCE 50
1150 ROTAT 90
1160 AVANCE 60"##########,
    "pages 46, 47, 48"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_050,
    r##########"1180 DEFI CARRE
1190 REPETE 4
1200 AVANCE 60
1210 ' ROTAT 90
1220 ' ENCORE
1230 ' FINF 1240
1250 CARRE MUSIQUE ______________________________________________ Sur AMSTRAD la définition des notes se fait en donnant la période associée, ce qui est moins pratique que de définir la note et le niveau d’octave. Ci-dessous, nous indiquons en DATA la correspondance note-période pour le niveau d’octave -1. Pour les autres niveaux d’octave, nous calculons la période nécessaire à l’instruction “SOUND canal,période”."##########,
    "page 49"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_051,
    r##########"10 •------------------------------------- MUSIQUE
20 DIM NT$112),PER(12)
30 VALEURS PERIODES POUR OCTAVE -1
40 DATA 956,D0
50 DATA 902,D0#
60 DATA 851,RE
70 DATA 804,Mlb
80 DATA 758,MI
90 DATA 716,FA
100 DATA 676,FA#
110 DATA 638,SOL
120 DATA 602,SOL#
130 DATA 568,LA
140 DATA 536,SIb
150 DATA 506,SI
160 '
170 FOR J = 1 TO 12:READ PER(J) , NT$ ( J): NEXT J
180 '--------------------------------- MORCEAU A JOUER
190 DATA LA,2,20,SOL,2,20,LA,2,50
200 DATA SOL,2,20,FA,2,20,MI,2,20
210 DATA RE,2,20,DO#,2,50,RE,2,90
220 DATA LA,1,20,SOL,1,20,LA,1,50
230 DATA MI, 1,35,FA,1,35,DO#,1 ,35
240 DATA RE,1,80,LA,0,20,SOL,0,20
250 DATA LA,0,50,SOL,0,20,FA,0,20
260 DATA MI ,0,20,RE,0,20,DO#,0,20
270 DATA RE,0,90,RE,2,20,DO,2,20
280 DATA RE,2,20,SIb,1,20,RE,2,20
290 DATA LA, 1,20,RE,2,20,SOL,1,20
300 DATA RE,2,20,FA#,1,20,RE,2,20
310 DATA SOL,1,20,RE,2,20,LA,1,20
320 DATA RE,2,20,SIb,1,20,RE,2,20
330 DATA RE,0,20,RE,2,20,MI,0,20
340 DATA RE,2,20,FA#,0,20,RE,2,20
350 DATA FIN,0,0
360 '-------
370 READ NT$,0CT,DUR
380 IF NT$="FIN" THEN END
390 PRINT NT$,0CT,DUR
400 FOR N=1 TO 12
410 IF NT$=NT$(N) THEN SOUND 1 , PER(N)/(2A(OCT+1)),DUR
420 NEXT N
430 GOTO 370
440 '-------------------------------- Mleche haut Le programme ci-dessous décode un morceau défini sous forme de périodes pour donner la note et l’octave."##########,
    "pages 49, 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_052,
    r##########"10 •------------------------- DECODAGE PERIODE —> NOTE ET OCTAVE
20 DIM NT$ ( 12) ,PER <12>
30 '------- VALEURS POUR OCTAVE -1
40 DATA 956,DO
50 DATA 902,DO#
60 DATA 851,RE
70 DATA 804,Mlb
80 DATA 758,MI
90 DATA 716,FA
100 DATA 676,FA#
110 DATA 638,SOL
120 DATA 602,SOL#
130 DATA 568,LA
140 DATA 536,SIb
150 DATA 506,SI
160 '
170 FOR N=1 TO 12:READ PER ( N) ,NT $(N): NE X T N
180 ’-MORCEAU A DECODER(PER IODE + DUREE) —> NOTE + OCTAVE + DUREE
190 DATA 71,20,80,20,71,50,80,20,89,20,95,20,106,20,113,55,106,9 0,142,20,159,20,142,50,190,35,179,35,213,60,213,80,284,20,319,20 ,284,50,319,20,358,20,379,20,426,20,426,50,426,90
200 DATA 999,999
210 '
220 READ PER,DUR: IF PER=999 THEN END
230 SOUND 1,PER,DUR
240 FOR N=1 TO 12
250 FOR OCT=-1 TO 4
260 R = PER (N) / (2A (OCT+1 ) ) ' A = Heche haut
270 IF PER>R-1 AND PER<R+1 THEN PRINT PER,NT$(N),OCT,DUR
280 NEXT OCT
290 NEXT N
300 GOTO 220 RUN"##########,
    "page 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_053,
    r##########"71 LA J 20
80 SOL 2 20"##########,
    "page 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_054,
    r##########"71 LA 2 50
80 SOL 2 20
89 FA 2 20
95 MI 2 20"##########,
    "page 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_055,
    r##########"10 '---------------ENREGISTREMENT D’UN MORCEAU DANS UN FICHIER
20 MODE 2:PAPER 0:PEN 1 s I NK 0,26:INK 1,0
30 '------------------------------------ octave -1
40 DIM NTI(12),PER(12),CLI(12)
50 DATA D0,956,D,D0#,900,E,RE,851 ,R,Mlb,804,N
60 DATA MI,758,M,FA,716,F,FA#,676,G,SOL,638,S
70 DATA SOL#,602,D,LA,568,L,SIb,536,A,SI ,506,Z
80 '
90 FOR N=1 T0 12
100 READ NT!(N),PER(N),CL*(N)
110 NEXT N
120 CLS
130 '
140 FOR N=1 T0 12
150 LOCATE N*5,10:PRINT NT*(N)
160 LOCATE N*5,llîPRINT CL*(N)
170 NEXT N
180 LOCATE 1,20:PRINT "OCTAVE: 0,1,2 "
190 LOCATE 1 , 21 :PRINT "G: FIN"
200 0CT = 2 ■ OCTAVE
210 LOCATE 1,4: INPUT "NOM FICHIER ";NFI
220 OPENOUT NFI
230 '-----------------------------
240 CI=INKEYI:IF LEN(C*)=O THEN 240
250 CI=UPPERI(Cl)
260 IF CI>="0" AND Cl<"=2" THEN OCT=VAL(Cl):LOCATE 1,1:PRINT " OCTAVE :"; OCT
270 IF CI="Q" THEN CLOSEOUTzEND
280 FOR N=1 TO 12
290 IF CI=CLI(N) THEN 330
300 NEXT N
310 GOTO 240
320 ’
330 PRINT #9,NTI(N)
340 PRINT #9,OCT
350 PRINT #9,20
360 LOCATE 20,l:PRINT NTI(N); SPC(2)
370 SOUND 1 , PER(N)/(2A(0CT+1)),20 ’ Mleche haut
380 GOTO 240"##########,
    "page 51"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_056,
    r##########"10 JOUE UN MORCEAU ENREGISTRE DANS UN FICHIERIPAR 'EMUS')
20 MODE 2
30 DIM NTT( 12) , PER(12)
40 '--- VALEURS PERIODES POUR OCTAVE -1
50 DATA 956,DO
60 DATA 902,DO#
70 DATA 851,RE
80 DATA 804,MIb
90 DATA 758,MI
100 DATA 7 16,FA
110 DATA 676,FA#
120 DATA 638,SOL
130 DATA 602,SOL#
140 DATA 568,LA
150 DATA 536,Sib
160 DATA 506,SI
170 '
180 FOR N=1 TO 12:READ PER(N),NTT(N):NEXT N
190 '------------------------------------------------------
200 INPUT "NOM FICHIER ";NF$
210 OPENIN NF$
220 '
230 IF EOF THEN CLOSEIN:END
240 INPUT #9, NTT,OCT,DUR
250 PRINT NTT,OCT,DUR
260 FOR N=1 TO 12
270 IF NTT=NTT(N) THEN SOUND 1 , PER(N)/(2A(OCT+1)),DUR
280 NEXT N
290 GOTO 230"##########,
    "page 52"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_057,
    r##########"10 '----- ---------- morse MORSESO —
20 ' • —
30 MODE 2:PAPER O:PEN 1 A
40 DIM M0RSE$(26) B — •••
50 DATA — • — •
60 DATA
70 DATA — — . . y ■ . . • y • . y —. — —y —• —y • —• • y — —
80 DATA — • 1 — — — y ■ — y . y. _ . y ... y — y . • — ---- ••
90 DATA
100 '
110 FOR 1 = 1 TO 26:READ MORSES( I ):NEXT I
120 '
130 C$=INKEY$:IF LEN(C$)=0 THEN 130
140 C$=UPPER$(C$)
150 P=ASC(CS)-64:IF P<1 OR P>26 THEN 130
160 '
170 XS=MORSES(P)
180 PRINT XS;SPC(2);
190 FOR P=1 TO LEN(XS)
200 IF MI DS ( XS , P , 1)= ". " THEN SOUND l,350,8:F0R TP=1 T0 150:NEXT TP
210 IF MI DS(XS,P,1)=" THEN SOUND l,350,40:F0R TP=1 T0 400:NEXT TP
220 NEXT P
230 FOR TP=1 T0 300:NEXT TP
240 GOTO 130"##########,
    "page 53"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_058,
    r##########"10 ' FICHIER D'ADRESSES SIMPLE
20 '
30 MODE 2:INK 0,0:INK 1,26:PAPER O:PEN 1
40 DIM NOM*(300),RUE*(300) , V ILLE*(300),CPST*(300)
50 DIM CLE*(200),INDEX(200)
60 '
70 NFICH=O ' nombre de Fiches
80 INPUT "Nouveau fichier (0/N) " ; R*
90 IF R*<>"0" AND R*<>"o" THEN GOSUB 550
100 ====================== MENU
110 CLS:PRINT "Modes: " :: PR I NT
120 PRINT TAB(3) "C: creation"
130 PRINT TAB(3) "LF: liste du fichier"
140 PRINT TAB(3) "S: suppressi on"
150 PRINT TAB(3) "M: modi f i cat i on"
160 PRINT TAB(3) "LT: liste tr i ee"
170 PRINT TABOU "ETIQ:: etiquettes"
180 PRINT TAB(3) "FIN: fin de session(sauvegarde)"
190 '
200 PRINT:INPUT "Mode ";MS
210 MS=UPPERS(MS)
220 IF MS="C" THEN GOSUB 310
230 IF MS="FIN" THEN GOSUB 440:END
240 IF MS="LF" THEN GOSUB 670
250 IF M$="M" THEN GOSUB 760
260 IF MS="S" THEN GOSUB 980
270 IF MS="LT" THEN GOSUB 1170
280 IF MS="ETIQ" THEN GOSUB 1460 ' programme suivant
290 GOTO 110
300 ============================ CREATION
310 PRINT
320 INPUT "Norn (ENTER pour -fin) ";NOMS
330 IF LEN(NOM$)=O THEN RETURN ' -fin de mode?
340 NF ICH = NFICH+1
350 RANG=NFICH ' adresse de rangement
360 NOMS(RANG)= N0MS
370 ■---------------------entree des zones
380 PRINT
390 LINE INPUT "Rue? ",RUES: RUES(RANG)=RUES
400 LINE INPUT "Ville? ",VILLES: VILLES(RANG)=VILLES
410 LINE INPUT "Code postal? ", CPSTS:CPSTS(RANG)=CPSTS
420 GOTO 310
430 ======================== SAUVEGARDE TABLES
440 OPENOUT "ADR"
450 PRINT #9,NFICH
460 FOR F=1 TO NFICH
470 PRINT #9,N0MS(F)
480 PRINT #9,RUES(F)
490 PRINT #9,VILLES(F)
500 PRINT #9,CPSTS(F)
510 NEXT F
520 CLOSEOUT
530 RETURN
540 ================= LECTURE FICHIER DANS TABLES
550 OPENIN "ADR"
560 INPUT #9,NFICH
570 FOR F=1 TO NFICH
580 LINE INPUT #9,N0MS(F)
590 LINE INPUT #9,RUES(F)
600 LINE INPUT #9,VILLES(F)
610 LINE INPUT #9,CPSTS(F)
620 NEXT F
630 CLOSEIN
640 PR I NT : PR I NT NFICH;"Fiches"
650 FOR TP=1 TO 2000:NEXT TP
660 RETURN
670 '= = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = liste du -fichier
680 PRINT:PRINT "Liste du f1 chier": PR I NT
690 FOR F=1 TO NFICH
700 IF F MOD 20 = 0 THEN INPUT "APPUYER SUR ENTER"; XS
710 PRINT NOMS(F) TAB(13) RUES(F) TAB(40) VILLES(F)
720 NEXT F
730 PRINT:INPUT "Appuyer sur ENTER ";XS
740 RETURN"##########,
    "pages 56, 57"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_059,
    r##########"332 FOR RANG=1 TO NFICH
333 IF NOM$ = NOM$ (RANG) THEN PRINT "Existe deja":G0T0 310
334 NEXT RANG Pour procéder à des modifications de fiches, il faut ajouter le sous-programme suivant et prévoir son appel au menu. Nous affichons les anciennes valeurs des zones et si l’opérateur ne désire pas les modifier, il appuie sur ENTER. On s’aperçoit que les modes CREATION et MODIFI­ CATION pourraient être fusionnés. C’est ce que nous ferons dans les programmes suivants. Si un nom a été écrit en majuscules lors de la création il doit être écrit aussi en majus­ cules lors de la recherche. En changeant l’instruction 800, la recherche peut s’effectuer en majuscules ou minuscules :"##########,
    "page 58"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_060,
    r##########"750 ========================= MODIFICATION
760 PRINT:INPUT "Quel nom(ENTER pour fin) ";N0M$
770 IF LEN(N0M$)=0 THEN RETURN ' fin de mode?
780 '
790 FOR R A N G =1 T0 NFICH ' recherche du nom
800 IF N0M$=N0M$(RANG) THEN GOTO 840
810 NEXT RANG
820 PRINT:PRINT "N'existe pas":G0T0 760
830 '
840 PRINTîPRINT "ENTER pour zones inchangées" : P RIN T
850 PRINT RUES(RANG) TAB(20) ' affichage ancienne valeur
860 LINE INPUT "Nouvelle rue? ",RUE$
870 IF LEN(RUE$)<>0 THEN RUE$(RANG)=RUE$
880 '
890 PRINT VILLES (RANG) TAB(20) ' affichage ancienne valeur
900 LINE INPUT "Nouvelle ville? ",VILLE*
910 IF LEN(VILLE*)<>0 THEN VILLE*(RANG)=V I LLE*
920 '
930 PRINT CPST$(RANG) TAB(20)
940 LINE INPUT "Nouveau code postal? ",CPST*
950 IF LEN(CPST*)<>0 THEN CPST*(RANG)=CPST*
960 GOTO 760 SUPPRESSION D’UNE FICHE Pour supprimer une fiche nous décalons toutes les fiches en aval de la fiche supprimée. NOMS Rang ->
970 ================================== SUPPRESSION
980 PRINT:INPUT "NOM ";NOM*:IF LEN(NOM*)=O THEN RETURN
990 '
1000 FOR RANG=1 TO NFICH
1010 IF NOM*(RANG)=N0M* THEN 1050
1020 NEXT RANG
1030 PRINT:PRINT "N'EXISTE PAS": PR I NT : GOTO 980
1040 '
1050 PRINT
1060 INPUT "SUPPRESSION OK (0/N) ";R*:IF R*<>"0" THEN 980
1070 FOR J=RANG TO NFICH-1
1080 NOM*(J)=NOM*(J +1 )
1090 RUE*(J)=RUE*(J+1)
1100 VILLE*( J)=V I LLE*(J + 1)
1110 CPST*(J)=CPST*(J +1 )
1120 NEXT J
1130 NOM*(NFICH)RUE*(NFICH)VILLE*(NFICH)CPST*(NFICH)
1140 NF ICH = NFI CH-1
1150 GOTO 980
1160 = = = = = = = = = = = = = = = = = = = = = = = = SELECTION/TRI PAR VILLE
1170 INPUT "Quelle ville (ENTER pour toutes) ";V$
1180 N C = 0 nombre de clés
1190 FOR F = 1 T0 NFICH
1200 IF LEN(V$X>0 THEN IF V$<>VILLE$(F) THEN 1220
1210 NC=NC+1:CLE$(NC)=VILLE$(F): INDEX(NC)=F
1220 NEXT F
1230 '
1240 GOSUB 1330 ' appel tri
1250 '---------edition
1260 FOR F=1 T0 NC
1270 X=INDEX(F)
1280 PRINT VILLET(X) TABU5) NOM$(X)
1290 NEXT F
1300 PRINT: INPUT "APPUYER SUR ENTER";
1310 RETURN
1320 '-------------------------------------------------------- tri shell
1330 ECART=NC
1340 ECART=INT (ECART/2): IF ECARTCI THEN RETURN
1350 IV = 0
1360 FOR K=1 TO NC-ECART
1370 J=K+ECART
1380 IF CLE$(J)>=CLE$(K) THEN 1410
1390 X$ = CLE$ ( K); CLE!(K)=CLE$(J):CLE$(J)=X$ : IV=1
1400 X=INDEX(K): INDEX(K) = INDEX(J): INDEX(J)=X
1410 NEXT K
1420 IF IV=1 THEN 1350
1430 GOTO 1340"##########,
    "pages 58, 59, 60"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_061,
    r##########"1440 ============================== ETIQUETTES
1445 ' A ajouter au programme 'fichier d'adresses'"##########,
    "page 61"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_062,
    r##########"1460 INPUT "Quelle ville (ENTER pour toutes) “ ; V$
1470 INPUT "Ecran ou imprimante (E/I) “;R$
1480 C=0:IF R$="I" THEN C=8 ' canal
1490 NC=O ' nombre de clés
1500 FOR F=1 T0 NFICH
1510 IF LEN(V$)<>0 THEN IF V$<>V ILLEI(F) THEN 1530
1520 NC = NC+1 :CLE$(NC)=VILLE$(F): INDEX(NC)=F
1530 NEXT F
1540 '
1550 GOSUB 1600 ' appel tri
1560 '
1570 GOSUB 1720
1580 RETURN
1590 '-------------------------------------------------------- tri shell
1600 ECART=NC
1610 ECART=INT(ECART/2) : IF ECART< 1 THEN RETURN
1620 IV=0
1630 FOR K=1 TO NC-ECART
1640 J’K+ECART
1650 IF CLES(J)>=CLE$(K) THEN 1680
1660 X$=CLE$(K)!CLE$(K)=CLE$(J):CLE$(J)=X$:IV=1
1670 X = INDEX(K): INDEX(K) = INDEX(J): INDEX(J)=X
1680 NEXT K
1690 IF IV=1 THEN 1620
1700 GOTO 1610
1710 '--------------------------------------------------------------
1720 NE=2 ' nombre etiquettes par ligne
1730 IHRIZ=25 ' intervalle horizontal
1740 IVERT=6 ' intervalle vertical
1750 NLP=71 ' nombre de lignes par page
1760 MARGE=2
1770 '
1780 NRANG=I NT(NLP)/I VERT
1790 SP = NLP-NRANG*I VERT
1800 '
1810 TRANG=O:PRINT
1820 11=1
1830 '
1840 IF M>NC THEN 1910
1850 GOSUB 1940
1860 M=M+NE
1870 TRANG=TRANG+1
1880 IF TRANG=NRANG THEN FOR K=1 TO SP:PRINT #C:NEXT K:TRANG=O
1890 GOTO 1840
1900 '
1910 PRINT: INPUT "APPUYER SUR ENTER";XS
1920 RETURN
1930 ' ---------
1940 FOR K = O TO NE-1
1950 PRINT #C, TAB(MARGE + K* I HR I Z) NOMS(INDEX (M+K));
1960 NEXT K
1970 PRINT #C:PRINT #0
1980 FOR K=0 TO NE-1
1990 PRINT #C,TAB(MARGE+ 3 + K»I HR IZ) RUES(INDEX(M + K));
2000 NEXT K
2010 PRINT #C
2020 FOR K = 0 TO NE-1
2030 PRINT #C,TAB(MARGE + 3 + K*I HR I Z) CPSTS( INDEX(M + K)) SPC( 1 ) VI LLESdNDEX (M + K) ) ;
2040 NEXT K
2050 PRINT #C
2060 FOR K=1 TO I VERT-4 : PR I NT #C:NEXT K
2070 RETURN"##########,
    "pages 61, 62"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_063,
    r##########"10 'FICHIER STOCK (PROGRAMME ADAPTABLE A D'AUTRES FICHIERS)
20 '
30 ' FICHK,): table a 2 dimensions contenant les -fiches
40 ' NFICH: nombre de fiches
50 ' NRUB$(): noms des rubriques
60 '
70 MODE 2:PAPER O:PEN 1 : INK O,1:INK 1,24 g0 ■----------------------------------A ADAPTER
90 NF$="STOCK" ' nom du fichier
100 NRUB=4 ' nombre de rubriques
110 NRUBi(1)="CODE" ' nom rubrique 1
120 NRUB$(2) = "LI BELLE " ' nom rubrique 2
130 NRUBi ( 3)="PR I X"
140 NRUBS(4)="STOCK"
150 '------------------------------------------------------------------
160 DIM FI CHI(200,NRUB) ' 200 fiches maxi
170 DIM CLEJ (200),INDEX(200)
180 NFICH = O ' nombre de fiches 190
200 INPUT "Nouveau fichier (0/N) ";Ri
210 IF Ri="O" OR R$="o" THEN 230
220 GOSUB 770
230 CLS:PRINT "Modes:": PRINT
240 PRINT TAB(3) "C: creation/modification"
250 PRINT TAB(3) "LF: liste du fichier"
260 PRINT TAB(3) "S: suppression"
270 PRINT TAB(3) "LTC: liste triee par code"
280 PRINT TAB(3) "FIN: fin de session ( sauvegarde)
290 PRINT:INPUT "Mode " ; Mi
300 M$=UPPER$(Mi)
310 IF Mi="C" THEN GOSUB 380
320 IF Mi="FIN" THEN GOSUB 670:END
330 IF M$="LFH THEN GOSUB 900
340 IF Mi="S" THEN GOSUB 1080
350 IF Mi="LTC" THEN GOSUB 1260
360 GOTO 230
370 ==== = = = = = = = = = = = = = = = = = = = = ==== = = = = CREAT ION/MODIFI CAT I ON
380 PRINT
390 PRINT NRUBi( 1 );
400 LINE INPUT " (ENTER pour fin ) ? ",CLEi
410 IF LEN(CLEi)=O THEN RETURN ' fin de mode?
420 '
430 LG=LEN(CLEi)
440 IF NFICH=O THEN 490
450 FOR RANG=1 TO NFICH
460 IF CLEi=LEFTi(FICHi(RANG,1),LG) THEN 550 ' nom existe?
470 NEXT RANG
480 '---------------------------nouvelle c 1 e
490 PRINT:INPUT "Nouvelle cle OK (0/N) ";R$
500 IF Ri<>"0" AND RiO"on THEN 380
510 NF ICH = NFICH+1
520 RANG=NFICH
530 FICHi(RANG,1)=CLEi
540 '---------------------------------------------- entr ee/modi f zones
550 PRINT
560 PRINT "R:zone arriéré"
570 PRINT "Appuyer sur ENTER pour zone inchangée en mode MODIF":P RINT
580 FOR R = 2 TO NRUB
590 PRINT FICH$(RANG , R) TAB(25) ' ancienne valeur
600 PRINT NRUBt(R) TAB (32) ' nom de zone
610 LINE INPUT "?",X$
620 IF X$="R" THEN IF R>1 THEN R=R-1:BOTO 590
630 IF LEN(X$)<>0 THEN FICH$(RANG , R)= X$
640 NEXT R
650 GOTO 380
660 ' = = = = = = = = = = = = = = = = = ====== = = = = = = = * = x = = = = = = = = = = = sauvegarde
670 OPENOUT NF$
680 PRINT #9,NFICH
690 FOR F=1 TO NFICH
700 FOR R=1 TO NRUB
710 PRINT #9,F ICH$(F,R)
720 NEXT R
730 NEXT F
740 CLOSEOUT
750 RETURN
760 '= = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = lecture -fichier
770 OPENIN NFS
780 '
790 INPUT #9,NFICH
800 FOR F=1 TO NFICH
810 FOR R=1 TO NRUB
820 LINE INPUT #9,FICH$(F,R)
830 NEXT R
840 NEXT F
850 CLOSEIN
860 PRINT:PRINT nfich ; " FICHES "
870 FOR TP=1 TO 2000:NEXT TP
880 RETURN
890 '====================*======= liste du fichier
900 CLS
910 INPUT "Ecran ou imprimante (E/I) " ;R$
920 CN=O:IF R$="I" THEN CN = 8
930 PRINT #CN,"Liste du fichier " : PRINT #CN
940 '
950 PRINT #CN,NRUBI(1) TAB(IO) NRUB$(2) TAB (35) NRUBK3) TAB(45)
960 PRINT #CN,NRUB$(4) TAB(52) " VALEURPR I NT #CN
970 FOR F=1 TO NFICH
980 IF CN=O THEN IF F MOD 20=0 THFN INPUT "Appuyer sur ENTER";XI
990 PRINT #CN,FICH$ ( F , 1 ) TAB(IO) ' zone 1 (code)
1000 PRINT #CN,FICH$(F,2) TAB(35) ' zone 2 (libelle)
1010 PRINT #CN,FICH$(F,3) TAB(45) ' zone 3 (prix)
1020 PRINT #CN,FICH$(F,4) TAB(52)
1030 PRINT #CN,VAL(FICH$(F,3))♦VAL(FICH$(F,4))
1040 NEXT F
1050 PRINT:IF CN=O THEN INPUT "Appuyer sur <ENTER> ";x$
1060 RETURN
1070 '================================== SUPPRESSION FICHE
1080 PRINT :PRINT NRUB$(1); : INPUT CLE*
1090 IF LEN(CLE$)=O THEN RETURN
1100 FOR RANG=1 TO NFICH
1110 IF CLE* = FI CH*(RANG , 1 ) THEN 1150
1120 NEXT RANG
1130 PRINT:PRINT "N'existe pas":G0T0 1080
1140 '
1150 PRINT: INPUT "ANNULE OK(O/N) ";RS: IF R*O"0" THEN 1080
1160 FOR J=RANG TO NFICH-1
1170 FOR R=1 TO NRUB
1180 FI CH*(J , R)=F I CH*(J + 1,R)
1190 NEXT R
1200 NEXT J
1210 FOR R=1 TO NRUB: FI CH*(NF I CH,R)="": NEXT R
1220 NF ICH = NF I CH-1
1230 GOTO 1080
1240 '-----Attention1 II n'y a pas d'espace entre "" Une liste triée s’obtient suivant le principe présenté pour le programme “fichier d’adresses”.
1250 ======================== SELECTION/TRI PAR CODE
1260 PTRI=1 ' position de tri (CODE)
1270 INPUT "Quelle cle (ENTER pour toutes ) ";CLE*
1280 NC=O ' ne: nombre de clés
1290 LG=LEN(CLE*)
1300 FOR F=1 TO NFICH
1310 IF LEN(CLE*)<>0 THEN IF LEFT*(F I CH*(F,PTRI),LG)<>CLE* THEN 1330
1320 NC = NC+1 : CLE*(NC)=FICH*(F,PTRI): INDEX(NC)=F
1330 NEXT F
1340 GOSUB 1440 ' appel tri
1350 PRINT:PRINT "LISTE TRIEE DES FICHES ":PRINT
1360 '-------------EDITION
1370 FOR F=1 TO NC
1380 X=INDEX(F)
1390 PRINT FICH*(X,1) TAB(IO) FICH*(X,2) TAB(40) FICH*(X,3)
1400 NEXT F
1410 PRINT:INPUT "APPUYER SUR ENTER ":X*
1420 RETURN
1430 '------------------------------------------------------------ tri SHELL-METZNER
1440 ECART=NC
1450 PRINTrPRINT "JE TRIE POUR VOUS ":PRINT
1460 ECART=INT(ECART/2): IF ECART/1 THEN RETURN
1470 J=1 :K=NC-ECART
1480 L = J
1490 M=L+ECART
1500 IF CLE*(L)<=CLE*(M) THEN 1560
1510 X* = CLE*(L): CLE*(L)=CLE*(M): CLE*(M)=X*
1520 X = INDEX(L): INDEX(L)=INDEX(M): INDEX(M)=X
1530 L=L-ECART:IF L<1 THEN 1560
1540 GOTO 1490
1550 '
1560 J = J + 1: IF J>K THEN 1460
1570 GOTO 1480"##########,
    "pages 64, 65, 66"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_064,
    r##########"10 ■ CONVERSION MINUSCULE ---> MAJUSCULE
20 '
30 MODE 2:PAPER 0:PEN 1
40 DIM LIGt(lOOO)
50 INPUT "Nom fichier ";NF$
60 OPENIN NFT
70 ■---------------------LECTURE FICHIER DANS TABLE LIGtO
80 FOR L=1 T0 1000
90 IF EOF=—1 THEN CLOSE IN :NL = L-1 : GOTO 150
100 LINE INPUT #9,LIG$(L)
110 PRINT LIG$(L)
120 NEXT L
130 STOP
140 ■-------------------CONVERSION
150 FOR L=1 TO NL
160 LIG$(L)=UPPER$(LIG$(L) )
170 PRINT LIG*(L)
180 NEXT L
190 '-------------------------ECRITURE FICHIER
200 OPENOUT NF$
210 FOR L=1 TO NL
220 PRINT #9,LIG$(L)
230 NEXT L
240 CLOSEOUT
250 '-------------------------------
260 ' SUR CPC664,0N PEUT OUVRIR UN FICHIER EN LECTURE
270 ' ET UN AUTRE EN ECRITURE S IMULTANEMENT(AVEC DES NOMS
280 ' DIFFERENTS). CHANGEMENT D’UN MOT DANS UN FICHIER SÉQUENTIEL Pour changer un mot dans un fichier séquentiel nous utilisons l'instruction INSTR qui nous donne la position du mot dans chaque ligne. Ce programme peut servir à changer des noms de variables d’un programme sauve­ gardé par “SAVE “XX”,A”."##########,
    "page 67"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_065,
    r##########"10 ' CHANGEMENT D'UN MOT DANS UN FICHIER SEQUENTIEL
20 '
30 MODE 2:PAPER O:PEN 1
40 DIM TXTK 1000)
50 '
60 INPUT "NOM DU FICHIER ";NFJ
70 OPENIN NF$
80 INPUT "ANCIEN MOT ";AM$"##########,
    "page 67"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_066,
    r##########"66 I BASIC AMSTRAD
90 INPUT "NOUVEAU MOT ";NMI
100 '---------------------------------------- LECTURE FICHIER DANS TABLE TXTIO
110 FOR 1=1 TO 1000
120 IF EOF THEN NL = I - 1 : CLOSE IN: GOTO 170
130 LINE INPUT #9,TXTI(I)
140 PRINT TXTI (I)
150 NEXT I
160 '----------------------------------------------
170 FOR 1=1 TO NL
180 DR=1 ' debut recherche dans la ligne
190 '
200 P=INSTR(DR,TXTI( I ) , AMI) ' recherche position ancien mot
210 IF P=0 THEN 240
220 TXTI(I)=LEFTI(TXTI(I),P-1)+NMI + RIGHTI (TXTI ( I ) ,LEN(TXTI(I))- P-LEN (AMI)+1 )
230 DR = P + LEN(NMI)+1 : GOTO 200
240 NEXT I
250 '--------------------------- ECRITURE FICHIER
260 OPENOUT NFI+"B"
270 FOR 1=1 TO NL
280 PRINT #9,TXT$(I)
290 PRINT TXTI(I)
300 NEXT I
310 PRINT-.PRINT "NOUVEAU F I CH I ER :";NFI+"B"
320 CLOSEOUT
330 '
340 'Attention! pour un programme,tai re 'SAVE "XX",A' NOM DU FICHIER ? X ANCIEN MOT ? LIGNE NOUVEAU MOT ? LG"##########,
    "page 68"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_067,
    r##########"10 '--------------- COPIE ECRAN(TEXTE)
20 FOR LIGNE=1 TO 24
30 FOR COL=1 TO 80
40 LOCATE COL,LIGNE:CI=COPYCHRI(#G)
50 PRINT #8,Cl;
60 NEXT COL
70 PRINT #8
80 NEXT LIGNE"##########,
    "page 68"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_068,
    r##########"10 '--------------- COPIE ECRAN(TEXTE)
20 FOR LG=1 TO 24
30 FOR COL=1 TO 80
40 LOCATE COL,LG :CI=COPYCHRI(#0)
50 PRINT #8,Cl;
60 NEXT COL
70 PRINT #8
80 NEXT LG NOUVEAU FICHIER:XB"##########,
    "page 68"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_069,
    r##########"10 ' COMPOSITION DE TEXTES
20 '
30 ' Le texte de base et le texte a ajouter doivent exister.
40 '
50 MODE 2:PAPER O:PEN 1
60 INPUT "NOM DU FICHIER DE BASE"îL$
70 '
80 '-------------------------------Lecture texte de base dans table LIG$()
90 OPENIN L$
100 DIM LIGI(IOO)
110 FOR L=1 TO 100
120 IF EOF THEN NL=L-1:GOTO 180
130 LINE INPUT #9,LIG$(L)
140 PRINT LIGS(L)
150 LIGI(L)=LEFT$(LIG$(L)+STRINGI(78,CHRI(32)),78)
160 NEXT L
170 '
180 CLOSEIN
190 '------------------------------------------
200 INPUT "NOM DU FICHIER A SUPERPOSER ";TS$
210 '
220 INPUT "LIGNE ”;LIGNE
230 INPUT "COLONNE " ; COL
240 OPENIN TS$
250 '
260 LINE INPUT #9,LIG$
270 PRINT LIGI
280 IF EOF THEN CLOSEIN:GOTO 330
290 MIDI(LIG$(LIGNE),COL,LEN(LIG$))=LIG$
300 L1GNE = LI6NE+1
310 GOTO 260
320 '-------------------Edition
330 FOR L=1 TO NL
340 PRINT LIGI(L)
350 NEXT L"##########,
    "page 70"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_070,
    r##########"70 I BASIC AMSTRAD"##########,
    "pages 71, 72"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_071,
    r##########"10 BIBLIOTHEQUE 20
30 MODE 2
40 INK 0,26:INK 1,0:PAPER O:PEN 1
50 NF$="BIBE" ' nom fichier
60 DIM T IT$(200) ,AUT$(200) ,ETAGS(200)
70 DIM CLES(200),INDEX(200)
80 NF=O '' nonommbbrree ddee ffiiches
90 INPUT "NOUVEAU FICHIER (0/N) ";RS
100 IF R$<>"0" AND RSO"o" THEN GOSUB 670
110 '
120 CLS
130 PRINT "MODES:":PRINT
140 PRINT "C :CREATION"
150 PRINT "R :RECHERCHE"
160 PRINT "L : LISTE DU FICHIER"
170 PRINT "S : SUPPRESSION"
180 PRINT "M MODIFICATION"
190 PRINT "LTA :LISTE TRIEE PAR AUTEUR"
200 PRINT "FIN : SAUVEGARDE FICHIER"
210 '
220 PRINT:INPUT "MODE "MS
230 MS=UPPERS(MS)
240 IF MS="C" THEN GOSUB 330
250 IF MS="R" THEN GOSUB 490
260 IF M$="L" THEN GOSUB 590
270 IF MS="S" THEN GOSUB 900
280 IF MS="LTA" THEN GOSUB 1250
290 IF MS="M" THEN GOSUB 1090
300 IF MS="FIN" THEN GOSUB 800:END
310 GOTO 120
320 ========================================== CREATION
330 PRINT:INPUT "TITRE (ou ENTER) ";TITS
340 IF LEN(TITS)=O THEN RETURN ' fin de mode?
350 TITS = UPPERS(T ITS):L = LEN(T ITS) ' titre majuscules
360 FOR F=1 TO NF ' titre existe t-il?
370 IF TITS=LEFTS(TITS(F),L) THEN PRINT TIT$(F): GOTO 330
380 NEXT F
390 NF=NF+1
400 '-----------
410 PRINT
420 INPUT "AUTEUR ";AUTS
430 INPUT "ETAGERE ";ETAGS
440 TITS(F)=TITS
450 AUTS(F)=AUTS
460 ETAGS(F)="ETAG="+ETAGS
470 GOTO 330
480 '===================================== RECHERCHE
490 PRINT: INPUT "Mot de (ou ENTER) ";MCS
500 IF LEN(MC$)=O THEN RETURN
510 PRINT
520 FOR F=1 TO NF
530 L I GS = UPPERS(TITS(F)+AUTS(F)+ETAGS ( F))
540 IF INSTR(LIGS,UPPERS(MCS)) =0 THEN 560
550 PRINT TITS(F) TAB(30) AUTS(F) TAB(45) ETAGS(F)
560 NEXT F
570 GOTO 490
580 ============================= LISTE DU FICHIER
590 PRINTsPRINT "LISTE DU FI CH I ER “ : PR I NT
600 FOR F=1 TO NF
610 PRINT F;TITI(F) TAB(30) AUTI(F) TAB(45) ETAGI(F)
620 IF F MOD 20=0 THEN INPUT "APPUYER SUR ' ENTER" ; XI
630 NEXT F
640 PRINT:INPUT "APPUYER SUR ENTER ";XI
650 RETURN
660 ================= LECTURE FICHIER DANS TABLES
670 OPENIN NFI
680 INPUT #9,NF
690 FOR F=1 TO NF
700 INPUT #9,TITI(F)
710 INPUT #9,AUTI(F)
720 INPUT #9,ETAGI(F)
730 PRINT TITI(F)
740 NEXT F
750 CLOSEIN
760 PRINT NF; "FICHES":PRINT
770 FOR TP=1 TO 1000.-NEXT TP
780 RETURN
790 ================= SAUVEGARDE TABLES DANS FICHIER
800 OPENOUT NFI
810 PRINT #9,NF
820 FOR F=1 TO NF
830 PRINT #9,TITI(F)
840 PRINT #9,AUT$(F)
850 PRINT #9,ETAGI(F)
860 NEXT F
870 CLOSEOUT
880 RETURN
890 ====================================== SUPPRESSION
900 PRINTîINPUT "TITRE (premieres lettres ou ENTER) ";TITI
910 IF LEN(TITI)=O THEN RETURN
920 TITI=UPPERI<TI Tl):L=LEN(TI Tl)
930 FOR F=1 TO NF
940 IF TITI = LEFTI(UPPERI(TI Tl(F)) , L) THEN GOTO 980
950 NEXT F
960 PRINTzPRINT "N'EXISTE PAS": PR I NT : GOTO 900
970 '
980 PRINT:PRINT TI Tl(F); : INPUT " OK (0/N) ";RI
990 IF UPPERI(RI)<>"0” THEN 900
1000 FOR J=F TO NF-1
1010 TI Tl(J)=TI Tl(J +1 )
1020 AUTI(J)=AUTI(J + l )
1030 ETAGI(J)=ETAGI(J + l )
1040 NEXT J
1050 TITl(NF)="":AUTI(NF)="": ETAGI (NF) = " "
1060 NF=NF-1
1070 GOTO 900
1080 ================================== MODIFICATION
1090 PRINT:INPUT "TITRE (premieres lettres ou ENTER) ";TIT$
1100 IF LEN(TIT!)=0 THEN RETURN ' fin de mode?
1110 T I T! = UPPER!(T I T!):L = LEN(T I T!)
1120 FOR F=1 TO NF ' titre existe t-il?
1130 IF TIT!=LEFT!(UPPER!(TIT!(F)) ,L) THEN 1170
1140 NEXT F
1150 PRINT:PRINT "N'EXISTE PAS ":PR I NT : GOTO 1090
1160 '-----------
1170 PR I NT : PR I NT TI T!(F): PR I NT
1180 PRINT AUT! (F) TAB(20) ' ancienne valeur
1190 INPUT "AUTEUR ";X!:IF LEN(X!)<>0 THEN AUTî(F)=X!
1200 PRINT ETAG!(F) TAB(20)
1210 INPUT "ETAGERE ";X!:IF LEN(X!)<>0 THEN ETAG!(F)="ETAG="+X!
1220 '
1230 GOTO 1090 Le tri par auteur s’obtient par :
1240 ======================== SELECTION/TRI PAR AUTEUR
1250 INPUT "Quelle AUTEUR (ENTER pour tous) " ; A!
1260 N C = 0 'nombre de clés
1270 L=LEN(A!)
1280 FOR F=1 TO NF
1290 IF UPPER!(A!)<>UPPER!(LEFT!(AUT!(F),L)) THEN 1310
1300 NC = NC+1 : CLE!(NC)=AUT!(F): INDEX(NC)= F
1310 NEXT F
1320 '
1330 GOSUB 1430 ' appel tri
1340 '---------edition
1350 PRINT:PRINT "LISTE TRIEE PAR AUTEUR": PRINT
1360 FOR F=1 TO NC
1370 X=INDEX(F)
1380 PRINT AUT!(X) TAB(15) TIT!(X)
1390 NEXT F
1400 PRINT:INPUT "APPUYER SUR ENTER";X!
1410 RETURN
1420 '-------------------------------------------------------- tri shell
1430 ECART=NC
1440 ECART=INT(ECART/2): IF ECARTCI THEN RETURN
1450 10 = 0
1460 FOR K=1 TO NC-ECART
1470 J=K+ECART
1480 IF CLE!(J)>=CLE! (K) THEN 1510
1490 X! = CLE!(K): CLE!(K)=CLE!(J): CLE!(J)=X!: 10= 1
1500 X=INDEX(K): INDEX(K)=INDEX(J): INDEX(J)=X
1510 NEXT K
1520 IF 10=1 THEN 1450
1530 GOTO 1440"##########,
    "pages 72, 73, 74"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_072,
    r##########"10 ' DICTIONNAIRE DE SYNONYMES
20 '
30 MODE 2: INK 0,26: INK 1,O:PAPER O:PEN 1
40 NFI="SYNO" ' nom fichier
50 DIM LIGI (200)"##########,
    "page 75"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_073,
    r##########"70 INPUT "NOUVEAU FICHIER (0/N) " ; RI
80 IF RI<>"0" AND RIO"o" THEN GOSUB 470
90 '
100 CLS:PRINT “MODES:":PRINT
110 PRINT "C iCREATION"
120 PRINT "R zRECHERCHE"
130 PRINT "L : LISTE DU FICHIER"
140 PRINT "FIN ■.SAUVEGARDE FICHIER"
150 '
160 PRINT:INPUT "MODE "; Ml
170 MI=UPPERI(Ml)
180 IF MI="C" THEN GOSUB 240
190 IF MI="R" THEN GOSUB 300
200 IF MI="L" THEN GOSUB 390
210 IF MI="FIN" THEN GOSUB 570:END
220 GOTO 100
230 ========================================= CREATION
240 PRINT: INPUT "LISTE DES SYNONYMES (ou ENTER) ";LIG!
250 IF LEN(LIG!)=O THEN RETURN ' -fin de mode?
260 NF=NF+1
270 LIG!(NF)= LIG! ' ou LIG!(NF)=UPPER!(LIG!)
280 GOTO 240
290 ======================================= RECHERCHE
300 PRINT:INPUT "MOT CLE (ou ENTER) ":MC!
310 IF LEN(MC!)=O THEN RETURN
320 PRINT
330 FOR F=1 TO NF
340 IF INSTR(UPPER!(LIG!(F)),UPPER!(MCÎ))=0 THEN 360
350 PRINT LIG!(F)
360 NEXT F
370 GOTO 300 .
380 ================================== LISTE DU FICHIER
390 PRINT:PRINT "LISTE DU F I CH I ER":PR I NT
400 FOR F=1 TO NF
410 PRINT F;LIG!(F)
420 IF F MOD 20=0 THEN INPUT "APPUYER SUR ENTER";X!
430 NEXT F
440 PRINT:INPUT "APPUYER SUR ENTER ";X!
450 RETURN
460 -=========== LECTURE FICHIER DANS TABLE LIGÎO
470 OPENIN NF!
480 INPUT #9,NF
490 FOR F=1 TO NF
500 LINE INPUT #9,LI G!(F): PR I NT LIG!(F)
510 NEXT F
520 CLOSEIN
530 PRINT NF;"FICHES": PRINT
540 FOR TP=1 TO 1000:NEXT TP
550 RETURN
560 '======================== SAUVEGARDE FICHIER
570 OPENOUT NF!
580 PRINT #9,NF
590 FOR F=1 TO NF:PRINT #9,LIG!(F): NEXT F
600 CLOSEOUT
610 RETURN"##########,
    "pages 75, 76"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_074,
    r##########"10 JUILLET 1985 /MENU = CAROTTES STEAK SALADE /1 NV ITE = DUPONT
12 JUILLET 1985 /MENU=THON COTE DE PORC RIZ AU LAI T/I NV ITE = DUPONT
16 JUILLET 1985 /MENU=CAROTTES GRATIN DAUPHINOIS RIZ AU LAIT /INVITE=DUPONT MOT CLE ? RIZ AU LAIT"##########,
    "page 77"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_075,
    r##########"12 JUILLET 1985 /MENU = THON COTE DE PORC RIZ AU LAI T/I NV ITE = DUPONT
16 JUILLET 1905 /MENU=CAROTTES GRATIN DAUPHINOIS RIZ AU LAIT /I NV ITE = DUPONT GESTION DE CHÈQUES ________________________________ Des chèques sont stockés dans des tables. Nous éditons la liste des chèques pour la période choisie ainsi que le solde. Les dates doivent être entrées sous la forme “année/ mois/jour” afin de permettre la sélection entre deux dates. Nous avons également prévu une ventilation par catégorie de dépenses. NCHEQSO LIB$() CT$() DT$() MT()"##########,
    "page 77"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_076,
    r##########"76 I BASIC AMSTRAD Date debut (AA/MM/J J) (ou ENTER) ? Date Tin (AA/MM/JJ)(ou ENTER) ? No cheque Libelle Credit Debit"##########,
    "pages 77, 78"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_077,
    r##########"10 ' GESTION DE CHEQUES
20 '
30 MODE 2: INK 0,0:INK 1,26:PAPER 0:PEN 1
40 NCH = O ' nombre de cheques
50 DIM NCHEQK200) ,DTS(200) ,LIB$(200) ,MT(200) ,CT$(200)
60 DIM CS ( 20 ),MC(20)
70 ' SO INPUT "Nouveau fichier (0/N) ";R$
90 IF R$<>"0" AND R$<>"o" THEN GOSUB 900
100 ======================= MENU
110 CLS:PRINT "MODES:":PRINT
120 PRINT TAB(3) "C :CREATION/MODIFICATION CHEQUE"
130 PRINT TAB (3) "L : LISTE DES CHEQUES"
140 PRINT TAB(3) "S SUPPRESSION CHEQUE"
150 PRINT TAB(3) "LC :MONTANT PAR CATEGORIE"
160 PRINT TAB(3) "F I N : SAUVEGARDE FICHIER"
170 '
180 PRINT:INPUT "Mode ";M$
190 M$=UPPER$(M$)
200 IF M$="C" THEN GOSUB 270
210 IF Mt="L" THEN GOSUB 510
220 IF M$="S" THEN GOSUB 1000
230 IF M$="LC" THEN GOSUB 1180
240 IF M$="FIN" THEN GOSUB 780:END
250 GOTO 110
260 '=============== CREATION/MODIFICATION
270 PRINTîINPUT "Numéro cheque (ENTER pour fin) ";X$
280 IF LEN(X$)=O THEN RETURN ' fin de mode?
290 FOR RANG=1 TO NCH ' cheque existe t-il?
300 IF UPPERS(X$)=UPPER$(NCHEQI(RANG)) THEN 360
310 NEXT RANG
320 NCH=NCH+1
330 RANG=NCH
340 NCHEQS(NCH)= X$
350 -
360 PRINT
370 PRINT DTS(RANG) TAB(20) ' ancienne valeur
380 INPUT "Date (AA/MM/JJ) ";X$
390 IF LEN(X$)<>0 THEN DT$(RANG)=X$
400 PRINT LIBS(RANG) TAB(20)
410 INPUT "Libelle ";X$
420 IF LEN(XS)<>0 THEN LIB$(RANG)= X$
430 PRINT MT(RANG) TAB(20)
440 INPUT "Montant " ; X
450 IF XOO THEN MT (RANG) =X
460 PRINT CTS(RANG) TAB (20)
470 INPUT "Categorie ";X$
480 IF LEN(XIX>0 THEN CTS(RANG) =X$
490 GOTO 270
500 ======================== LISTE DES CHEQUES
510 PRINT
520 INPUT "Date debut (AA/MM/JJ)(ou ENTER) ";DB$
530 INPUT "Date fin (AA/MM/JJ)(ou ENTER) ";DF$
540 ASOLDE=O:DEBIT=O:CREDIT=O
550 PRINT
560 PRINT "No cheque" TAB(ll) "Libelle" TAB(52) "Credit" TAB(64) "Debit"
570 PRINT
580 FOR F=1 TO NCH
590 IF LEN(DB$)<>0 AND LENtDFDOO AND DTKFXDBI THEN ASOLDE=ASOLDE+MT(F)
600 IF LEN(DB$)<>0 AND LEN(DF$)<>0 AND (DTKFXDBI OR DT$(F)>DF$) THEN 690
610 PRINT NCHEQI(F) TAB(ll) LIBI(F) TAB(34) CTI(F) TAB(40)
620 PRINT DTI(F) TAB(50)
630 IF MT(F)>0 THEN PRINT TAB(50) USING "######.##";MT(F)
640 IF MT(F)<0 THEN PRINT TAB(62) USING "######.##";MT(F)
650 IF F MOD 15=0 THEN INPUT "Appuyer sur ENTER";X$
660 '
670 IF MT(F)>0 THEN CRED IT = CREDIT + MT(F )
680 IF MT(F)<0 THEN DEB I T = DEBIT + MT(F)
690 NEXT F
700 PRINT
710 PRINT TAB(50) USING "######.## ";CREDIT;
720 PRINT TAB(62) USING "######.## ";DEBIT
730 PRINT:PRINT "ANCIEN SOLDEASOLDE
740 PRINT:PRINT " SOLDECRED IT + DEBIT +ASOLDE
750 PRINT: INPUT "Appuyer sur ENTER" ;X$
760 RETURN
770 ============================= SAUVEGARDE
780 OPENOUT "CHEQ"
790 PRINT #9,NCH
800 FOR F=1 TO NCH
810 PRINT #9,NCHEQI(F)
820 PRINT #9,DT$(F)
830 PRINT #9,LIBI(F)
840 PRINT #9,MT(F)
850 PRINT #9,CTS(F)
860 NEXT F
870 CLOSEOUT
880 RETURN"##########,
    "pages 79, 80"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_078,
    r##########"900 OPENIN "CHEQ"
910 INPUT #9,NCH
920 FOR F=1 TO NCH
930 INPUT #9,NCHEQI(F),DTI(F),LI Bl(F),MT(F),CTI(F)
940 NEXT F
950 CLOSEIN
960 PRINTzPRINT NCHCHEQUESPR I NT
970 FOR TP=1 TO 2000:NEXT TP
980 RETURN
990 ============================= SUPPRESSION
1000 INPUT "Numéro de chequelENTER pour fin) ";xl
1010 IF LEN(XI)=O THEN RETURN
1020 FOR F=1 TO NOH ' recherche cheque
1030 IF UPPERI(XI)=UPPERI(NCHEQI(F)) THEN 1070
1040 NEXT F
1050 PRINT:PRINT "N'existe pas ":PRINT:GOTO 1000 1060
1070 FOR J = F TO NCH-1
1080 NCHEQI(J)=NCHEQI(J +1 )
1090 LIBI(J)=LI Bl(J+l)
1100 DTI(J)=DTI(J + l )
1110 MT ( J)= MT(J +1)
1120 CTI(J)=CTI(J+l)
1130 NEXT J
1140 NCHEQI(NCH)="LIBI (NCH)="": DTI(NCH)= "":MT(NCH)=O
1150 NCH=NCH-1
1160 GOTO 1000
1170 ================== VENTILATION PAR CATEGORIES
1180 PRINT
1190 NC=O ' nombre categories
1200 FOR 1 = 1 TO 20: Cl(J) =H":MC(J)=0:NEXT I
1210 FOR J=1 TO NCH
1220 FOR K=1 TO NC
1230 IF CTI(J)=CI(K) THEN 1290 ' categorie existe?
1240 NEXT K
1250 NF--NC+1
1260 Cl(NC)=CTI(J)
1270 K = NC 1280
1290 MC (K)= MC(K)+MT(J) ' cumul
1300 NEXT J 1310
1320 FOR J=1 TO NC
1330 PRINT CI(J) TAB(IO) MC(J)
1340 NEXT J
1350 PRINTîINPUT "APPUYER SUR ENTER ";XI
1360 RETURN"##########,
    "pages 80, 81"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_079,
    r##########"10 ' SL SAISIE LETTRE
20 '
30 MODE 2
40 '
50 OPENOUT "LETTRE"
60 '
70 PRINT "* pour fin":PRINT
80 LINE INPUT "? ",LIG$
90 IF LIG$="*" THEN CLOSEOUT:END
100 PRINT #9,LIG$
110 GOTO 80 ? LIBERT PIERRE LE 21.7.1985 ? RUE DE PARIS ? 53000 SOUCE 9 9 ? <NOM> ? <RUE> ? <VILLE> Cher <NOM>,veui11ez trouver ci joint ................... 7 ? ? P.LIBERT ? ♦"##########,
    "page 82"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_080,
    r##########"10 ' EDLB EDITION LETTRE
20 '
30 OPENIN "LETTRE"
40 '---------------------------------- Lecture lettre standard
50 DIM TXTt(lOO)
60 FOR L=1 TO 100
70 IF EOF THEN CLOSE IN:NL= L: GOTO 120
80 LINE INPUT #9,TXTt(L)
90 PRINT TXTt(L)
100 NEXT L
110 '-------------------------------------------- Introduction paramétrés reels
120 PRINT
130 FOR P=1 TO 10
140 PRINT P;
150 LINE INPUT "Paramétré (ex:NOM=BALU /* POUR FIN ) ";Xt
160 IF Xt="»" THEN 210
170 EG=INSTR(Xt," = "): IF EG = O THEN 140
180 PRMt(P)=LEFTt(Xt,EG-1 ):Tt(P)=R I GHTt(Xt, LEN(Xt)-EG)
190 NEXT P
200 '----------------------------------------Edition lettre
210 PRINT
220 FOR LG=1 TO NL
230 LIGt=TXTt(LG)
240 Pl = INSTR(LI Gt,"<")
250 IF P1=O THEN 340
260 P2=INSTR(LI Gt, " > " )
270 Mt = MIDt(LIGt,P1 + 1,P2-P1 -1)
280 FOR NP=1 TO 10
290 IF UPPERt(Mt)=UPPERt(PRMt(NP)) THEN 320
300 NEXT NP
310 PRINT "PARAMETRE NON TROUVE":STOP
320 LI Gt = LEFTt(LI Gt,P1 - 1 )+ Tt(NP)+RIGHTt(LI Gt,LEN(LI Gt)-P2) 330
340 PRINT LIGt
350 NEXT LG"##########,
    "page 83"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_081,
    r##########"10 ' EDITEUR DE LIGNE
20 '
30 MODE 2:PAPER O:PEN 1 : I NK 0,26: INK 1,0
40 C0L=5:LIG=10 ' ligne et colonne
50 GOSUB 80
60 END
70 ■------------------------------------SAISIE D'UNE LIGNE
80 PC= 1 ' position curseur colonne
90 '
100 LOCATE COL+PC-1,LIG:PRINT CHR*(143) ' curseur
110 '
120 C*=INKEY*
130 IF LEN(C*)=0 THEN LOCATE COL + PC-1,LIG : PR I NT MID*(LI G*,PC, 1 ): GOTO 100
140 '
150 C=ASC(C$)
160 L=LEN(LIG*)
170 IF C=13 OR C = 241 THEN LOCATE COL + PC-1 ,LIG : PR I NT MID*(LI G*,PC , 1): RETURN
175 '---------------------suppression
180 IF CO127 THEN 220
190 IF PC< = 1 THEN 120
200 LIG* = LEFT*(LI G*,PC-2)+RIGHT*(LI G*,L-PC+1 ):PC=PC-1
205 LOCATE COL , L IG : PR I NT LIG* SPC(2):G0T0 100
210 '--------- curseur gauche
220 IF C = 242 THEN IF POL THEN LOCATE COL + PC-1 , L IG : PR I NT SPC(l)
230 IF C = 242 THEN IF PO1 THEN LOCATE COL + PC-1, LIG:PRINT MID*(LIG*,PC,1):PC = PC-1 : GOTO 100 ELSE 100
240 '---------curseur droite
250 IF C=243 THEN IF PC<=L THEN LOCATE COL+PC-1, LIG:PRINT M I D*(LI G*,PC,1):PC = PC+1 : GOTO 100 ELSE 100
260 '
270 IF C = 240 THEN RETURN
280 '-------------------------------- caractère normal ajoute
290 IF L>78 THEN 120
300 LIG*=LEFT* (LIG*, PC- 1 )+ CÎ + RIGHT*(LIG*,L-PC+1 )
310 LOCATE COL,LIG:PRINT LIG*
320 '
330 PC=PC+1
340 GOTO 100"##########,
    "page 86"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_082,
    r##########"10 ' EDITEUR DE TEXTE PLEIN ECRAN
20 '
30 MODE 2:PAPER O:PEN 1: INK 0,1:INK 1,24
40 WINDOW #0,1,80,1,21 ' -fenetre texte
50 WINDOW #1,1,80,22,25 ' -fenetre commandes
60 PRINT # 1 , " I nsertion caractères automatique / Fléchés pour depl acer DEL pour sup"
70 PRINT #1,"CTRL A-.sauvegarde CTRL B:ajout ligne CTRL Cisuppre ssion ligne11
80 PRINT #1,"CTRL D:edition imprimante CTRL E et F: saut 10 lign es CTRL Z:fin"
90 C$="":C=0:LIG$="":L=0:PC=0
100 DIM LIG$(200)
110 '
120 PRINT
130 INPUT "NOM TEXTE(ENTER pour nouveau) ";NF$
140 IF LEN(NF$)=O THEN CLS#0:GOTO 210
150 OPENIN NF$
160 FOR L=1 TO 200
170 IF EOF=-1 THEN MX=L-liGOTO 210
180 LINE INPUT #9,LIG$(L)
190 NEXT L
200 '
210 BH= 1 : HP = 19: BB = BH + HP ' bornes haut et bas/hauteur page
220 PC=1 ' pointeur curseur colonne
230 LIG=BH
240 GOSUB 440
250 '---------
260 IF LIG>MX THEN MX=LIG
270 LIG$=LIG$(LIG):GOSUB 530
280 LOCATE 1,LIG-BH +1 : PR I NT LIG$;SPC(1)
290 LIGÎ(LIG)= LIG$
300 IF R=2 THEN IF LIG>1 THEN L I G = LIG- 1 : GOTO 390 ELSE 390
310 IF R = 3 THEN LI G = L IG+1 : GOTO 390
320 IF R = 4 THEN GOSUB 470
330 IF R = 5 THEN GOSUB 500
340 IF R = 6 THEN GOSUB 900
350 IF R = 7 THEN GOSUB 980
360 IF R = 8 THEN IF LIG>10 THEN LIG = LIG-10: GOTO 390 ELSE LIG = 1: GOTO 390
370 IF R = 9 THEN LIG = LIG+10: GOTO 390 ELSE 390 380
390 IF LIG<=BB AND LIG>=BH THEN 260
400 '
410 IF LIG>BB THEN BB = LIG :BH=BB-HP:GOSUB 440:G0T0 260
420 IF LIG<BH THEN IF BH>1 THEN BH = LIG :BB = BH + HP:GOSUB 440:G0T0 260 ELSE 260
430 '------- Affichage page écran
440 CLS #0
450 FOR L = BH TO BBiLOCATE 1,L-BH+1 : PR I NT LIG$(L):NEXT LiRETURN
460 '-----Ajout ligne
470 FOR L = MX TO LIG STEP-1 :LIG$(L+l)=LIG$(L): NEXT L
480 LIG$(LIG)=“”:GOSUB 440:MX = MX + 1: RETURN
490 '------- Suppression ligne
500 FOR L = LIG TO MX :LIG$(L)=LIG$(L +1 ):NEXT L:L I G$ ( MX)= " "
510 MX=MX-1:GOSUB 440:RETURN
520 ■------------------------------------SAISIE D'UNE LIGNE
530 IF PC>LEN(LIG$) THEN PC = LEN (L16$) +1
540 IF LEN(LIG*)=0 THEN PC=1 550
560 LOCATE PC , LIG-BH+1 : PR I NT CHR*(143) 'curseur 570
580 C*=INKEY$
590 IF L E N ( C * ) = 0 THEN LOCATE PC,LIG-BH+1 : PR I NT MID*(LIG*,PC,1): GOTO 560 600
610 C=ASC(C*)
620 L=LEN(LIG*)
630 IF C=240 THEN R=2:RETURN ' fléché haut
640 IF C=13 OR C = 241 THEN R=3:RETURN ' fléché bas
650 IF CO127 THEN 710
660 '-------suppression caractère
670 IF PC<=1 THEN 580
680 LIG* = LEFT* (LI G*,PC-2)+RIGHT*(LI G*,L-PC+1):PC = PC-1
690 LOCATE 1 ,LIG-BH+1: PRINT LIG* SPC(2):G0T0 560
700 '---------curseur gauche
710 IF C = 242 THEN IF POL THEN LOCATE PC,LIG-BH +1 : PRINT SPC ( 1 )
720 IF C = 242 THEN IF PO1 THEN LOCATE PC, LIG-BH+1 : PR I NT MID*(LIG*,PC,1):PC = PC-1 : GOTO 560 ELSE 560
730 '---------curseur droite
740 IF C=243 THEN IF PC<=L THEN LOCATE PC, LIG-BH+1: PRINT MI D*(LI G*,PC,1):PC = PC +1 : GOTO 560 ELSE 560 750
760 IF C=1 THEN R=6:RETURN ' sauvegarde
770 IF C=2 THEN R=4:RETURN ' insertion ligne
780 IF C=3 THEN R=5:RETURN ' suppression ligne
790 IF C=4 THEN R=7:RETURN ' affichage imprimante
800 IF C = 5 THEN R=8:RETURN
810 IF C=6 THEN R=9:RETURN
820 IF C = 26 THEN END
825 IF C<32 THEN 560
830 ---------------------------------- caractère normal ajoute
840 IF L>78 THEN 580
850 LI G* = LEFT*(LI G*,PC-1)+C* + RIGHT*(LI G* , L-PC +1 )
860 LOCATE 1,LIG-BH+1 : PRINT LIG*
870 PC=PC+1
880 GOTO 560
890 ============================ SAUVEGARDE
900 LOCATE #1,1,4:INPUT 11,"NOM ";NF$
910 ÜPENOUT NF*
920 FOR L=1 TO MX
930 PRINT #9,LIG*(L)
940 NEXT L
950 CLOSEOUT
960 RETURN
970 -============= EDITION IMPRIMANTE
980 FOR L=1 TO MX
990 PRINT #8,LIG*(L)
1000 NEXT L
1010 RETURN
1020 '-----Les instructions: LIG * = " " ne comportent pas d'espace en"##########,
    "pages 87, 88"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_083,
    r##########"10 ' JUSTIFICATION A DROITE D'UNE LIGNE
20 '
30 MODE 2
40 LL=30 ' longueur maxi
50 LIG$=“LE PETIT CHAT RONRONNE"
60 '
70 S=1
80 '
90 IF LEN(LIGt)>=LL THEN END
100 FOR K=S T0 LEN(LIG$) ' recherche debut espace
110 X$=MID$(LIG$,K, 1)
120 IF X$=CHR$(32) THEN GOTO 170
130 NEXT K
140 IF LEN(LIG$)<LL THEN 70
150 END
160 '
170 FOR V=K+1 TO LEN(LIG$) ' recherche fin espace
180 X$ = MID$(LIG$,V, 1 )
190 IF X$OCHR$(32) THEN 220
200 NEXT V
210 '
220 LIG$=LEFT$(LI 6$,V-l)+SPACEI(1)+RIGHT$(LIG$,LEN(LIG$)-V +1)
230 PRINT LIG$
240 S=V+1
250 GOTO 90 LE PETIT CHAT RONRONNE LE PETIT CHAT RONRONNE LE PETIT CHAT RONRONNE LE PETIT CHAT RONRONNE LE PETIT CHAT RONRONNE LE PETIT CHAT RONRONNE LE PETIT CHAT RONRONNE LE PETIT CHAT RONRONNE"##########,
    "page 89"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_084,
    r##########"10 ' JUSTIFICATION A DROITE D'UN PARAGRAPHE
20 '
30 MODE 2:PAPER O:PEN 1: INK 0,0:1NK 1,26
40 DIM TXT*(1000)
50 LLIG=60 ' Longueur decoupage ligne
60 '
70 INPUT "NOM DU FICHIER ";NF*
80 OPEN IN NF*
90 ======================== NORMALISATION DES LIGNES
100 ' Les lignes sont lues dans une mémoire tampon
110 ' puis decoupees en lignes de 65 caractères.
120 '
130 NL = O ' nombre de lignes dans T X T t ( )
140 '
150 IF EOF THEN CLOSE IN :GOSUB 230 :NL = NL+1 : TXT*(NL)=TAMP*: GOTO 400
160 '
170 IF LEN(TAMP*)>150 THEN GOSUB 230
180 LINE INPUT #9,LIG*
190 PRINT LIG*
200 TAMP*=TAMP*+LIG*+CHR*(32)
210 GOTO 150
220 '------------------------- Decoupage mémoire tampon dans TXT*()
230 IF LEN(TAMP*XLLIG THEN RETURN
240 D=1:AP=O ' debut -ancienne position
250 '
260 P1 = INSTR(D,TAMP* , CHR*(32)) ' recherche espace
270 P2=INSTR(D , TAMP* , ", " ) ' recherche virgule
280 IF Pl=0 AND P2=0 THEN 340
290 P = MIN(Pl,P2>
300 IF Pl=0 THEN P = P2
310 IF P2 = 0 THEN P = P1
320 IF PCLLIG THEN AP = P:D = P+1 : GOTO 260
330 '
340 X*=LEFT*(TAMP*,AP)
350 IF ASC(RIGHT*(X*,1))=32 THEN X* = LEFT*(X *,LEN(X*)-1 )
360 NL=NL+ 1 : TXT* ( NL)=X* ' transfert TXT*()
370 TAMP*=RIGHT*(TAMP*,LEN(TAMP*)-AP) ' reste
380 GOTO 230
390 '-------------------------EDITION TEXTE NORMALISE
400 PRINT "TEXTE NORMAL I SEPR I N T
410 FOR 1=1 TO NL
420 PRINT TXT*(I)
430 NEXT I
440 ========================= JUSTIFICATION A DROITE
450 PRINTîPRINT "TEXTE JUSTIFIE A DROI TE": PR I NT
460 '
470 FOR L=1 TO NL
480 L I G* = TX T*(L)
490 IF LEN(LIG*)<LLIG*O.75 THEN 700
500 R=0 'témoin espace
510 S=1 ' debut recherche
520 IF LEN(LIG*)> = LLIG THEN 700
530 FOR K=S TO LEN(LIG*) ' recherche espcace
540 X* = MID*(LIG*,K, 1 )
550 IF X*=CHR*(32) THEN R=1:GOTO 610
560 NEXT K
570 IF R = 0 THEN 700
580 IF LEN(L16$)<LL IG THEN GOTO 500
590 GOTO 700
600 '
610 FOR V=K+1 TO LEN(LIG$) ' recherche fin espace
620 X$=MID$(LIG$,V, 1)
630 IF X$OCHR$(32) THEN 660
640 NEXT V
650 '
660 LIG$=LEFT$(LIG$,V-1)+SPACEI(1)+RIGHTi(LIG$,LEN (L16$)-V+1 )
670 S=V+1
680 GOTO 520 690
700 PRINT LIG$
710 NEXT L Exemple de texte à justifier : L’auteur du “MIGRATEUR” n’a évidemment jamais habité quelque part. Il a pourtant fini par poser ses valises dans l’île d’Houat, en Bretagne. Dans la solitude, il écrit l’his­ toire de sa jeunesse, “sa chute hors du lycée, dans le hasard”. Lire pages 18 à 30. □ Texte normalisé L’auteur du “MIGRATEUR” n’a évidemment jamais habité quelque part. Il a pourtant fini par poser ses valises dans l’île d’Houat, en Bretagne. Dans la solitude, il écrit l’histoire de sa jeunesse, “sa chute hors du lycée, dans le hasard”. Lire pages 18 à 30. □ Texte justifié à droite L’auteur du “MIGRATEUR” n’a évidemment jamais habité quelque part. Il a pourtant fini par poser ses valises dans l’île d’Houat, en Bretagne. Dans la solitude, il écrit l’histoire de sa jeunesse,“sa chute hors du lycée, dans le hasard”. Lire pages 18 à 30."##########,
    "pages 90, 91"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_085,
    r##########"10 ' MAILING
20 '
30 ' Les fichiers lettre et adresses doivent déjà exister
40 ' Cf programme 'fichier d'adresses' et éditeur de texte.
50 '
60 MODE 2:PAPER 0:PEN 1
70 INPUT "NOM DU FICHIER LETTRE " ;L$
80 INPUT "NOM DU FICHIER ADRESSE ";NF*
90 '--------------------------------Lecture lettre dans table L I G$ ( )
100 OPENIN L*
110 DIM LIG*(100)
120 FOR L=1 TO 100
130 IF EOF THEN NL=L-1:GOTO 180
140 LINE INPUT #9,LIG*(L)
150 PRINT LIG*(L)
160 NEXT L
170 '
180 CLOSEIN
190 '------------------------------------------
200 INPUT "QUELLE VILLE (ENTER POUR TOUTES)":V*
210 '
220 OPENIN NF*
230 INPUT #9,NFI CH : PR INT NFI CH ;"FI CHES": PR I NT
240 '
250 IF EOF THEN CLOSEINzEND
260 LINE INPUT #9,NOM*
270 LINE INPUT #9,RUE*
280 LINE INPUT #9,VILLE*
290 LINE INPUT #9,CPST*
300 IF LEN(V*)<>0 THEN IF VILLE*OV* THEN 250
310 '------------------------------------Edition lettre
320 PRINT TAB(30) NOM*
330 PRINT
340 PRINT TAB(35) RUE*
350 PRINT TAB(35) CPST* SPC(l) VILLE*
360 PRINT
370 PRINT
380 FOR L=1 TO NL
390 PRINT LIG*(L)
400 NEXT L
410 PRINT:INPUT "APPUYER SUR ENTER ";
420 GOTO 250"##########,
    "page 93"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_086,
    r##########"10 ' GESTION DE CLUBS SPORTIFS
20 '
30 MODE 2:PAPER O:PEN 1:INK 0,1:INK 1,24
40 DIM NOM*(100),CLUB*(100),PO I NT(100)
50 DIM CLE*(100),INDEX(100)
60 NFICH=O ' nombre de fiches
70 INPUT "Nouveau fichier (0/N) ":R*
80 IF R*="0" OR R*="o" THEN 110
90 GOSUB 660 ' appel lecture du fichier
100 '
110 CLS:PRINT "Modes: " : PR I NT
120 PRINT TAB(3) "C: creation/modification"
130 PRINT TAB(3) "LF: liste du fichier"
140 PRINT TAB(3) "LFN: liste triee par nom"
150 PRINT TAB(3) "LFC: liste triee par club"
160 PRINT TAB(3) "LFP: liste du fichier par points"
170 PRINT TAB(3) "TC: total par clubs"
180 PRINT TAB(3) "FIN: fin de session (sauvegarde)"
190 PRINT:INPUT "Mode " : M*
200 M*=UPPER*(M*)
210 IF M*="C" THEN GOSUB 300
220 IF M*="LFN" THEN GOSUB 1000
230 IF M*="LFC" THEN GOSUB 1140
240 IF M*="LFP" THEN GOSUB 1280
250 IF M*="TC" THEN GOSUB 1430
260 IF M*="FIN" THEN GOSUB 550:END
270 IF M*="LF" THEN GOSUB 780
280 GOTO 110
290 ================================ CREAT I ON/MOD IF ICAT I ON
300 PRINT
310 PRINT "ENTER pour zone inchangee(en modif)": PR I NT
320 INPUT "Nom (ENTER pour fin) "; NOM*:NOM* = UPPER*(NOM*)
330 IF LEN(NOM*)=O THEN RETURN ' fin de mode?
340 '
350 IF NFICH=O THEN 400
360 FOR RANG=1 TO NFICH
370 IF NOM*=NOM*(RANG) THEN 460 ' nom existe t-il?
380 NEXT RANG
390 '-------------------------------- nouveau nom
400 PRINT : INPUT "Nouveau nom OK (0/N) ";R*
410 IF R*<>"0" AND R*<>"o" THEN 300
420 NF ICH = NFICH+1
430 RANG=NFICH
440 NOM*(RANG)=NOM*
450 '--------------------------------------------entree/modification zones
460 PRINT
470 PRINT CLUB*(RANG) TAB(15) ' ancienne valeur
480 INPUT "Club CLUB*:CLUB*=UPPER*(CLUB*)
490 IF LEN(CLUB*)<>0 THEN CLUB*(RANG)=CLUB*
500 PRINT POINT(RANG) TABU5)
510 INPUT "Points ";POINT
520 IF POINTOO THEN PO I NT ( RANG)=P01 NT
530 GOTO 300
540 ================================== SAUVEGARDE
550 0PEN0UT "CLUBS"
560 '
570 PRINT #9, NF I CH
580 FOR F=1 TO NFICH
590 PRINT #9,NOM*(F)
600 PRINT #9,CLUB*(F)
610 PRINT #9, POINT(F)
620 NEXT F
630 CLOSEOUT
640 RETURN
650 ■=============================== LECTURE DU FICHIER
660 OPENIN "CLUBS"
670 '
680 INPUT #9,NFICH
690 FOR F=1 TO NFICH
700 INPUT #9,NOM*(F)
710 INPUT #9,CLUB*(F)
720 INPUT #9,P0INT(F)
730 NEXT F
740 CLOSEIN
750 PRINTzPRINT NF I CH ;"Fich es"
760 FOR TP=1 TO 2000:NEXT TP
770 RETURN
780 '=================================== liste du fichier
790 PRINT "liste du fichier": PR I NT
800 '
810 FOR F=1 TO NFICH
820 IF F MOD 20=0 THEN INPUT "APPUYER SUR ENTER";X*
830 PRINT NOM*(F) TAB(15) CLUBJ(F) TAB(30> POINT(F)
840 NEXT F
850 PRINT:INPUT "Appuyer sur <ENTER> ";X*
860 RETURN
870 '----------------------------------------------------- tri shel 1
880 ECART=NCL
890 ECART=INT(ECART/2): IF ECARTXl THEN RETURN
900 IV = O
910 FOR K=1 TO NCL-ECART
920 J=K+ECART
930 IF CLE* (J)> = CLE* (K) THEN 960
940 X*=CLE*(K): CLE*(K)=CLE*(J):CLE*(J)=X*:I V=1
950 X = INDEX (K): INDEX(K)=INDEX(J): INDEX(J)=X
960 NEXT K
970 IF IV=1 THEN 900
980 GOTO 890
990 ================== LISTE TRIEE PAR NOM
1000 NCL=O 'nombre de clés
1010 FOR F=1 TO NFICH
1020 NCL = NCL+1:CLE*(NCL)=NOM*(F): INDEX(NCL)=F
1030 NEXT F
1040 GOSUB 880
1050 '--------------EDITION
1060 PRINT:PRINT "LISTE TRIEE PAR NOMS":PRINT
1070 FOR F=1 TO NCL
1080 X=INDEX(F)
1090 PRINT NOM* (X) TAB(15) CLUB*(X) TAB(40) POINT(X)
1100 NEXT F
1110 PRINT: INPUT "APPUYER SUR ENTER ";X*
1120 RETURN
1130 ============================= TRI PAR CLUB
1140 N C L = 0 'noubredecles
1150 FOR F = 1 TO NFICH
1160 NCL = NCL+1: CLES(NCL)=CLUB$(F)+ NOMS(F): INDEX ( NCL ) =F
1170 NEXT F
1180 GOSUB 880
1190 '--------EDITION
1200 PRINT:PRINT "LISTE TRIEE PAR CLUBS":PRINT
1210 FOR F=1 TO NCL
1220 X = INDE X(F)
1230 PRINT CLUBS (X) TAB(15) NOM*(X) TABI30) POINT (X)
1240 NEXT F
1250 PRINT:INPUT "Appuyer sur ENTER";X$
1260 RETURN
1270 ========================= TRI par points
1280 NCL = O
1290 FOR F=1 TO NFICH
1300 NCL=NCL+1
1310 CLES(NCL)=RIGHT$(" "+STRS(PO I NT(F)),4)
1320 INDEX(NCL)=F
1330 NEXT F
1340 GOSUB 880
1350 -------------EDITION
1360 FOR F=1 TO NCL
1370 X=INDEX(F)
1380 PRINT POINTU) TAB(IO) NOMS (X)
1390 NEXT F
1400 PRINT:INPUT "APPUYER SUR ENTER ";XS
1410 RETURN
1420 =============================== TOTAL par club
1430 NCLUB = O
1440 FOR F=1 TO 10:TP(F)=0:NEXT F
1450 FOR F=1 TO NFICH
1460 FOR C=1 TO NCLUB
1470 IF CLUBS(F)=CLUBS(C) THEN 1520
1480 NEXT C
1490 NCLUB=NCLUB+1 ' nouveau club
1500 C=NCLUB:CLUBS(NCLUB)=CLUBS(F) 1510
1520 TP (C)=TP(C)+POINT(F)
1530 NEXT F
1540 ‘-----------EDITION
1550 PRINT:PRINT "TOTAL PAR CLUB":PRINT
1560 FOR C=1 TO NCLUB
1570 PRINT CLUBS(C),TP(C)
1580 NEXT C
1590 PRINT:INPUT "APPUYER SUR ENTER";XS
1600 RETURN"##########,
    "pages 95, 96, 97"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_087,
    r##########"10 'idlv INDEX DE LIVRE
20 '
25 MODE 2:PAPER 0:PEN 1
30 DIM CLESI200),PGS(200)
40 DATA 81,ABS
50 DATA 22,AND
60 DATA 59,ASC
70 DATA 94,CLEAR
80 DATA 15,AUTO
90 DATA 60,CHRS
100 DATA 63,BINS
110 DATA 28,BORDER
120 DATA *
130 '-------------------LECTURE DATAS DANS TABLES
140 FOR J=1 TO 200
150 READ PGS(J): IF PGS(J)="«" THEN NM=J-1:GOTO 200
160 READ CLES(J)
170 PRINT PGS ( J) , CLES ( J )
180 NEXT J
190 '----------------------- TRI
200 IV=0
210 FOR J = 1 TO NM-1
220 IF CLES (J + l )>=CLES (J) THEN 250
230 XS=CLES(J):CLES(J)=CLES(J + 1 ):CLES(J+l)= XS
240 XS = PGS ( J) :PGS(J)=PGS(J +1 ):PGS(J+l)=XS:IV=1
250 NEXT J
260 IF IV=1 THEN 200
270 '--------------------------------------EDITION
280 CN=O ' CN=8 pour imprimante
290 FOR J=1 TO NM
300 IF LEFTS(CLES(J-1),1)=LEFTS(CLES(J) , 1 ) THEN 330
310 PRINT #CN:PRINT #CN, LEFTS(CLES(J),1): PR I NT #CN
320 '
330 PRINT #CN,TAB(3) CLES(J) STR INGS(18-LEN(CLES(J) i
340 PRINT #CN,TAB(22) PGS(J)
350 NEXT J A ABS.................................. 81 AND.................................. 22 ASC.................................. 59 AUTO................................ 15 B BINS................................ 63 BORDER........................... 28 C CHRS................................ 60 CLEAR............................. 94"##########,
    "page 98"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_088,
    r##########"10 ' HISTOGRAMME CIRCULAIRE (CPC664)
20 '
30 MODE 1
40 INK 0,26:INK 1,O:PAPER O:PEN 1
50 INK 2,2:INK 3,6
60 '
70 DATA AMSTRAD,0.3, MSX,O. 1
80 DATA APPLE,0.2, IBM,0.3
90 DATA ATARI,0.1
100 '
110 FOR 1 = 1 TO 5:READ HI(I),H(I): NEXT I
120 '
130 XA=200:YA = 200 ' centre
140 R=60 ' rayon
150 '--------------------------------------
160 AD=O ' angle debut
170 FOR P=1 T0 5
180 CE =(P MOD 3)+l ' couleur
190 XD=XA+R«COS(AD):YD=YA+R*SIN(AD)
200 PLOT XA,YA:DRAW XD,YD,CE
210 AF=AD + P I *2*H ( P ) ' angle -fin
220 GOSUB 370
230 X=XA+R*COS(AF):Y = YA + R*SIN ( AF)
240 PLOT XA,YA:DRAW X,Y,CE
250 '
260 AT=AD+PI♦H(P) ' affichage texte
270 X=XA+R*1.4»C0S(AT):Y=YA+R»1.4*SIN(AT)
280 ■-M'OVE XA + R/2*C0S(AT) , YA + R/2*S IN ( AT)
290 FILL CE
300 IF AT>PI/2 AND AT<3»PI/2 THEN X=X-16*LEN(HI(P))
310 PLOT X , Y : TAG:PR I NT HI(P);
320 AD=AF
330 NEXT P
340 PEN 1
350 END
360 ■---------------------------PORTION DE CERCLE
370 FOR A=AD TO AF+0.05 STEP 0.05
380 X=XA+R*COS(A)
390 Y = YA + R*SIN(A)
400 DRAW X,Y,CE
410 NEXT A MSX AMSTRAD
420 RETURN APPLE fl ATARI IBM"##########,
    "page 99"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_089,
    r##########"10 -------------------------HISTOGRAMME DOUBLE
20 MODE 1
30 INK 0,26:INK 1,6:INK 2,0:PAPER O:PEN 1
35 DIM MOIS*(12),VNTE1(12),VNTE2(12)
40 NM=7 ' nombre de mois
50 DATA JV,500,300, FV,400,800
60 DATA MRS,600,700, AVR,700,600
70 DATA MAI,600,400, JUIN,500,600
80 DATA JL,700 ,780
90 ' —
100 FOR M=1 TO NM:READ MOISI(M),VNTE1(M),VNTE2(M): NEXT M
110 '
120 XA=80:YA=30 ' depart axes
130 IX=70 ' intervalle X
140 HECR=300 ' hauteur écran
150 '-------------------------------------------------------------- recherche «axi
160 MX=VNTE(1)
170 FOR M=2 TO NM
180 IF VNTE1(M)>MX THEN MX = VNTE1 (M)
190 IF VNTE2(M)>MX THEN MX=VNTE2(M)
200 NEXT M
210 ECH=HECR/MX ' echelle
220 '-------------------------------------------------------------- axes
230 PLOT XA,YA:DRAW XA+NM*I X,YA,1
240 PLOT XA,YA:DRAW XA,HECR+YA,1
250 '------------------------------------------------------------------ affichage mois
260 FOR M=1 TO NM
270 X=XA+8+IX*(M-l):Y=YA-12
280 PLOT X,Y : TAG : PR I NT MOIS$(M);
290 NEXT M
300 '---------------------------------------------------------------- courbe 1
310 FOR M = 1 TO NM
320 X1 = XA+10+1 X*(M-1): Y1 = YA + VNTE1(M)»ECH
330 FOR DX=O TO 5
340 PLOT Xl+DX,YA:DRAW Xl+DX,Yl,l
350 NEXT DX
360 NEXT M
370 '---------------------------------------------------------------- courbe 2
380 FOR M=1 TO NM
390 X1 = XA+20+1 X»(M-1 ): Y1 = YA + VNTE2(M)«ECH
400 FOR DX=O TO 5
410 PLOT Xl+DX,YA:DRAW Xl+DX,Yl,2
420 NEXT DX
430 NEXT M
440 '---------------------graduations
450 NG=10 ' nombre de graduations
460 PAS = INT(MX/NS)
470 IF PAS>50 AND PAS<100 THEN PAS=100
480 IF PAS>10 AND PAS<50 THEN PAS=50
490 NG=INT(MX/PAS)
500 IG=HECR/NG ' intervalle graduations
510 FOR 6=1 TO NG
520 MOVE 1,YA+G»I6
530 TAGîPRINT G*PAS;
540 NEXT G"##########,
    "page 101"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_090,
    r##########"10 'MOT MOT LE PLUS LONG
20 '
30 MODE 1: INK 0,0: INK 1,26:PAPER O-.PEN 1
40 NM = 6 ' Nombre de lignes
50 DATA FI CH IERSW,FI CH I ERS,♦
60 DATA PEDALESTO,PEDALES,PETALES,PEDALOS,»
70 DATA MOQUETTEA,MOQUETTE,MAQUETTE,*
80 DATA ARMOIRESI ,ARMOIRIES,»
90 DATA FAUTEUILY,FAUTEUIL,»
100 DATA CHEVELUAR,CHEVELU,CHALEUR,CHAULER,» HO '-----------------
120 RESTORE ' debut DATA
130 CLS
140 FOR J = 1 TO 9 : F(J)=0 :NEXT J
150 X=INT(RND(1>*NM) ' mot au hasard
160 FOR J=1 TO X ' lecture des precedents
170 READ X$:IF X$<>"»" THEN 170
180 NEXT J
190 '------- stockage réponses
200 READ MPS
210 FOR J = 1 TQ 5
220 READ MOT$(J)
230 IF MOT$(J)="*" THEN 260
240 NEXT J
250 '---------Affichage des lettres au hasard
260 LOCATE 10,10
270 FOR J=1 TO 9
280 P=1 +1 NT(RND(1)»9)
290 IF F(P)=O THEN PRINT MI DS(MPS , P,1); SPC(1); : F(P) = 1 ELSE 280
300 NEXT J
310 PRINT
320 HD=TIME
330 '-------POUR VERSION CHRONO: 340 GOSUB 520
340 LOCATE 6,20:INPUT "Votre reponse(ou FIN) ";LIG$
350 LOCATE 1,22:PRINT INT((TIME-HD)/400);"secondes"
360 '
370 LIG$=UPPERS(LIGS): IF LIGS="FIN" THEN END
380 FOR J = 1 TO 5
390 IF MOT$(J)="»" THEN 440
400 IF LIG$=MOTS(J) THEN LOCATE 1,15:PRINT "OK":GOTO 480
410 NEXT J
420 '
430 '----affichage bonnes repenses
440 FOR J=1 TO 5
450 IF MOTS(J)="»" THEN 480
460 LOCATE 1,14+J:PRINT MOTS(J)
470 NEXT J
480 FOR TP=1 TO 3000:NEXT TP
490 GOTO 120
500 ■-----------------------ENTREE AVEC CHRONO--
510 ' Ajouter les lignes 10 a 490
520 HD=TIME
530 AFTER 800,0 GOSUB 600
540 XS=6:YS=20
550 LOCATE XS,YS:PRINT “VOTRE REP0NSE?(ou FIN) ";SPC(12);
560 XS=XS+23:GOSUB 640
570 X=REMAIN(O)
580 RETURN
590 -----------SOUS PROGRAMME CHRONO
600 LOCATE 1,23:PRINT “TROP TARD"
610 TT=1
620 RETURN
630 '------------------- saisie d'une ligne avec INKEY$
640 LIG$="" ' pas d'espace entre les ""
650 TT=O ' temps
660 '
670 LG = LEN(LIG$):LOCATE XS + LG , YS: PR I NT CHRt ( 1 43);CHR$ ( 32) ' 143 : curseur
680 LOCATE XS+LG,YS
690 '
700 C$=INKEY$
710 IF TT=1 THEN RETURN
720 IF LEN(C$)=O THEN LOCATE 1,22:PRINT I NT((TIME-HD)/400): GOTO 670
730 C=ASC(C$)
740 '
750 IF C<>127 THEN 780 ' code suppression
760 IF LG>0 THEN LIGI= LEFTS(LIG$,LG-1): PR I NT CHRS (8);CHRS(32): GOTO 670 ELSE 670
770 '
780 IF C=13 THEN 840 ' code ENTER?
790 IF C<32 OR 0127 THEN PRINT CHRS ( 7) ; : GOTO 700
800 LIG$=LIG$+C$ ' ajout caractère
810 PRINT C$ ' af-fichage caract.
820 GOTO 670
830 ’
840 LOCATE XS + LG,YS: PR I NT CHR$(32)
850 RETURN"##########,
    "pages 104, 105"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_091,
    r##########"10 LE COMPTE EST BON 20
30 D'apres J.BUISSON (ORDINATEUR INDIVIDUEL 54BI 40
50 NB ( ) : nombres
60 RP O : résultats partiels
70 AD ( ) : pointeurs vers additions"##########,
    "page 107"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_092,
    r##########"30 DV ( ) : pointeurs vers divisions
90 OC ( ) : témoin occupation de N B( ) 100
110 MODE 2:PAPER O:PEN 1 : I NK 0,26:INK 1,0
120 INPUT "Nombre a trouver “;RP(O) 130
140 FOR J = 1 TO 6:INPUT "Nombre ";NB(J):NEXT J 150
160 FOR J = 1 TO 6:OC(J)=0:NEXT J
170 NV=1 180
190 AD(NV)=0 :DV(NV) = 1 ' Initialisation pointeur 200
210 IF OC(AD(NV))=1 THEN 320 ' Déjà utilise 22,0 IF OC(DV(NV))=1 OR A D ( N V ) = D V ( N V ) THEN 290 ' memes nombres 230
240 RP(NV) = (RP(NV-1)-NB(AD(NV)))/NB(DV(NV) )
250 IF RP(NV)=ABS(I NT(RP ( NV))) THEN 0P$ ( NV)=" + ":GO TO 380
260 RP(NV)=(RP(NV-1)+NB(AD(NV)))/NB(DV(NV))
270 IF RP(NV)=ABS(INT(RP(NV))) THEN 0P$ ( NV)="-":GOTO 380 280
290 DV(NV)= DV(NV)+1 : IF DV(NV)<=6 THEN 220
300 DV(NV)=1 310
320 AD(NV)=AD(NV)+1 s IF AD(NV)<=6 THEN 210
330 '--------------------------- pas de solution de niveau N
340 IF NV=1 THEN PRINT "Pas de sol utionSTOP 350
360 NV=NV-1:OC(AD(NV))=0:OC(DV(NV))=0:GOTO 290
370 '--------------------------------solution niveau NV
380 IF RP(NV)=O OR RP(NV)=1 THEN 440
390 IF AD(NV)<>0 THEN OC(AD(NV))=1
400 OC(DV(NV))=1
410 —
420 NV=NV+1:GOTO 190
430 '------------------------------------AHichage résultats
440 PRINTiPRINT "Voici le compte": PRINT
450 FOR J=NV TO 1 STEP-1
460 IF RP(J)=O THEN 530
470 IF RP(J)=1 THEN 510
480 IF NB(DV(J))=1 THEN 510
490 PRINT RP(J) "X" NB(DV(J)) " = " RP(J)*NB ( DV ( J)) 500
510 IF NB(AD(J))=0 THEN 530
520 PRINT RP(J)*NB(DV ( J)) OPI(J) NB(AD(J)) " = “ RP(J-l)
530 NEXT J
540 ---------------------------------P0L)R TEST
550 PRINT
560 FOR 1=0 TO NVîPRINT RP ( I) , NB ( AD ( I )) , NB(DV( I )):NEXT I"##########,
    "page 107"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_093,
    r##########"106 I BASIC AMSTRAD On pourra effectuer des divisions du type suivant en faisant NB(7)=1 : NCHERCHE+-N Il faut pour cela ajouter au programme précédent :
155 NB(7)=1"##########,
    "page 108"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_094,
    r##########"290 DV(NV)=DV(NV) +1îIF DV(NV)< = 7 THEN 220
380 IF DV(NV)=7 AND AD(NV)= 0 THEN 290
381 IF RP(NV)=1 AND DV(NV)=7 THEN 290
382 IF NV>4 THEN 340
385 IF RP(NV)=0 OR RP(NV)=1 THEN 440
390 IF AD(NV)<>0 THEN 0C(AD(NV))=1
400 IF DV(NV)<>7 THEN 0C(DV(NV))=1 Le nombre de solutions augmente sensiblement mais le temps de recherche augmente aussi très nettement. Essayez de trouver 876 avec 50,8,7,4,3,25. On obtient : 25-3=22 22*8 = 176 176-7=169"##########,
    "page 108"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_095,
    r##########"10 ' JEU DE MARIENBAD
20 '
30 '
40 ' SI VOUS COMMENCE Z,VOUS PERDEZ
50 '
60 MODE 2:PAPER O:PEN 1 : I NK O,O:INK 1,26
70 J EU(1)=7 : J EU(2)=5
80 JEU(3)=3:JEU(4)=1
90 '
100 GOSUB 910
110 '-------
120 PRINTîPRINT "ESC 2 Pois pour FIN":PRINT
130 INPUT "Voulez vous commencer (0/N) M ; RI
140 JR = O:IF R$="0" OR Rl="o" THEN JR=1
150 '
160 FP=O ' Pin de partie
170 '
180 IF JR=1 THEN GOSUB 230;JR=1-JR
190 IF FP=b THEN 70
200 IF JR = O THEN GOSUB 350:JR=1-JR
210 IF FP=1 THEN 70
220 GOTO 180
230 ======================= JOUEUR
240 PRINTiINPUT "Quelle ligne ";L
250 INPUT "Combien ";N
260 '
270 IF N>JEU(L) OR N<=0 THEN PRINT:PRINT "Vous tri chez PRINT:PRINT:GOTO 240
280 JEU(L)=JEU(L)-N
290 '
300 GOSUB 910 ' aPPichage partie en cours
310 GOSUB 980 ' Fin de partie?
320 IF S=1 THEN PR I NT : PR I NT : PR I NT "Vous avez gagne":FP=l
330 RETURN
340 =============================== MACHINE
350 FOR L=1 TO 4
360 X=JEU(L):GOSUB 700
370 NEXT L
380 '
390 '--------------------------- RECHERCHE JEU GAGNANT
400 FOR L=1 TO 4 '4 lignes
410 FOR N=1 TO JEU(L) ' on essaie d'en prendre 1 a JEU(L)
420 IF JEU(L)=0 THEN 480
430 X = JEU(L)-N:GOSUB 680
440 GOSUB 760 ' sommation colonnes
450 IF GN=1 THEN 550
460 NEXT N
470 X = JEU(L):GOSUB 680 ' retabliss. ligne binaire de B( ,)
480 NEXT L
490 '---------------------------------- PAS DE SOLUTION GAGNANTE
500 FOR L=1 TO 4
510 IF JEU(L)>0 THEN N=1:GOTO 550
520 NEXT L
530 STOP 540
550 IF SMI 1)00 OR SM(2)<>0 THEN 610
560 IF SM(0)=2 AND N=JEU(L) AND JEU(L)>1 THEN N = JEU(L)-1 : GOTO 610
570 IF SM(0)=4 THEN N = JEU(L): GOTO 610
580 IF SM(0)=2 THEN N = JEU(L): GOTO 610
590 IF SM(O)=O AND N>1 THEN N=JEU(L)-1
600 1
610 PRINT:PRINT "Je joue: ligne:";L;" J'en prendS;";N 620
630 J EU(L)=J EU(L)-N
640 GOSUB 910:G0SUB 980
650 IF 5=1 THEN PRINT:PRINT "J'AI GAGNE ":PRINT:FP=1
660 RETURN
670 '-------CV BINAIRE
680 ' ENTREE:X,L SORT IE:REMPLIT BN(L,I 690
700 FOR P=2 TO 0 STEP-1
710 BN ( L , P ) = I NT ( X I ( 2AP ) +0.01 ) ' Mleches haut
720 X=X-BN(L,P)*(2AP)
730 NEXT P
740 RETURN
750 ■------------------------- SOMMATION COLONNES
760 FOR C = 2 TO 0 STEP-1
770 S = 0
780 FOR Ll=l TO 4
790 S = S + BN(L1 ,C)
800 NEXT LI
810 SM(C)=S
820 NEXT C 830
840 GN = O
850 FOR C=2 TO 0 STEP-1 •
860 IF INT(SM(C)/2)<> SMCO/2 THEN GN=O:RETURN
870 NEXT C
880 GN=1
890 RETURN
900 ■-----------------------------DESSIN PARTIE
910 PRINT
920 FOR L=1 TO 4
930 PRINT L;SPC(2);
940 PRINT STRING!(JEU(L) ,"I")
950 NEXT L
960 PRINT:RETURN 97p '-------------- FIN DE partie?
980 S = O:FOR L=1 TO 4 : S = S +J EU(L): NE XT L:RETURN"##########,
    "pages 110, 111"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_096,
    r##########"10 ' APPR JEU DE MARIENBAD PAR APPRENTISSAGE
20 '
30 DIM TJ (300),6(300),J (30),LUI(30)
40 MODE 2:PAPER O:PEN 1: INK O,O:INK 1,26
50 JEU(1)=7:JEU(2)=5
60 J EU<3)=3 : JEU(4) = 1
70 ■
80 GOSUB 670 90
100 PRINTzPRINT "ESC 2 fois pour stopper ":PRINT
110 INPUT "Voulez vous commencer (0/N) ";Rt
120 JR=OsIF R$="0" OR R$="o" THEN JR=1 '
130 '
140 FP=O:CQUP=O
150 '
160 IF JR= 1 THEN GOSUB 210:JR=* 1 — JR
170 IF FP= 1 THEN GOSUB 970:GOTO 50
180 IF JR = O THEN GOSUB 350:JR=1-JR
190 IF FP=1 THEN GOSUB 970:G0T0 50
200 GOTO 160
210 ==================================== JOUEUR
220 INPUT "Quelle ligne ";L
230 INPUT "Combien ";N
240 '
250 IF N>JEU(L) OR N<=0 THEN PRINTzPRINT "Vous trichez "zPRINTzG OTO 220
260 '
270 JEU (L)=JEU (L)-N
280 GOSUB 790sG0SUB 920
290 GOSUB 670 ' Affichage partie en cours
300 GOSUB 730 ' Fin de partie?
310 IF S=1 THEN PRINTzPRINT "Vous avez gagne ":FP=1:G=-1
320 IF S=0 THEN PRINTzPRINT "J'ai gagne ":FP=1:G=1
330 RETURN
340 ================================ MACHINE
350 MCOUP=O
360 '-------------------------RECHERCHE DANS TABLE DES JEUX
370 IF TJ(l)=0 THEN N=lzGOTO 620
380 FOR LL=1 TO 4
390 FOR NN=1 TO JELKLL) ' On essaie d'en prendre NN
400 SJ=JEU(LL) ' Sauvegarde ligne modifiée
410 JEU(LL)=JEU(LL)-NNzGOSUB 790
420 FOR J=1 TO 200 ' Recherche config gagnante
430 IF TJ(J)=0 THEN 470
440 IF JCODOTJ(J) THEN 460
450 IF G(J)>MCOUP THEN MC0UP=G ( J): NJ = NNzPL = LL
460 NEXT J
470 JEU(LL)=SJ
480 NEXT NN
490 NEXT LL
500 IF MCOUP=O THEN 620
510 L=PL:N=NJ
520 '
530 PRINTzPRINT "J'en prends:";N;" Ligne :":L
540 J EU(L)=JEU(L)-N
550 GOSUB 790zGOSUB 920
560 GOSUB 670
570 GOSUB 730
580 IF S=1 THEN PRINTzPRINT "J'ai gagne "zPRINT :G=1:FP=1
590 IF S = 0 THEN PRINTzPRINT "Vous avez gagne": PRINT:G = -l:FP=1
600 RETURN
610 '-------------------PAS DE SOLUTION EN TABLE
620 L= INT(RND(1)»4)+1 ' ligne au hasard
630 IF JEU(L)<1 THEN 620
640 N=I NT(RND(1)♦JEU(L))+l
650 GOTO 530
660 '--------------------------------- DESSIN PARTIE
670 PRINT"##########,
    "pages 113, 114"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_097,
    r##########"600 FOR L = 1 TO 4
690 PRINT LjSTRING!(JEU(L)I")
700 NEXT L
710 PRINT
720 '-----------------------------FIN DE PARTIE? .
730 S = 0
740 FOR L=1 TO 4
750 S=S+JEU(L)
760 NEXT L
770 RETURN
780 '-------------------------------CONVERSION JEUO --> JEU CODE
790 FOR J = 1 TO 4 : X(J)=J EU(J): NE X T J
800 ' — tri
810 IV=O
820 FOR J=1 TO 3
830 IF X(J + 1XX(J) THEN X = X(J): X(J)=X(J + l)s X(J + l>=X: IV = 1
840 NEXT J
850 IF IV=1 THEN 810 860
870 JCOD=O
880 FOR J=1 TO 4
890 JCOD = JCOD + X(J)♦10A (J-1) ' A fléché haut
900 NEXT J
910 RETURN
920 '---------------------------------MAJ TABLE JEU ACTUEL
930 COUP = COUP+1: J(COUP)=JCOD
940 LUI(COUP)=JR
950 RETURN
960 '-------------------------------MAJ FIN DE PARTIE
970 FOR K=1 TO COUP
980 X=J(K)
990 FOR J=1 TO 300
1000 IF TJ(J)=0 THEN TJ(J)=X
1010 IF X=TJ(J) THEN 1040
1020 NEXT J 1030
1040 IF LUI (K)=l THEN G(J)=G(J)-G
1050 IF LUI (K)=0 THEN G ( J)=G(J)+G
1060 NEXT K
1070 RETURN"##########,
    "page 114"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_098,
    r##########"10 ' JEU DE LA VIE NO 1
20 '
30 DEFINT A,Z
40 MODE 2:PAPER O:PEN 1;INK O,1:INK 1,24
50 Xl=2:X2=30 ' bornes territoire
60 Yl=2:Y2=20
70 DIM VA(X2+1,Y2+1) ' ancienne generation
80 DIM VN(X2+1 , Y2+1 ) ' nouvelle generation
90 '
100 LOCATE Xl + 1,Y1-1:PRINT STR ING$(X2-X1,"-")
110 LOCATE X1 +1,Y2+1 : PR I NT STR INGI(X2-X 1 , " - " )
120 '-----------------------------SAISIE
130 LOCATE 1,24:PRINT "Entrer les positions pour chaque ligne"
140 LOCATE 1,25:PRINT "Ex: *»*»♦** (puis ENTER) ou ENTER"
150 FOR Y=Y1 TO Y2
160 LOCATE X1-1,Y
170 LINE INPUT "?“,LIG$
180 FOR X=X1 TO X2
190 IF MID$(LIG$,X,1)="♦" THEN VA(X,Y) = 1
200 NEXT X
210 NEXT Y
220 GOSUB 460
230 '----------------------- Remplissage table V N ( )
240 FOR Y = Y1 TO Y2
250 FOR X=X1 TO X2
260 VN(X,Y)=0 :TV = O
270 FOR DY = -1 TO 1 '8 points autour du point X ,Y
280 FOR DX=-1 TO 1
290 IF DX = O THEN IF DY=O THEN 310
300 IF VA(X+DX,Y+DY)=1 THEN TV=TV+1
310 NEXT DX
320 NEXT DY
330 IF TV> = 4 THEN VN(X,Y)=0: GOTO 360
340 IF TV=3 THEN VN(X,Y)=1
350 IF (TV = 2 OR TV=3) AND VA(X,Y)=1 THEN VN(X,Y)=1
360 NEXT X
370 NEXT Y
380 '---------------transfert VN ( ) —>VA()
390 GN=GN+1
400 FOR Y = Y1 TO Y2
410 FOR X = X1 TO X2: VA(X,Y)=VN(X,Y): NEXT X
420 NEXT Y
430 GOSUB 460
440 GOTO 240
450 '------------------------------- AFFICHAGE
460 CLS
470 FOR Y=Y1 TO Y2
480 FOR X = X1 TO X2
490 IF VA(X,Y)=l THEN LOCATE X,Y:PRINT
500 NEXT X
510 NEXT Y
520 LOCATE 30,l:PRINT "GENERATION ";GN
530 RETURN"##########,
    "page 118"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_099,
    r##########"10 ' JEU DE LA VIE NO 3
20 "
30 DEF INT A,Z
40 MODE 2
50 Xl=2:X2=50 ' bornes
60 Y1=2:Y2=22
70 DIM VA ( X2 +1 ,Y2+1) ' ancienne generation
80 DIM VN(X2+1,Y2+1) ' nouvellle generation
90 DIM XX(IOOO),YY( 1000) ,XI(1000),Y1 ( 1000)
100 '-------------Saisie
110 LOCATE Xl + 1,Y1—IsPRINT STR I NG$(X2-X1,"-")
120 LOCATE Xl + 1,Y2+1 : PRINT STR I NG$(X2-X1,"-")
130 LOCATE 1,24:PRINT "Entrer les positions pour chaque ligne"
140 LOCATE 1,25:PRINT "Ex: ♦**♦**♦* (puis ENTER) ou ENTER"
150 N = 0
160 FOR Y = Y1 TO Y2
170 LOCATE X1-1,Y
180 LINE INPUT "?",LIG$
190 FOR X=X1 TO X2
200 IF MID$(LIG$,X,1)="*" THEN VA ( X,Y)=1 :VN(X , Y)=3:NC = NC+1 : XX(NC)= X:YY(NC)*Y
210 NEXT X
220 NEXT Y
230 CLS
240 GOSUB 400
250 '------------------------------------
260 FOR N=1 TO NC
270 X=XX(N):Y=YY(N)
280 FOR DY=-1 TO 1
290 IF Y+DYXYl OR Y+DY>Y2 THEN 340
300 FOR DX=-1 TO 1
310 IF X+DX<X1 OR X+DX>X2 THEN 330
320 VN(X+DX,Y+DY)=Vn(X+DX,Y+DY)+1
330 NEXT DX
340 NEXT DY
350 NEXT N
360 GN=GN+1
370 GOSUB 390
380 GOTO 260
390 '--------------------------------AFFICHAGE
400 N1=O 410
420 FOR N=1 TO NC
430 XX=XX(N):YY=YY(N)
440 FOR DY=-1 TO 1
450 FOR DX = -1 TO 1
460 X = X X + DX:Y = YY + DY
470 IF VN(X,Y)=0 THEN 540
480 IF VN(X,Y)<=2 THEN LOCATE X,Y:PRINT CHR$(32): VA(X , Y)=0: GOTO 540
490 IF VN(X,Y)=3 THEN LOCATE X,Y:PRINT ": VA(X,Y)=1 : GOTO 530
500 IF VN(X,Y)>=5 THEN LOCATE X,Y:PRINT CHR$(32): VA(X,Y)=0: GOTO 540
510 IF VA(X,Y)=1 AND VN(X,Y)=4 THEN 530
520 GOTO 540
530 N1=N1 + 1 : X 1 (NI)=X:Y1(NI)=Y
540 VN(X,Y)=0
550 NEXT DX
560 NEXT DY
570 NEXT N
580 NC=N1
590 FOR N=1 TO NC:YY(N)=Y1(N): XX(N)=X1(N): NEXT N
600 LOCATE 30,l:PRINT "GENERATION ";GN
610 RETURN OBSTACLE ____________________________________________ Un point se déplace à travers des obstacles. Vous devez adapter la direction à l’aide des flèches pour éviter les obstacles."##########,
    "pages 119, 120"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_100,
    r##########"10 ' OBSTACLE
20 '
30 MODE 1
40 INK 0,0:INK 1,26
50 INPUT "NIVEAU (1,2,3 (9 pour fin)) ";NV
60 IF NV = 9 THEN END
70 CLS
80 LOCATE 1,24:PRINT "Utilisez les fléchés"
90 '-----------CADRE
100 XD=20:XF=500
110 YD=40:YF=350
120 PLOT XD,YD,1:DRAW XF,YD
130 DRAW XF,YF:DRAW XD,YF:DRAW XD,YD
140 '
150 -----------OBSTACLES
160 FOR P=1 TO (NV+l)*20
170 X=RND*(XF-XD)+XD
180 Y = RND* (YF-YD)+YD
190 PLOT X,Y,1
200 NEXT P 2io '------------------------------- COORDONNEEES DEPART
220 X=200+RND*10:Y = 200 + RND* 10: IF TEST(X,Y)<>0 THEN 220
230 '----------------------- ATTENTE DEPART
240 C$=INKEY$
250 IF LEN(C$)<>0 THEN GOSUB 300:G0T0 370
260 PLOT X,Y,1
270 PLOT X,Y,0
280 GOTO 240
290 '-----------Actualisation X,Y
300 C=ASC(C$)
310 IF C = 242 THEN DX = -2: DY = O
320 IF C = 243 THEN DX = 2 : DY = O
330 IF C = 241 THEN DX = O: DY = -2
340 IF C = 240 THEN D X = 0 : DY = 2
350 RETURN
360 '-----------
370 P=0
380 '
390 C$=INKEY$:IF LEN(C$)<>0 THEN GOSUB 300
400 P=P+1
410 X=X+DX:Y=Y+DY
420 IF TEST(X,Y)<>0 THEN 470
430 PLOT X,Y,1
440 FOR TP=1 TO 30-NV*3:NEXT TP
450 GOTO 390
460 '---------
470 PRINT CHRK7) ;
480 LOCATE 1,2:PRINT P;"POINTS"
490 FOR TP=1 TO 2000:NEXT TP
500 GOTO 30"##########,
    "page 121"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_101,
    r##########"10 ----------------METEORITES
20 MODE 1:PAPER O:PEN 1
30 INK 0,26: INK 1,0
40 PRINT "Fléchés < - et -> / ESC 2 -fois pour -fin"
50 FOR TP=1 T0 2000:NEXT TP
60 CLS:F=O 70XV=15:YV=3 'position véhiculé
80 C=145:SYMB0L AFTER C
90 SYMBOL C,8,8,28,127,28,34,65,0 ' dessin etoile
100 FOR L=1 TO 10000
110 AX = XV:AY= YV ' ancienne position
120 IF INKEY(1)=O THEN IF XV<33 THEN XV=XV+1 ' droite
130 IF INKEY(8)=0 THEN IF XV>2 THEN XV=XV-1 ' gauche
140 X = RND(1)*32 + 1 : LOCATE 1,25
150 PRINT TAB(X) CHRI(C) TABIX + 2) CHRS (C) ' 2 étoiles
160 PRINT ' scrolling
170 IF TEST((XV-1)♦16 + 8,399-(YV)» 16 + 8)<>0 THEN F=l: PRINT CHR$(7);:FOR TP=1 TO lOOOsNEXT TP ' collision?
180 '
190 LOCATE A X , A Y-1 : PR I NT CHR$ (32) ' e-ffacement véhiculé
200 LOCATE XV,YV:PRINT CHR$ ( 231 ) ' nouvelle position
210 IF F=1 THEN LOCATE 1,23:PRINT "Sc ore:";L: GOTO 240
220 NEXT L
230 '
240 FOR.TP=1=1 TO 3000:NEXT TP:60T0 60"##########,
    "page 122"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_102,
    r##########"59 POINTS
122 I BASIC AMSTRAD l0 ■------------------- CASSE BRIQUES"##########,
    "pages 123, 124"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_103,
    r##########"20 MODE 1:PAPER O:PEN 1
30 INK 0,26:INK 1,0:INK 2,6
40 INPUT "Niveau (1,2,3) (9 pour fin) " ; NV
50 IF NV = 9 THEN END
60 '--------------------- dessin terrain
70 MODE 1
80 XD=18+NV*2 ' bornes
90 XG=2:YB=2:YH=21
100 LOCATE 1,23:PRINT "Fléchés pour raquette"
110 FOR X = XG TO XD:LOCATE X,YB:PRINT CHRI(143): NEXT X
120 FOR Y=YB TO YH-1
130 LOCATE XG,Y:PRINT CHRS(143):LOCATE XD,Y:PRINT CHRK143)
140 NEXT Y
150 PEN 2
160 FOR Y=YB+1 TO YB+5
170 FOR X=XG+1 TO XD-1:LOCATE X,Y:PRINT CHR$(143): NEXT X
180 NEXT Y
190 PEN 1
200 '-­
210 SC=O ' score
220 RQ$=CHR$(32)+CHR$(143)+CHR$(143)+CHR$(143)+CHR$(32) 230DX=l:DY=-l ' déplacement
240 X L = 5 +1N T ( R N D ( 1 ) * 5 ) : Y L = 10 ' balle
250 XR=10:YR=YH ' raquette
260 LOCATE X R-1,YR : PR I NT RQI
270 '-------------------------------------------------- déplacement balle
280 LOCATE XL,YL:PRINT CHR*(32) ' effacement balle
290 XL=XL+DX:YL=YL+DY ' nouvelle position
300 IF YL=YB+6 THEN YL=YL+ I NT(RND*2)*DY
310 1
320 IF TEST((XL-1)*16+1,399-(YL-1)*16)< >0 THEN SC=SC+1
330 LOCATE XL,YL:PRINT CHRK231)
340 LOCATE XL,YL-1
350 IF YL>YB+1 THEN IF TEST((XL-1)* 16+1,399-(YL-2)* 16)<>0 THEN PRINT CHR$(32):SC = SC+ 1
360 IF XL>=XD—1 THEN DX=-DX ' rebonds
370 IF YL<=YB+1 THEN DY=-DY
380 IF XL<=XG+1 THEN DX=-DX
390 '
400 IF XL>XR-2 AND XL<XR+4 AND YL>YR-2 THEN DY=-DY
410 '
420 IF YL>=YR OR (XD-XG- 1)*5 = SC THEN 490
430 '-------------------------------------------- deplacement raquette
440 IF INKEY(1)=O THEN IF XRCXD-3 THEN XR=XR+1
450 IF INKEY(8)=0 THEN IF XR>XG+1 THEN XR=XR-1
460 LOCATE XR-1,YR : PR I NT RQ$
470 GOTO 280
480 '
490 LOCATE 30,23:PRINT SC;"POINTS"
500 FOR TP=1 TO 4000:NEXT TP
510 GOTO 20"##########,
    "page 124"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_104,
    r##########"10 ' GLOUTONS
20 '
30 MODE IzPAPER 0:PEN 1:INK 0,1:INK 1,24
40 LOCATE 1,25:PRINT "UTILISER LES FLECHES / FIN:ESC 2 FOIS"
50 SYMBOL AFTER 145
60 SYMBOL 145,120,188,254,63,63,254,252,120 ' glouton pos 1
70 SYMBOL 146,30,61,127,252,252,127,63,30 ' glouton pos 2
80 M=0 ' gloutons morts
90 XC=18:YC=12 ' vos coordonnées
100 LOCATE XC, YCzPRINT "0"
110 '---------------------
120 FOR J=1 TO 3
130 XM(J> = I NT(RND(1)*38) +1 : IF ABS(XM(J)-XC)<B THEN 130
140 YM(J)=I NT(RND(1)*23)+1 : IF ABS(YM(J)-YC)<8 THEN 140
150 LOCATE XM(J) ,YM(J): IF XM(J)>XC THEN PRINT CHR$(145) ELSE PRI NT CHR$(146)
160 LOCATE RND ( 1 )*7 + XC,RND(1)«7 + YC: PRINT CHR$(143)
170 NEXT J
180 FOR TP=1 TO lOOOzNEXT TP
190 '-------------------------
200 FOR J=1 TO 3
210 LOCATE XC,YCzPRINT CHR$(32) ' effacement
220 IF INKEY(1)=O THEN IF XC<38 THEN XC=XC+1
230 IF INKEY(8>=0 THEN IF XC>1 THEN XC=XC-1
240 IF INKEY (0>=0 THEN IF YO1 THEN YC = YC-1
250 IF INKEY(2)=0 THEN IF YCC24 THEN YC=YC+1
260 LOCATE XC,YCzPRINT "0" ' nouvelle position
270 '
280 IF XM(J)=0 THEN 400
290 LOCATE XM(J),YM(J): PR I NT CHR$(32) ' effacement glouton
300 XM(J)=XM(J)-(SGN(XM(J)-XC)) ' maj position glouton
310 YM(J)=YM(J)-(SGN(YM(J)-YC))
320 ' --test piege
330 IF TEST( (XM(J)-1 )* 16 +1,399-(YM(J)-1)* 16)=0 THEN 360
340 XM(J)=O:YM(J)=0:M = M+1 : GOTO 390
350 test devore
360 X 1 =XM(J) : Y1=YM(J)
370 LOCATE XI , Y1 : IF XI>XC THEN PRINT CHR$(145) ELSE PRINT CHRK146)
380 IF X1 = XC AND Y1=YC THEN LOCATE 1,23:PRINT " PERDU GOTO 430
390 IF M = 3 THEN LOCATE 1,23:PRINT "GAGNE":GOTO 430
400 NEXT J
410 GOTO 200
420 '--­
430 FOR TP=1 TO 3000:NEXT TP
440 GOTO 30"##########,
    "page 126"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_105,
    r##########"10 ' BIORYTHME
20 '
30 MODE 2:PAPER O:PEN 1:1NK 0,26:INK 1,0
40 DIM A(12),JM(12)
50 '
60 DATA 31,28.5,31,30,31,30,31,31,30,31,30,31
70 FOR M=1 TO 12
80 A(M)=A(M-1):READ X : A(M)= A(M)+X : JM(M)=X
90 NEXT M
100 CP=23:CE=28:CI=33 ' Cycles physique/Emot./Intel.
110 '
120 PRINT "NAISSANCE:":PRINT
130 INPUT " Jour,Mois,Annee ( ex : 5,4,1984) ";JN,MN,AN
140 IF AN< 1900 OR ANT1999 THEN 130
150 J=JN:M=MN:A=AN:GOSUB 230:XV=JV
160 '
170 PRINT:PRINT " BIORYTHME11 : PR I NT
180 INPUT "Mois,An (ex:7,1985) ";MB,AB
190 IF AB< 1900 OR AB.M999 THEN 180
200 J=1:M=MB:A=AB:GOSUB 230:NJOUR=JV-XV
210 GOTO 270
220 '---------------calendrier
230 N=365.25*(A-1901)+A(M)+J
240 JV=INT(N)
250 RETURN
260 '---------------------Af-fichage courbes
270 MODE 1: INK 0,26:INK 1,0: INK 2,20:INK 3,6
280 TAGOFF: LOCATE 1,2:PEN 1:PRINT "Noir: physique"
290 LOCATE 1,3:PEN 2:PRINT "Bleu: émotionnel"
300 LOCATE 1,4:PEN 3:PRINT "Rouge:intel1ectuel":PEN 1
310 PLOT 10,200:DRAWR 10+16*31,0,1 ' Axe
320 C = CP:CL=1 :GOSUB 420
330 C=CE:CL=2:GOSUB 420
340 C=CI:CL=3:GOSUB 420
350 FOR J=1 TO JM(MB)+1 STEP 3
360 MOVE 10+(J-2)*16,190 : TAG : PR I NT J;
370 NEXT J
380 TAGOFF: LOCATE 1,22:PRINT "Jours vécusNJOUR
390 LOCATE 1,23:PRINT " Naiss:"; JN; MN ; AN ;"Bior :"; MB ; AB
400 END
410 '-----------trace sinusoide
420 PLOT 10,SIN(NJ0UR*2*PI/C)♦100 + 200
430 FOR D=1 TO JM(MB)
440 P1 = 100*S IN ( (D + NJOUR)*2*P I/C)
450 X = 1 0 + D*16 : DRAW X,200+Pl,CL
460 NEXT D
470 RETURN"##########,
    "page 128"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_106,
    r##########"10 ' COMPLETER UNE PHRASE
20 '
30 MODE 2:PAPER 0:PEN 1 s INK 0,0:INK 1,26
40 LOCATE 1,20:PRINT "COMPLETER LA PHRASE"
50 DATA IL ETAIT UNE *»»* DANS L'OUEST,FOIS
60 DATA LUNDI MARDI *♦**♦♦** JEUDI,MERCREDI
70 DATA *
80 XA=1:YA=10 ' Affichage
90 '
100 READ LIGt!IF LIG$=“*" THEN END
110 READ MOT$
120 '
130 P=INSTR(LIG$,"* " ) ' Recherche *
140 '
150 LOCATE XA,YA:PRINT SPC(40)
160 LOCATE XAjYAsPRINT LIG$
170 PL = 1
180 '
190 LOCATE XA+P-1,YA:PRINT CHR$(143) ' curseur
200 C$=INKEY$:IF LEN(C$)<>0 THEN 230
210 GOTO 190
220 ' —
230 C$=UPPER$(C$)
240 IF C$<>MID$(MOT$,PL, 1 ) THEN PRINT CHR$(7):GOTO 190
250 LOCATE X A + P-1,YA : PRINT C$
260 P=P+1
270 PL=PL+1:IF PLXEN(MOTI) THEN 100
280 GOTO 190"##########,
    "page 129"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_107,
    r##########"10 ' DICTEE 20
30 MODE 1
40 PAPER 0:PEN 1 : INK 1 ,0:INK 0,26 50
60 DATA "QU'EST CE QUE SIGHFIE APPRIVOISER" ,N
70 DATA "DIT LE P*TIT PRINCE" ,E
80 DATA "C'EST UNE CHOSE TROP OUBLIEE,"
90 DATA "D*T LE RENARD" ,1
100 DATA "CA SI*NIFIE CREER DES LIENS." ,6
110 DATA *
120 ---------------------------------- LECTURE DATAS DANS TABLE TX.TS
130 DIM TXTS(100),CS(100)
140 FOR L=1 TO 100
150 READ TXTS(L): IF TXT$(L)="*" THEN NL = L-l:G0T0 190
160 READ C$(L)
170 NEXT L
180 '------------------------------------ AFFICHAGE TEXTE
190 CLS
200 LOCATE l,20:PRINT "TU DOIS COMPLETER LES MOTS "
210 FOR L=1 T0 NL
220 LOCATE 1,L:PRINT TXTI(L)
230 NEXT L
240 •-------------------------
250 FOR L=1 T0 NL
260 P=INSTR(TXTS(L) , " *") ' position lettre manquante
270 IF P=O THEN 320
280 LOCATE P,L:PRINT CHRK143)
290 C$=INKEY$:IF LEN(CI)=O THEN 290
300 IF UPPERS(CS)<>UPPER$ (CS (L)) THEN PRINT CHRS ( 7); : GOTO 290
310 LOCATE P,L:PRINT 2 S ____________________________
320 NEXT L
330 END QU EST CE QUE SIGNIFIE APPRIVOISER DIT LE P»TIT PRINCE C'EST UNE CHOSE TROP OUBLIEE, D*T LE RENARD CA SHNIFIE CREER DES LIENS."##########,
    "page 130"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_108,
    r##########"80 NM = 4 ' nombre de mots
81 X = I NT(RND(1)*NM)
82 RESTOREzFOR J=1 TO X:READ MOTS:NEXT J
83 READ MOTS"##########,
    "page 131"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_109,
    r##########"10 ' MISE EN ORDRE D'UN MOT
20 '
30 MODE IsPAPER 0:PEN 1
40 DIM T(12) ' témoin
50 DATA AMSTRAD,ECOLE,MAISON,BASIC
60 DATA ♦
70 '
80 READ MOTS
90 IF MOTS®"»" THEN END
100 L=LEN(MOTS)
110 CLS
120 FOR P=1 TO 12: T(P)=0:NEXT P
130 FOR N=1 TO L
140 P=INT(RND(1)*L)+ 1 s IF T(P)=1 THEN 140
150 PRINT MI DS(MOTS,P , 1 );
160 T(P)=1
170 NEXT N
180 '
190 PRINT:PRINT
200 INPUT "MOT";MS
210 IF UPPERS (MS)=UPPERS(MOTS) THEN PRINT "BRAVO"
220 IF UPPERS(MS)<>UPPERS(MOTS) THEN PRINT " NON , C ' EST MOTS
230 FOR TP=1 TO 2000:NEXT TP
240 GOTO 80 DTRSAAM MOT? AMSTRAD BRAVO"##########,
    "page 131"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_110,
    r##########"10 ’ REMISE EN ORDRE D'UNE PHRASE
20 '
30 MODE 2
40 CE=1:CF=O ' écriture et fond
50 INK 0,26:INK 1,O:PAPER 0:PEN 1
60 '
70 READ L$:IF L$="*" THEN END ' lecture d'une ligne de DATA
80 LIG$ = L!
90 GOSUB 230
100 '-------------------------Affichage dans le desordre
110 CLS
120 FOR F=1 TO N
130 P = I NT(RND(1)*N)+1 ' position au hasard
140 IF LEN(M$(P))=0 THEN 130
150 LOCATE 1, F : P RIN T M$(P):M$(P)=........ pas d'espace entre ""
160 NEXT F
170 '
180 LOCATE 1,13:INPUT "Frappez la phrase en ordre ";PHR$
190 IF UPPER!(PHR!)=UPPER!(L!) THEN PRINT "OK " ELSE PRINT "Non c ' e s t : " ; L !
200 FOR TP=1 TO 2000:NEXT TP
210 GOTO 70
220 '--------------------------- DECOUPAGE PHRASE
230 N=O:Y!="" ' pas d'espace entre ""
240 P=INSTR(LI G!,CHR!(32)) ' recherche espace (chr!(32))
250 IF P = 0 THEN N = N +1 :M!(N)=LIG!: RETURN
260 N = N+1 :M!(N)=LEFT!(LI G!,P-1)
270 L I G! = R IGHT!(LI G!,LEN(LI G!)-P)
280 GOTO 240
290 '-------
300 DATA LE PETIT CHAT RONRONNE
310 DATA JE M'APPELLE AMSTRAD
320 DATA * PETIT CHAT LE RONRONNE Frappez la phrase en ordre ? LE PETIT CHAT RONRONNE OK"##########,
    "page 132"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_111,
    r##########"10 ‘ LISTE A COMPLETER
20 '
30 MODE 2.-INK 0,1: INK 1,24:PAPER O:PEN 1
40 DIM ANS ( 100) ,C$1100)
50 '
60 DATA LE CHIEN,ABOIE
70 DATA LE CHAT,MIAULE
80 DATA LE MOUTON,BELE
90 DATA LE CHEVAL,HENNI T
100 '
110 DATA
120 '-----------------------lecture DATAS dans tables
130 FOR J = 1 TO 100
140 READ AN$(J),C$(J)
150 IF AN$(J)="*" THEN NM=J-1:GOTO 190
160 NEXT J
170 STOP
180 '-----------------------Affichage
190 CLS
200 LOCATE 10,22:PRINT "Tu dois completer la liste"
210 LOCATE 10,23:PRINT "ex: LE CHIEN ABOIE"
220 FOR L=1 TO NM
230 LOCATE 2,L:PRINT AN$(L)
240 NEXT L
250 '
260 FOR L=1 TO NM
270 LOCATE 13,L:INPUT LIG$
280 IF UPPER!(C$(L))<>UPPER$(LIG$) THEN PRINT CHR$(7):G0T0 270
290 NEXT L LE CHIEN ? ABOIE LE CHAT ? MIAULE LE MOUTON ? BELE LE CHEVAL ?"##########,
    "page 133"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_112,
    r##########"10 ' QUESTIONNAIRE
20 '
30 MODE 2:PAPER O:PEN 1 40
50 ' Question Reponses possibles 60
70 DATA "Donnez un synonyme de DELAI1 "SURSIS REPIT PROLONGATION II
80 DATA "Capitale de 1'ESPAGNE", "MADRID"
90 DATA "Capitale de la FRANCE ", "PARIS"
100 DATA * 110 120
130 READ QI: IF Q*="*" THEN END
140 READ RP*
150 PRINT
160 PRINT Q* ' affichage questi on
170 INPUT "Votre réponse ";RI
180 P=INSTR(UPPERI(RP*), UPPERI(RI))
190 IF POO THEN PRINT: PRINT "OK" ELSE PRINT RP*
200 GOTO 130 Donnez un synonyme de DELAI Votre réponse ? REPIT 0K"##########,
    "page 134"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_113,
    r##########"10 ' QUID
20 '
30 MODE 2: PAPER O:PEN l.-INK 0,0: INK 1,26
40 '
50 DATA TOUR DE FRANCE 1979 HINAULT
60 DATA TOUR D'ITALIE 1979 SARONNI
70 DATA CHAMPIONNAT DU MONDE 1979 RASS
80 DATA TOUR DE FRANCE 1980 HINAULT
90 DATA TOUR D'ITALIE 1980 HINAULT
100 DATA CHAMPIONNAT DU MONDE 1980 HINAULT
110 DATA TOUR DE FRANCE 1981 HINAULT
120 DATA TOUR D'ITALIE 1981 BATAGLIN
130 '
140 DATA ♦
150 '
160 PRINTîINPUT "Mot cle (ou FIN)";M!
170 M! = UPPER!(M!) : IF M!="FIN" THEN END
180 '
190 RESTORE
200 READ LIG!
210 IF LIG!="»" THEN 160
220 P=INSTR(UPPER! (LI G!) .UPPER!(Ml))
230 IF P<>0 THEN PRINT LIG!
240 GOTO 200 Mot cle ? 1980 TOUR DE FRANCE 1980 HINAULT TOUR D'ITALIE 1980 HINAULT CHAMPIONNAT DU MONDE 1980 HINAULT Mot cle ? HINAULT TOUR DE FRANCE 1979 HINAULT TOUR DE FRANCE 1980 HINAULT TOUR D'ITALIE 1980 HINAULT CHAMPIONNAT DU MONDE 1980 HINAULT TOUR DE FRANCE 1981 HINAULT Mot cle ? FRANCE TOUR DE FRANCE 1979 HINAULT TOUR DE FRANCE 1980 HINAULT TOUR DE FRANCE 1981 HINAULT"##########,
    "page 135"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_114,
    r##########"10 ' ADDITION EN BASE B
20 '
30 MODE 1:PAPER 0:PEN 1 : INK 0,26: INK 1,0
40 B=10 ' base
50 NCH = 4 ' 4 chiffres
60 XL=10:YL=10 ' AFFICHAGE
70 '-------------------Choix au hasard
80 N1 $- " ": N 2 $-"" ' pas d'espace entre ""
90 FOR J=1 T0 NCH
100 X = I NT(RND(1)*B)
110 Y= I NT(RND(1)«B)
120 N1$ = N1$ + M I D$(STR$(X),2,1)
130 N2$ = N2$ + MID$(STR$ (Y) ,2, 1 )
140 NEXT J
150 '
160 CLS
170 LOCATE 1, 3 :PRINT "BASE: ";B
180 LOCATE XL,YL-3:PRINT Nl<
190 LOCATE XL-1 ,YL-2:PRINT "+";N2$
200 '
210 LOCATE XL —2,YL: PRINT " = "
220 '---------------------------------------- Saisie résultats
230 P=LEN(N1Î) 240RET=0 'retenue
250 '
260 IF P=0 THEN TR=RET:G0T0 290
270 TR = VAL(M I D$(N1 $,P ,1))+VAL(MI D$ ( N2$ , P , 1 ) )+RET
280 '
290 RET=INT(TR/B)
300 RS=TR-RET*B
310 '
320 LOCATE 1,18:PRINT "TOTAL POUR LA COLONNEISANS RETENUE)"
330 LOCATE XL + P- 1 , YL : PR I NT CHRK143)
340 LOCATE XL+P-1,YL
350 R$=INKEY$: I F LEN(R$)=O THEN 350
360 IF RSOVAL(R$) THEN PRINT CHR$(7):GOTO 330
370 PRINT R$
380 '--------------------------------------Retenue de rang N+l
390 IF RET=O THEN 480
400 '
410 LOCATE 1 , ISsPRINT "ENTRE LA RETENUE POUR LA COLONNE" SPC(4)
420 LOCATE XL + P-2,YL-4: PR I NT CHR$ ( 1 43)
430 LOCATE XL+P-2.YL-4
440 R$=INKEY$:IF LEN(R$)=O THEN 440
450 IF VAL(R$)ORET THEN PRINT CHRK7) :GOTO 420
460 PRINT R*
470 '
480 IF P=0 THEN 530
490 IF P=1 AND RET=O THEN 530
500 P=P-1
510 GOTO 260
520 '
530 FOR TP=1 TO 3000:NEXT TP
540 GOTO 80 JEU DU PENDU _________________________________________ Vous devez retrouver un mot en proposant des lettres. Vous avez droit à sept erreurs. BCDEFGH . JKL . . OPÛR . TUUJ4 . VZ MAIS .N Quelle lettre"##########,
    "pages 136, 137"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_115,
    r##########"125 X = I NT(RND(1)*NM)
126 REST0RE:F0R J = 1 TO X:READ MOT$;NEXT J"##########,
    "page 138"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_116,
    r##########"10 ' JEU DU PENDU
20 '
30 DATA MAISON,SAPIN, VELO,PATIN
40 DATA AMSTRAD
50 DATA *
60 XM=12:YM=10 ' affichage mot
70 XA=2:YA=2 ' affichage alphabet 80
90 MODE 1:PAPER O:PEN 1 : I NK 0,26:INK 1,0
100 '-----affichage alphabet
110 FOR J = 1 TO 26:L0CATE XA +J , YA : PR I NT CHR$(64 + J): NEXT J
120 '
130 READ MOTS:M0T$ = UPPER$(MOT*): IF MOT$="*" THEN END
140 L=LEN(M0T$)
150 'BPL:bien places/PD:coups perdus /LU$:lettres déjà utilisées
160 BPL=0:PD=0:LU$="" ' pas d'espace entre ""
170 LOCATE XM,YM:PRINT STR I NG$ ( L , ". " )
180 '
190 LOCATE 2,23: INPUT "Quelle lettre ";L$:L$=UPPER$(L$)
200 IF LEN(L$)=O THEN 190
210 P=I NSTR(LU* , L*): IF P)0 THEN GOSUB 380;G0T0 310
220 '-------------------------------------------- bonne lettre?
230 IF ASC(L$)<65 THEN 190
240 B=0
250 FOR P=1 TO L
260 IF L$ = MID$ ( MOTS , P , 1 ) THEN B = 1 :LU$ = LU$ + L$:BPL = BPL +1 :GOSUB 340:G0SUB 360
270 NEXT P
280 IF B = 0 THEN GOSUB 380:GOSUB 360
290 IF BPL = L THEN LOCATE 12, 15:PRINT "BRAVO":FOR TP=1 TO 2000:NEX T TP:GOTO 90
300 '
310 IF PD>=8 THEN LOCATE 12,15:PRINT "PERDU .. LE MOT ETAIT:";MOT $:FOR TP = 1 TO 2000:NEXT TP:GOTO 90
320 GOTO 190
330 '------------------------------- affichage mot
340 LOCATE XM + P-1,YM : PR I NT L$:RETURN
350 '--------------------------- mise a jour alphabet
360 LOCATE XA+ASC(L$)-64,YA:PRINT ".":RETURN
370 '-------------------coup perdu
380 PD = PD+1: PRINT CHRK7)
390 ON PD GOSUB 410,420,430,4 40,450,460,470,490 : RETURN
400 '
410 PLOT 20,130,1:DRAW 40,130,1 : RETURN
420 PLOT 30,130,1 : DRAW 30,290,1 : RETURN
430 PLOT 30,285,1 : DRAW 75,285,1 : RETURN
440 PLOT 30,265:DRAW 50,285 : RETURN
450 PLOT 75,285:DRAW 75,245 : RETURN
460 PLOT 65,245:DRAWR 20,0:DRAWR 0,-20:DRAWR -20,0:DRAWR 0,20:RET URN
470 PLOT 75,225:DRAW 75,200
480 PLOT 60,210:DRAW 90,210 : RETURN
490 PLOT 75,200:DRAW 60,170:PL0T 75,200:DRAW 90,1 70 : RETURN DICTIONNAIRE FRANÇAIS/ANGLAIS Un dictionnaire français/anglais est stocké dans deux tables FR$() et ANG$(). Les tables sont sauvegardées sur cassette ou disquette. FR$() ANG$() LIVRE —> BOOK CHAT —> CAT CHIEN —> DOG MAISON —> HOUSE Un mode “recherche” donne la traduction d’un mot cherché. Vous pouvez placer dans chaque ligne des tables la liste des synonymes d’un mot. Un mode “interrogation” propose un mot au hasard et demande la traduction. Ce mode pourrait être amélioré en enregistrant dans une table les mots mal connus de l’élève afin de les proposer en priorité. Mode ? L LISTE DES MOTS LIVRE BOOK CHIEN DOG TRAVAIL WORK CHAT CAT BUREAU DESK JAMBE LEG BRAS ARM CALCULATEUR COMPUTER TETE HEAD RECHERCHE SEEK/SEARCH/RESEARCH ARRETER TO STOP/TO TERMINATE CHAMBRE ROOM APPUYER SUR ENTER ? «Break*"##########,
    "pages 138, 139"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_2_programmes_et_fichiers_1986__acme__listing_117,
    r##########"10 ' DICTIONNAIRE ANGLAI S/FRANCAIS
20 '
30 MODE 2:PAPER 0:PEN 1: INK O,1:INK 1,24
40 '
50 DIM FR$ ( 1000),ANG$(1000)
60 NFICH=0
70 '
80 INPUT "Nouveau fichier (0/N) ";R$
90 IF R$<>"0" AND R$<>"o" THEN GOSUB 390
100 '
110 CLS:PRINT "Modes: ":PRINT
120 PRINT "C: Creation"
130 PRINT "R: Recherche"
140 PRINT "L: Liste "
150 PRINT "INT: Interrogation""
160 PRINT "FIN: Sauvegarde "
170 PRINT:INPUT "Mode ";M$
180 M$=UPPER$(M$)
190 IF M$="C" THEN GOSUB 260
200 IF M$="FIN" THEN GOSUB 480:END
210 IF M$="L" THEN GOSUB 560
220 IF M$="R" THEN GOSUB 630
230 IF M$="INT" THEN GOSUB 720
240 GOTO 110
250 ' ==================================== CREATION
260 PRINT:INPUT "Mot FRANÇAIS ( ENTER pour fin) ";X$
270 IF LEN(X$)=O THEN RETURN ' fin de mode?
280 FOR RANG=1 TO NFICH
290 IF X$=FR$(RANG) THEN PRINT "Existe deja":GOTO 260
300 NEXT RANG
310 NF I CH = NF ICH+1
320 RANG=NFICH
330 FR$(RANG)=X$
340 '
350 INPUT "Mot ANGLAIS ";Y$
360 ANG$(RANG)=Y$
370 GOTO 260
380 ============================ LECTURE FICHIER
390 OPENIN "DICO"
400 INPUT #9,NFICH
410 FOR J=1 TO NFICH
420 INPUT #9,FR$ ( J) ,ANG$(J)
430 NEXT J
440 PRINT:PRINT NF I CHMOTS" : PR I NT
450 FOR TP=1 TO 2000:NEXT TP
460 RETURN
470 ======================== SAUVEGARDE FICHIER
480 OPENOUT "DICO"
490 PRINT #9,NFICH
500 FOR J = 1 TO NFICH
510 PRINT #9,FRI (J)
520 PRINT #9,ANGI(J)
530 NEXT J
540 RETURN
550 ======================== LISTE
560 PRINTzPRINT "LISTE DES MOTS":PRINT
570 FOR J=1 TO NFICH
580 PRINT FRI(J) ,ANGÎ ( J)
590 NEXT J
600 PRINT:INPUT "APPUYER SUR ENTER ";XI
610 RETURN
620 ================================= RECHERCHE
630 PRINT:INPUT "QUEL MOT (ou ENTER) ";XI
640 IF LEN(XI)=O THEN RETURN 650
660 FOR J = 1 TO NFICH
670 P=INSTR(UPPERI(FRI (J)) ,UPPERI(XI))
680 IF POO THEN PRINT: PRINT FRI ( J) , ANSI(J)
690 NEXT J
700 GOTO 630
710 ================= INTERROGATION ALEATOIRE
720 X = I NT(RND(1)*NFI CH)+ 1
730 MOTI=FRI(X)
740 PRINT
750 PRINT MOTI;SPC(1);:INPUT "Traduction (ENTER pour fin) ";XI
760 IF LEN(XI)=0 THEN RETURN
770 P= INSTR(UPPERI(ANGI (X)),UPPERI(XI))
780 IF POO THEN PRINT:PRINT "OK" ELSE PRINT:PRINT "Non c'est: ";ANGI(X)
790 GOTO 720 800 810
820 Les recherches peuvent s'effectuer en majuscules ou
830 mi nuscules.
840 Pour éviter d'entrer en minuscules des mots
850 déjà dans le fichier en majuseu1 es,faire: 860
870 290 IF UPPERI(XI)=UPPERI(FRI(RANG)) THEN PRINT "Existe deja":G0T0 260 880
890 On pourra egalement 'normaliser' les mots
900 en majuscules avant de les placer dans le fichier:
910 330 FRI(RANG)=UPPERI(XI)"##########,
    "pages 140, 141"
);
