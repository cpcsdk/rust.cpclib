// Tests for BASIC listings from BASIC_AMSTRAD_CPC464-664-6128_1-Methodes_pratiques(1985)(acme).pdf
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
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_001,
    r##########"10 PRINT "BONJOUR"
15 PRINT "MONSIEUR"
20 GOTO 10 SUPPRESSION D’UNE LIGNE _____________________________________ Il suffit de frapper le numéro de ligne suivi de|ENTER.|"##########,
    "page 13"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_002,
    r##########"15 7 La ligne 15 List—s a disparu 10 PRINT "BONJOUR"
20 GOTO 10 □ Chaque ligne ne doit pas excéder 255 caractères. □ Tout ce qui suit REM ou ’ sur une ligne de programme est du commentaire. □ Plusieurs instructions sur une même ligne doivent être séparées par"##########,
    "page 13"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_003,
    r##########"10 PRINT "BONJOUR" Frapper EDIT 10 La ligne 10 est alors affichée avec le curseur au début de la ligne.
10 PRINT "BONJOUR" □ Positionner le curseur après le caractère à supprimer avec la flèche —> puis appuyer sur la touche DEL. Le caractère “M” disparaît. Vous pouvez également placer le curseur sur le caractère à supprimer et utiliser la touche CLR pour supprimer le caractère. □ Frapper le caractère à insérer (N). □ Valider la ligne modifiée avec la touche[ËNTER|. La flèche gauche «— permet de déplacer le curseur à gauche et ainsi d’effectuer d’autres corrections sur la même ligne. Pour annuler une modification en cours, appuyer sur ESC. Pour modifier une ligne en cours de frappe, les règles sont les mêmes. Lorsqu’une erreur de syntaxe est détectée à l’exécution, BASIC affiche la ligne erronée qui peut ainsi être modifiée en mode EDIT. MODIFICATION PAR RECOPIE __________________________________ Ce mode est plus délicat à utiliser. Il permet de recopier une ou plusieurs parties d’une ligne avec la toucheICOPYl de “sauter” les caractères à supprimer en appuyant sur “SHIFT-»”. Pour modifier la ligne : 10 PRINT “BOMJOUR” □ Positionner le curseur sur la ligne 10 en appuyant sur|SHIFT]et “ f ” simultanément. Il apparaît alors deux curseurs : le curseur principal et le curseur de copie."##########,
    "page 14"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_004,
    r##########"10 PRINT "BONJOUR" ligne
10 PRINT "BO recopiée □ “Sauter” la lettre à supprimer en appuyant sur |SHlFT|et|—T|simultanément. □ Frapper la nouvelle lettre (N) qui est alors ajoutée à la ligne recopiée. □ Recopier le reste de la ligne avec la touche [COPŸj. □ Valider avec|ENTER|. La ligne en cours de recopie peut être modifiée comme dans le mode EDIT. Une modification en cours peut être annulée par|ESC|. Rien ne vous empêche de composer une ligne à partir de plusieurs lignes. Le mode d’édition par recopie peut également être utilisé pour modifier une commande mal frappée. Pour sauvegarder un programme sur cassette ou disquette, utiliser la commande SAVE “nom-programme” (cf. chapitre “Commandes” page 18). Exemple : SAVE “ESSAI”. La commande “LOAD” permet de lire un programme sur cassette. Exemple : LOAD “ESSAI”."##########,
    "page 15"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_005,
    r##########"10 DATA &HFF
20 READ x
30 PRINT x DEFINT - DEFREAL - DEFSTR ____________________________________ (définition globale de type de variables) DEFtype lettres Permet de définir globalement le type de toutes les variables dont les noms commencent par les lettres spécifiées, plutôt que de déclarer explicitement le type de chaque variable par un caractère (%, $, I). On peut cependant déclarer explicitement par un caractère (I, %, $) un type de variable qui aurait été défini par DEFtype. La déclaration explicite est prioritaire. DEFREALJ Toutes les variables commençant par J et non déclarées expli­ citement par un caractère (!, %, $) sont des variables réelles. DEFSTR A-C Toutes les variables commençant par une lettre A, B ou C sont du type chaîne. DEFINT l-N, R-T Spécifie deux domaines de variables entières."##########,
    "page 23"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_006,
    r##########"10 DEFINT a
20 a=2.1
30 aa!=2.3 ' declaration explicite Prioritaire
40 PRINT a,aa! CONVERSION DE TYPES DE VARIABLES Le stockage d’une valeur se fait suivant le type de la variable."##########,
    "page 23"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_007,
    r##########"10 a‘;=123.45
20 PRINT a‘i 123
22 I BASIC AMSTRAD LES EXPRESSIONS ET OPERATEURS Une expression peut être simplement une constante du type numérique ou chaîne, une variable ou une combinaison de constantes et de variables liées par des opérateurs. Exemple : (X - 2) + 4/6 Les opérateurs effectuent des opérations sur des valeurs. Ils sont classés en trois catégories : 1/Arithmétiques 2/Relationnels 3/Logiques OPÉRATEURS ARITHMÉTIQUES _________________________________ Exponentiation Multiplication Division + - Addition Soustraction MOD Modulo \ Division entière L’évaluation des expressions se fait avec l’ordre des priorités des opérateurs défini ci- dessus."##########,
    "pages 23, 24"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_008,
    r##########"20 mxm=-< < a >b )*a+( a<b )*b ')
30 PRINT mxrn 5 Remarque : La fonction MAX(X,Y) donne directement le maximum de deux nombres. OPÉRATEURS BOOLEENS _______________________________________ La manipulation de bits et les opérations booléennes sur ces bits s’effectuent avec les opérateurs AND, OR, XOR. Ces derniers opèrent sur des groupes de 16 bits au plus qui sont spécifiés par des nombres allant de -32768 à +32767 (représentés en complément vrai de façon in­ terne). Les opérations s’effectuent BIT à BIT. Exemple : 15 --------->0600000000001111"##########,
    "page 26"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_009,
    r##########"15 AND 4 ------------------>0000000000000100 —------->4 Exemple : 4 --------->0000000000000100 --------->0000000000000010"##########,
    "page 26"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_010,
    r##########"26 I BASIC AMSTRAD La couleur du stylo (PEN) et du papier (PAPER) se définit à l’aide d’une table de correspondance. A la mise sous tension, PAPER est égal à 0 et “pointe” vers la case 0 de la table de correspondance (couleur 1) ; PEN est égal à 1 et “pointe” vers la case 1 de la table (couleur 24). PEN 3 fait pointer vers la case 3 (couleur 6). PAPER 2 fait pointer vers la case 2 (couleur 20). Essayez en mode direct : PEN 3 PAPER 2 INK case, couleur ------------------ ------------------------------------------------------ Change la couleur d'une case de la table de correspondance. INK 0,26 place la couleur
26 dans la case 0. Si PAPER est égal à 0, la couleur de fond devient blanche. o X 26"##########,
    "pages 27, 28"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_011,
    r##########"10 MODE 0
20 FOR P = 1 TO 15
30 PEN P
40 PRINT "stylo";P
50 NEXT P stylo 1 stylo 2 stylo 3 stylo 4"##########,
    "page 28"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_012,
    r##########"10 '--------------écriture texte
20 MODE 0
30 INK 0-24
40 DIM c<15>
50 FOR P = 1 TO 15
60 et P)=P
70 PEN P"##########,
    "page 29"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_013,
    r##########"30 PR I NT LEFTSC " RMSTRRD...................." > P )
30 NEXT P
100 '-------- Permutation couleurs
110 x=c(l)
120 FOR P=1 TO 14
130 c<P)=c<P + l)
140 INK P-cCP)
150 NEXT P
160 c( 15>x= INK 3,c( 15)
170 FOR tP = l TO 400:NEXT tP"##########,
    "page 29"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_014,
    r##########"10 MODE 0
20 FOR P=1 TO 15
30 PLOT Pt30,100,P
40 DRRN Pt30-200-P
50 NEXT P CLS Cette instruction permet d’effacer l’écran. TABLE DES COULEURS __________________________________________ 1° Couleur de l’encre n° Couleur de l’encre n° Couleur de l’encre"##########,
    "page 29"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_015,
    r##########"28 I BASIC AMSTRAD TABLE DE CORRESPONDANCE DES COULEURS A LA MISE SOUS TENSION _______________________________________ n° stylo/papier Couleur Mode 0 Mode 1 Mode 2"##########,
    "pages 29, 30"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_016,
    r##########"10 14 20 1
11 16 6 24
12 18 1 1
13 22 24 24
14 1,24 20 1
15 16,11 6 24 Remarque importante : Lorsque vous utilisez PAPER 0 et PEN 1 par exemple, si les deux cases 0 et 1 contiennent la même couleur, le texte affiché n’apparaît pas. Pour le faire apparaître, frapper en mode direct : INK 0,1 s INK 1,24 : PAPER O : PEN 1 INK case, couleurl, couleur! _______________________________________ Lorsque deux couleurs sont définies dans INK, les deux couleurs sont affichées alter­ nativement. BORDER couleurl, couleur! _______________________________________ Définit la couleur du pourtour de l'écran. Lorsque deux couleurs sont spécifiées, les couleurs sont affichées alternativement."##########,
    "page 30"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_017,
    r##########"10 INK 1,24,6 ' couleur 24 et 6
20 SPEED INK 100,20 ’ 1sec et .2sec JEU DE CARACTÈRES ___________________________________________ Le programme ci-dessous affiche le jeu de caractères de l’AMSTRAD."##########,
    "page 31"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_018,
    r##########"10 ,-------------------------jeu de caractères RMSTRRD
20 FOR c=32 TO 255
30 PRINT CHR$<c);
40 NEXT c Il existe une série de caractères accessibles au clavier avec CTRL. Le programme suivant affiche la table de codes. On trouvera en annexe la liste des codes ASCII."##########,
    "page 31"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_019,
    r##########"10 ■'---------------------------affichage codes écran
20 MODE 2 3© debut=31 ’ code debut
40 ncol=10 ? nb de colonnes
50 ni=24
60 '
70 FOR 1=1 TO ni SO FOR cl = l TO ncol
90 cd=<l-l )*ncol+cl+debut:IF cd>255 THEN END
100 PRIHT cd >CHR«<cd >;
110 NEXT cl
120 PRINT
130 NEXT I LOCATE X,Y ____________________________________________________ Positionne le curseur texte dans la colonne X et à la ligne Y. Pour LOCATE, l’écran est divisé en 25 lignes de 20, 40 ou 80 colonnes suivant le mode défini par l’instruction MODE. Pour LOCATE, l’origine de l’écran est en haut et à gauche."##########,
    "page 31"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_020,
    r##########"10 MODE 1
20 LOCRTE 10,15 ' colonne 10,li9ne 15
30 PRINT "COUCOU
30 I BASIC AMSTRAD Le programme ci-dessous fait défiler le message « AMSTRAD »."##########,
    "pages 31, 32"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_021,
    r##########"10 '------------------enseigne
28 MODE 1
30 æ$=”AMSTRAD..........."
40 ■'
50 LOCATE 10,10:PRINT .3.$
60 a$=RIGHT$( .a$, 1 >+LEFT$( -a$, LEN< a$ >-l )
70 FOR tP = l TO 100:NEXT tP
80 GOTO 50 WINDOW #numéro fenêtre, gauche, droite, haut, bas -------------------------- WINDOW définit une fenêtre d'écran. L’écriture se fait seulement dans la fenêtre adres­ sée."##########,
    "page 32"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_022,
    r##########"10 ’------ exemple de fenetre
20 MODE 1
30 WINDOW #1,20,30,12,16
40 PEN #1,1 : PRINT #1,"li9ne1 fenetre1"
50 FEN #1,3: PRINT #l,"li9nel fenetrel"
60 '
70 PRINT "li9nel"
80 PRINT "Ii9ne2""##########,
    "page 32"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_023,
    r##########"10 PRINT "ASDF";
20 PRINT POSC#0 >"##########,
    "page 33"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_024,
    r##########"10 CLS
20 LOCATE 10,20:PRINT "COUCOU";
30 PRINT VPOS(#0) WINDOW SWAP fenêtrel, fenêtre! __________________________________ Commute deux fenêtres. Ce qui s’écrivait dans la fenêtre 1 s'écrit dans la fenêtre 2."##########,
    "page 33"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_025,
    r##########"10 MODE 1
20 WINDOW #1, 10,20,10,20
30 WINDOW #2,22,32,2,8
40 PRINT #2,"fenetre2"
50 PRINT #1,"fenetre 1"
60 WINDOW SWAP 1,2
70 PRINT #1,"suite fenetre 1""##########,
    "page 33"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_026,
    r##########"10 INPUT "Votre nom ";nomî
20 PRINT nomJ run Votre nom ? DUPONT DUPONT Exemple avec deux variables :"##########,
    "page 34"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_027,
    r##########"10 INPUT "Nom>a9e ";nom$.>a9e
15 '
20 PRINT nommage run Nom,a9e ? DUPONT, 30 DUPONT 30 Pour une variable chaîne, si l’opérateur appuie sur ENTER sans entrer de valeur, la variable chaîne devient vide. Une variable numérique prend une valeur égale à 0. Si l'opérateur entre une chaîne alors que c’est une valeur numérique qui est attendue, le message “REDO FROM START” est envoyé."##########,
    "page 34"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_028,
    r##########"20 PRINT rue* Ru.e? 11, rue NOBEL 11,rue NOBEL Remarque : BASIC n’envoie pas de point d’interrogation à la suite du message comme c’est le cas avec ’INPUT’. INKEY$ _________________________________________________________ INKEY$ lit le clavier en permanence. Si aucun caractère n'a été frappé, la chaîne lue (C$ sur l’exemple) est vide. Le caractère frappé au clavier n’est affiché que si le pro­ gramme l’a prévu (et non pas automatiquement comme c’est le cas avec INPUT). Tous les caractères frappés doivent être analysés, y compris le caractère ENTER (code ASCII 13) et le caractère “DEL” (code 127), (avec “INPUT” ces caractères sont gérés par “Basic”).
30 c$=INKET$:IF c$="" THEN 30 ' boucle d'attente
40 PRINT RSC<c$),c$ ‘ affiche le code et le caractère
50 GOTO 30
65 R
66 B Avec INKEY$, le curseur n'apparaît pas."##########,
    "page 35"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_029,
    r##########"10 C$=INKEY$
28 IF'cîO"" THEN END
30 PRINT "HPPuaer sur- une touche"
40 GOTO 10 HP Pua er sur une touche HP Puaer sur- une touche- AP Puaer sur une touche HP Puaer sur- une touche- HP Puaer sur une touche BOUCLE D’ATTENTE □ Attend que l’opérateur appuie sur une touche quelconque."##########,
    "page 36"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_030,
    r##########"10 c$=INKEY$ IF c$="" THEN 10
20 PRINT "C'est Parti" □ Teste si l’opérateur répond assez vite."##########,
    "page 36"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_031,
    r##########"10 PRINT "RePondez (O/N) (vite) "
20 h=TIME
30 '
40 r$=INKEYSD IF r$<>"" THEN 80
50 IF TIME>h+1000 THEN PRINT "TroP tard"=END
60 GOTO 40
70 '
80 ' suite Le programme ci-dessous saisit une chaîne de caractères. Les caractères frappés peuvent être contrôlés dès leur frappe sans attendre la validation par ENTER."##########,
    "page 36"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_032,
    r##########"10 '---------------------saisie d'une ligne avec INKEYÎ
30 MODE 2
40 xs=10;as=10 ' coordonnées saisie
50 GOSUB 80
60 PRINT ■■ PRINT li9S
65 END
70 '---------------------------------------------------------spgrn saisie dans lig$"##########,
    "page 36"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_033,
    r##########"30 li9$=""
90 '
100 19=LEN(li9$)=LOCATE xs+19,as:PRINT CHR$<143);CHR$<32) ' 1 43■curseur
110 LOCHTE xs+l9>as
120 '
130 cS=INKEY#:IF cS="" THEN 130 ' attente fraPPe
140 c=ASCCC$)
150 '
160 IF c<>127 THEN 190 ' code suppression
170 IF 19>0 THEN li9S=LEFT$Cli9$,19-1);PRINT CHR$(8);CHR5K 32)■ GOTO 100 ELSE 100
180 '"##########,
    "page 36"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_034,
    r##########"130 IF c=13 THEN 250 ' code ENTER?
200 IF c<32 OR c>127 THEN PRINT CHR$( 7 >; ■ GOTO 130
210 Ii9$=li9$+c$ ' ajout caractère
220 PRINT c$ ' afficha9e caract
230 GOTO 100
240 '
250 LOCATE xs+19,as;PRINT CHRÏC32)
260 RETURN INKEY (n° touche) ________________________________________________ Permet de tester si une touche est enfoncée. Lorsque l’utilisateur appuie sur la touche spécifiée, la valeur obtenue est égale à 0. Si l’utilisateur appuie simultanément sur une touche et SHIFT ou CTRL ou SHIFT et CTRL, on obtient : touche non enfoncée —1 touche seule 0 touche + SHIFT 32 touche + CTRL 128 touche + SHIFT + CTRL 160"##########,
    "page 37"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_035,
    r##########"10 PRINT "APPUYEZ SUR 'A'"
20 •’
30 IF INKEY(63>=0 THEN PRINT "Vous aPPuaez sur A"
40 IF INKEY<69>=32 THEN PRINT "Vous aPPuaez sur A et SHIFT"
50 IF INKEYC63>=128 THEN PRINT "Vous aPPuaez sur CTRL et A"
60 GOTO 30 INKEY ne tient pas compte du délai de répétition comme INKEY$. Plusieurs touches enfoncées peuvent être testées simultanément. Le programme ci-dessous permet de rechercher le numéro d'une touche."##########,
    "page 37"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_036,
    r##########"10 ■'-------------------------------fonction INKEYCtouche)
20 PRINT "APPuaez sur une touche"
30 FOR i=0 T0 73
40 c=INK.EY( i >
50 IF c=0 THEN PRINT "code="ji ' touche enfoncee?
60 NEXT i
70 GOTO 30 Les numéros de touche sont distincts des codes ASCII des caractères associés. Les caractères associés aux touches peuvent être redéfinis par KEYDEF."##########,
    "page 37"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_037,
    r##########"10 ' telécran
15 '
20 LOCATE 1,22’PRINT "utiliser les fléchés
30 MODE 1
40 '
50 X=200;3=200
60 '
70 IF INKEY<8)=0 THEN x=x-l
80 IF INKEYC1>=0 THEN x=x+l
90 IF INKEY(0)=0 THEN 3=3+1
100 IF INKEY<2)=0 THEN 3=3-1
110 ’
120 PLOT x. ,3,1
130 GOTO 70 Pour obtenir un déplacement dans les directions diagonales, appuyer sur deux flèches simultanément. Nous avons écrit le programme de télécran ci-dessus d’une autre façon. L’expression (INKEY(8)=0) s’évalue en -1 si le test est vrai et en 0 si le test est faux. En la combi­ nant à (INKEY(1 )=0) nous obtenons le déplacement du curseur sur l’axe horizontal."##########,
    "page 38"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_038,
    r##########"10 ‘---------------------------tel écran avec fléchés
20 MODE 1
30 LOCATE 1,22’PRINT "Utiliser les fléchés"
40 X=200’3=200
53 '
60 x=x+< INKEYC 8 )=0 >-( INK.EYC 1 )=© >
70 3=3-( IblKEM 0 )=0 )+( INKEYC 2 >0 )
80 '
90 PLOT x,3,l
100 GOTO 68 Le programme "télécran” ci-dessous permet de dessiner et d’effacer dans 8 directions. La couleur se choisit avec 1,2,3."##########,
    "page 38"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_039,
    r##########"10 •’-----------------------------------tel écran 8 directions + couleurs
20 MODE 1 ! INK 0/24-INK 1, 1 '■ PAPER 0 : PEN 1
30 ce=l:cf=0 ' ecriture/fond
40 SPEED KEY 10/2 ' vitesse clavier
50 LOCATE 1/20:PRINT "fleches Pour dePlacer"
60 LOCATE 1/21:PRINT "L:lever B=baisser E:effacer Couleurs: 1,2/3"
70 x=200:y=200
80 •'--------------------------------------------------curseur clignotant
90 t=TEST<X/a)
100 '
110 cS=INK.EY$ : IF c$<>"" THEN 150
120 PLOT x/y/ce: PLOT x/y/cf
130 GOTO 110 140
150 IF 1=0 THEN PLOT x/y.. ce
160 IF 1=1 THEN PLOT x/y/t
170 IF 1=2 THEN PLOT x/y/cf
180 ■'
190 IF INKEYC8)=0 THEN IF x>2 THEN X=X-2
200 IF INKEY<1 )=0 THEN IF x<600 THEN X=X+2
210 IF INKEY<0)=0 THEN IF a<397 THEN Y=Y+2
220 IF INKEY<2)=0 THEN IF y>2 THEN Y=Y-2
230 '
240 c$=UPPERS<c$)
250 IF c$="L" THEN 1 = 1 ' lever-
260 IF c$="B" THEN 1=0 ' baisser
270 IF c$="E" THEN 1=2 ' effacer
280 IF VAL<c$><>0 THEN ce=VAL< c$ )
290 GOTO 90 SPEED KEY délai, intervalle répétition ---------------------------------------------- Règle le délai de répétition des touches et la vitesse de répétition en 1 /50e de seconde. SPEED KEY 10,20 A la mise sous tension, SPEED KEY est égal à 10,3. ON BREAK GOSUB n° ligne --------------------------------------- - ------------------ Définit l’adresse d’un sous-programme vers lequel il y aura branchement si l’opérateur appuie sur “ESC” deux fois."##########,
    "page 39"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_040,
    r##########"10 ---------------------------0H break GOSUB
20 ON BREAK GOSUB 100
30 PRINT "APPuyer sur ESCAPE 2 fois"
40 GOTO 30
90 >---------------------sPÿm 0H BREAK
100 INPUT "On arrête vraiment <0/'N) ";r$
105 IF r$="0" THEN END
120 RETURN"##########,
    "page 39"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_041,
    r##########"10 >---------------------------0H BREAK stop
20 ON BREAK GOSUB 100
30 PRINT "APPu.yer su.r ESCAPE 2 fois"
40 GOTO 30"##########,
    "page 40"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_042,
    r##########"30 '---------------------sP9rn ON BREAK
100 INPUT "Annulation UN BREAK (Q/N) ">r$
105 IF r$="0" THEN ON BREAK STOP
120 RETURN KEY numéro, CHR$(n) + chaîne + CHR$(n) Associe une chaîne de caractères à une touche dite de fonction. Lorsque vous appuyez sur la touche de fonction, la chaîne associée à la touche est affichée, facilitant ainsi l’écriture des programmes. Si la chaîne est suivie de CHR$(13), la commande codée dans la chaîne est exécutée. Par exemple, si vous écrivez : KEY 128,"1ist"+CHR$(13) il suffit d’appuyer sur le 0 du "pavé” numérique pour obtenir la liste du programme. La chaîne définie dans KEY ne doit pas dépasser 32 caractères et le total des chaînes pour l'ensemble des touches de fonction ne doit pas être supérieur à 100. Les numéros des 12 touches du “pavé" numérique vont de 128 à 140."##########,
    "page 40"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_043,
    r##########"10 -------------------------touche de fonction
20 KEY 138, "CLS" ' . du. cl avier nu.rner iTu.e
30 '----------------
40 KEY 137, "cls"+CHR$< 13) ' 3 du. clavier nu.rner i 4 u.e
50 '
60 KEY 128,CHR$<13)+"cls:ink 1,.0’ink 1,24"+CHR$(13) ' 0 numérique"##########,
    "page 40"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_044,
    r##########"10 '---------------------redefinition touche M
29 KEY DEF 38,1,65,66,07 ’ normal ),EK shift),C<control) Pour­ touche M “répétition” égal à 1 spécifie que la touche est à répétition. Pour connaître le numéro de touche, cf. “INKEY”. JOY (0 ou 1) _____________________________________________________ Deux joysticks peuvent être connectés sur AMSTRAD. Joystick(n) donne la direction du manche de joystick. Les valeurs décimales obtenues sont : Valeur Direction Bit décimale HAUT 1 0 BAS 2 1 GAUCHE 4 2 DROITE 8 3 FEU1 16 4 FEU2 32 5"##########,
    "page 41"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_045,
    r##########"10 PRINT JOYC0)
28 GOTO 10 run 1 1 8 8 Les joysticks peuvent également être lus comme les touches du clavier par INKEY$ ou INKEY (n° touche). La figure ci-dessous donne les numéros de touche correspondant aux joysticks. touche n° 72 48 î î n° 74 75 50 — —51 l 1
73 49 FEU1: 76 FEU1: 52 FEU2: 77 FEU2: 53"##########,
    "page 41"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_046,
    r##########"10 MODE 1
50 x=280;y=200
60 '
70 IF INKEY(74)=0 THEN x=x-l
80 IF INKEY<75)=0 THEN x=x+l
90 IF INKEY(72>=0 THEN y=y+l
100 IF INKEY<73)=0 THEN y=y-l
110 '
120 PLOT x,y,1
130 GOTO 70 Sans joystick, on pourra utiliser les flèches du clavier. Les numéros de touches sont : 0 î 1 2 La touche “COPY” peut être utilisée comme touche de mise à feu."##########,
    "page 42"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_047,
    r##########"10 INPUT "Nombre FL Nombre B "ja,b
30 IF a>b THEN PRINT "FOB" ELSE PRINT "FK=B"
40 GOTO 10 ’ run Nombre FL Nombre B ? 3,5 R<=B Nombre FL Nombre B ? 6,2 R>B En fait, le test peut se faire, non seulement sur une expression logique, mais aussi sur une expression arithmétique qui est interprétée comme fausse si elle a une valeur nulle ou comme vraie pour toute autre valeur. Mais on évitera d'utiliser cette particularité du BASIC."##########,
    "page 43"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_048,
    r##########"10 IF i THEN PRINT "I est different de 3""##########,
    "page 43"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_049,
    r##########"10 IF <i>10 i<20) THEN PRINT "I est comPris entre 19 et 28
42 I BASIC AMSTRAD Les IF...THEN...ELSE... peuvent être emboîtés, mais il faut alors bien s’assurer qu'à chaque IF-THEN il correspond un ELSE."##########,
    "pages 43, 44"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_050,
    r##########"10 INPUT "Nombre R> Nombre B";a,b
20 IF à<=b THEN IF a<b THEN PRINT "R<B" ELSE PRINT "R=B" ELSE PRINT "R>B""##########,
    "page 44"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_051,
    r##########"10 FOR c=l TO 4 'Pour c=l JusAu'a 4
20 PRINT c,c*c
30 NEXT c 'augmenter c Puis aller aPres for si c<4
40 PRINT "fin";c"##########,
    "page 46"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_052,
    r##########"10 '------------------exemple avec steP négatif
20 FOR c=4 TO 1 STEP -1
30 PRINT c/c*c
40 NEXT c
50 PRINT c;"fin" i 4s"##########,
    "page 46"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_053,
    r##########"30 FOR i=l TÛ x
40 IF i=5 AND f=l THEN x=8
50 PRINT i;
60 NEXT i"##########,
    "page 46"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_054,
    r##########"10 FOR i=l TO 5
20 PRINT i;
30 INPUT "nombre ";a(i)
40 IF a< i X.0 THEN PRINT "erreur" ; i=i-l
50 NEXT i"##########,
    "page 46"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_055,
    r##########"10 FOR i=l TO 3
20 PRINT"i=
30 FOR J=1 TO 8
40 PRINT Ji
50 NEXT J
60 PRINT
70 NEXT i i = 1 J = 1 2 3 4 5 6 7 8 i = 2 J = 1 2 3 4 5 6 7 8 i = 3 j = 1 2 3 4 5 6 7 8 S’il n’y a pas d’instruction entre NEXT J et NEXT I, NEXT J,I donne le même résultat. NEXT (au lieu de NEXT J) est accepté, puisqu'on fait NEXT incrémente le compteur du FOR le plus récent et que celui-ci est supprimé dès qu’il atteint la valeur limite. Mais pour des raisons de lisibilité, on indiquera le nom de la variable. SORTIE D’UNE BOUCLE FOR On peut sortir d’une boucle FOR par “GOTO” sans problème. (Les versions Microsoft antérieures posaient un problème dans le cas où un indice non “épuisé” était utilisé dans une autre boucle FOR “interne”)."##########,
    "page 47"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_056,
    r##########"10 FOR i=l TO 5 J .
20 IF i=3 THEN GOTO 50 ’sortie Par 9oto
30 NEXT i
40 ’
50 FOR j=l TO 3
60 FOR i=l TO 4 ’i utilise a nouveau
70 PRINT "i = ";i;
80 NEXT i
85 PRINT
90 NEXT J
100 PRINT FRE<0)
110 GOTO 10 i= 1 i= 2 i= 3 i= 4 i= 1 i= 2 i = 3 i= 4 i= 1 i= 2 i= 3 1= 4 43349 i= 1 i = 2 i= 3 i= 4 i= 1 i= 2 i= 3 i= 4 i= 1 i = 2 i= 3 i = 4 43349 i= 1 i= 2 i= 3 i= 4"##########,
    "page 47"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_057,
    r##########"10 WHILE INKEY$="" ' tant Aue INKEYS est vide
20 PRINT "APPuaez sur une touche"
38 HEND run fippuaez sur une touche APPuyez sur une touche RPPuaez sur une touche"##########,
    "page 48"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_058,
    r##########"10 DATA 10, 16, 13? 14 aauu ddééppaarrtt
15 ' \ Dear mo PPooiinntteeuurr ddee DDAATTAA jü rcb.HU Nid aapprrè£sS 22 rReEaAdD eexxééccuuttééss
40 READ N3 ---------------------------
50 READ N4
60 ■'
70 PRINT N1,N2,N3,N4"##########,
    "page 49"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_059,
    r##########"10 16 MOYENNE : 13.25 READ ___________________________________________________________ “READ N1 ” lit la première donnée (10) dans N1. Le pointeur de DATA (géré par BASIC) progresse de 1. Ainsi “READ N2” lit la deuxième donnée dans N2, etc... Les données peuvent être écrites sur plusieurs lignes :
48 I BASIC AMSTRAD"##########,
    "pages 49, 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_060,
    r##########"10 DATA 10,16
20 DATA 13,14 Les lignes 10 et 20 sont équivalentes à la ligne :"##########,
    "page 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_061,
    r##########"10 DATA 10,16,13,14 L’implantation des DATAs dans un programme n'a pas d’importance. Elles sont lues dans l’ordre de la numérotation. Les chaînes de caractères comportant des caractères spéciaux doivent être placées entre guillemets. Sur l’exemple ci-dessous, sans la présence de guillemets, la virgule serait considérée comme séparateur.
10 DATA "8,rue de Provence"
20 '
38 READ x$
40 PRINT x$ 8, rue de Provence RESTORE _______________________________________________________ Positionne en début de DATA ce qui permet de relire les données depuis le début."##########,
    "page 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_062,
    r##########"10 DATA 6,3,14
20 '
30 READ A,B,C PREMIERE LECTURE
40 '
50 RESTORE ■'DEBUT DATA
60 '
70 READ D,E,F 'DEUXIEME LECTURE
80 '
80 PRINT A,B,C
100 PRINT D,E,F o 3 14"##########,
    "page 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_063,
    r##########"10 DATA 6,3,14
20 DATA 4,2,8
30 RESTORE 20 ■'LIGNE 20
40 READ X,'ï',Z
50 PRINT X,Y,Z"##########,
    "page 50"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_064,
    r##########"10 DATA 15j 10
15 Il manque une donnée |
20 READ X
30 READ
40 READ DATA exhausted in 40 ■ SYNTAX ERROR : Le type de la donnée lue doit s’accorder avec le type de la variable."##########,
    "page 51"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_065,
    r##########"10 DATA NICOLAS-*— ,
15 ' Chaîne,
20 READ X -*____ __ ____, jJXIumérique Syntax error in 10 L’exemple ci-dessous lit un nom au hasard dans une liste de quatre noms."##########,
    "page 51"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_066,
    r##########"10 DATA NICOLAS,SOPHIE,JACQUES,BRUNO
20 '
30 RESTORE
40 X=INT(RND*4)
50 ’
60 FOR 1=1 TO X:READ X$:NEXT I
70 ■’
80 READ NOM#
100 PRINT NOMS
110 GOTO 30 NICOLAS BRUNO JACQUES"##########,
    "page 51"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_067,
    r##########"10 DRTR MAISON,PORTE,CHAMBRE
20 DRTA *
30 DRTR JEAN, PIERRE, PAUL,JACQUES
40 DRTR *.
50 DRTR ROUE,ORDRE,FREIN,PEDALE,SELLE
60 DATA *.
70 '
80 RESTORE
90 INPUT "QUEL GROUPE (1,2,3)";G
95 ■'
100 IF G=1 THEN 200
105 ’
110 RERD X$:IF XS="*" THEN G=G-1: GOTO 100
120 GOTO 110
130 ’
200 READ MOTS
210 PRINT MOTS
220 GOTO 80 QUEL GROUPE (1,2,3) ? 2 JERN QUEL GROUPE (1,2,3) ? 1 MAISON QUEL GROUPE (1,2,3) ? 3 ROUE"##########,
    "page 52"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_068,
    r##########"10 DIM dePenseC12) 20
30 FOR rn=l TO 12
40 PRINT "Mois
50 INPUT dePenseCm)
60 NEXT m run Mois 1 1209 Mois 2 1100 Mois 2 1300 Mois 12 ? 1500"##########,
    "page 53"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_069,
    r##########"10 DIM dePense( 12>
20 '
30 FOR m=l TO 12
40 PRINT "Mois ";m;
50 INPUT dePense<m)
60 NEXT m
70 ---------------------------------
90 PRINT
100 INPUT "Mois l";ml
110 INPUT "Mois 2";m2
120 '
140 ttal=0
150 FOR m=ml TO m2
160 ttal=ttal+dePense( m)
170 NEXT m
180 PRINT
190 PRINT "Total!; ml ; m2;ttal
200 GOTO 90 run saisie table Mois 1? 1 Mois 2? 12 Total’ 1 12 14600 Mois 1? 1 Mois 2? 6 Total" 1 6 7200 Il existe pour les tables un élément 0 :"##########,
    "page 54"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_070,
    r##########"18 DIN stck<4,5)
20 '
30 FOR tyPe=l TO 4
40 FOR usn=l TO 5
50 PRINT "Stock/TyPe!";tyPe; "Usineusn;
60 INPUT stckCtaPe,usn)
70 NEXT usn
80 NEXT tyPe Stock/Type■ 1 Usine: 1 ? 10 Stock/Type: 1 Us i ne■ 2 ? 5 Stock/Type: 1 Usine: 3 ? 3 Stock/Type■ 1 Usine: 4 ? 7 Stock/TyPe: 1 Usine 5 ? 9 Stock/Type: 2 Usine: 1 ? S O Stock/TyPe• 2 Usine■ 2 Lorsque la table est documentée, pour connaître le nombre de véhicules d’un type, toutes usines confondues, on fait :
100 INPUT "TyPe ";tyPe
110 ‘
120 ttal=O
130 FOR usn=l TO 5
140 ttal=ttal+stck< type? usn >
150 NEXT usn
160 '
170 PRINT "Total tyPe‘; type;ttal run saisie table TyPe ? 2 Total tyPe; 41"##########,
    "page 55"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_071,
    r##########"10 DIM a<200}
20 PRINT FRE<0> ? Place libre
30 ERRSE a ' effacement table a< )
40 PRINT FRE(0) ' Place libre run 42377 43391 Lorsqu’une table n’est pas dimensionnée par DIM, elle est dimensionnée par défaut avec 10 éléments par BASIC dès qu’un élément de la table est référencé en lecture ou en écriture. Si une instruction DIM est exécutée après le dimensionnement par défaut, le message ARRAY ALREADY DIMENSIONNED est envoyé par BASIC."##########,
    "page 56"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_072,
    r##########"10 a<4>=15 ' dimensionnement implicite
20 DIM a<20) Il faut dimensionner la table avant de référencer un élément. Lorsqu’une table n’est pas dimensionnée explicitement par DIM et que l’on essaie de référencer l’élément 11, on obtient le message SUBSCRIPT OUT OF RANGE."##########,
    "page 56"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_073,
    r##########"10 FOR i=l TO 15
20 PRINT ati )j
30 NEXT i Il faut ajouter :"##########,
    "page 56"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_074,
    r##########"10 PRINT FRE( 0 )
20 DIM -a( 200 )
30 PRINT FREC0) run 43494 42480"##########,
    "page 57"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_075,
    r##########"10 PRINT FREC0)
20 DIM ar/X 100)
30 PRINT FRE<0 ) run 43494 43283
56 I BASIC AMSTRAD LES CHAÎNES DE CARACTÈRES ■ LEFTS ■ LEN ■ ASC ■ STRINGS ■ BINS ■ RIGHTS ■ STR$ ■ CHR$ ■ SPACES ■ UPPERS ■ MID$ ■ VAL ■ INSTR ■ HEX$ ■ LOWERS L’affectation d’une valeur à une chaîne s’écrit : nom de chaîne = « suite de caractères » Exemple : 10 NOMS = « DUPONT » Les « » indiquent que DUPONT doit être interprété comme une chaîne de caractères et non comme une variable. La longueur d’une chaîne de caractères, qui n’a pas à être déclarée, peut varier en cours d'exécution du programme (jusqu’à 255). De même, la longueur de chaque élément d’une table de chaînes peut varier dynamiquement. La concaténation (réunion) de chaînes de caractères est réalisée par l’opérateur noté « 4" »."##########,
    "pages 57, 58"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_076,
    r##########"10 nom$="DUPONT"
20 Pren$="Jean"
30 nP$=nom$+Pren$
40 PRINT nP$ run DUPONTJean COMPARAISONS La comparaison de chaînes de caractères se fait avec les opérateurs : Les chaînes sont comparées caractère par caractère de la gauche vers la droite jusqu’à ce que l’un des caractères d’une chaîne soit plus grand que l’autre (code ASCII supé­ rieur). C’est alors cette chaîne qui est considérée comme la plus grande (“DURAND” est plus grand que “DUPONT”). Si tous les caractères sont égaux, les chaînes sont considérées comme égales."##########,
    "page 58"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_077,
    r##########"10 INPUT "1er nom ";nl$
20 INPUT "2eme nom ";n2$
30 IF nlî>n2$ THEN PRINT nlSU " est Plus grand lue ">n2S
40 IF nl$<n2$ THEN PRINT nl$;" est Plus Petit que "jn2S
50 GOTO 10 run 1er nom ? DURAND 2eme nom ? DUPOND DURAND est Plus 9rand> Aue DUPOND"##########,
    "page 58"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_078,
    r##########"10 INPUT "RePonse (oui ou non) ";r$
20 IF rî="oui" THEN PRINT ‘Vous avez dit oui"
30 IF r$="non" THEN PRINT "Vous avez dit non"
40 IF r$="“ THEN PRINT "Vous avez aPPui-ie sur ENTER"
50 GOTO 10 Remarquez le test d'une chaîne vide en 40(IF R$=“ ”). LEFT$,RIGHTS,MID$ ____________________________________________ LEFTS, RIGHTS, MID$ permettent d’accéder respectivement aux caractères de gauche, de droite et de l’intérieur d’une chaîne. LEFTS (CHAINE, longueur à prendre à gauche) RIGHTS (CHAINE, longueur à prendre à droite) MID$ (CHAINE, position début, longueur à prendre) “CHAINE”, “longueur à prendre” et “position début” peuvent être des expressions. Les valeurs de “longueur à prendre” et de “position début” doivent être comprises entre 0 et 255. Exemple : NOMS = MIDS (X$, 3, 4) I SXS = LEFTS (X$. 2) MR BALU JEAN PRENS = RIGHTS (X$, 4)"##########,
    "page 59"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_079,
    r##########"10 x*="MRBALUJEAN"
20 sx$=LEFT$(x$,2) ' 2 caractères a Gauche
30 Pren$=RIbHT$( x$j4 ) ■' 4 caractères a. droite
40 noro$=MID$( x$> 3/4 ) ' 4 caractères a Partir du 3eme
50 PRINT sxî, Pren$, norn$ run MR JEAN BALU Dans l’exemple ci-dessous, nous testons la première lettre de la réponse :"##########,
    "page 59"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_080,
    r##########"10 INPUT "RePonse (OUI/NON) ";r$
20 IF LEFT$(rî>1)="0" THEN GOTO 190
58 I BASIC AMSTRAD Autre exemple :"##########,
    "pages 59, 60"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_081,
    r##########"10 nomJ="DUBClNET"
20 FÜR i=l TO 7
30 PRINT LEFTïCnom$>i)
40 NEXT i run D DU DUB DUBO DUBON DUBONE DUBONET Si "longueur à prendre” spécifiée est plus grande que la longueur de la chaîne, le résultat est la chaîne elle-même. Lorsque “longueur à prendre” n’est pas précisée dans MID$, cette fonction devient équivalente à RIGHTS. Si “position début” spécifiée dans MID$ est plus grande que la chaîne elle-même, une chaîne vide est retournée. MID$(CHAINE1, position début, longueur à remplacer) = CHAINE2 Permet, à partir de “position début” dans “CHAINE1” et sur la longueur spécifiée, de remplacer des caractères par ceux de “CHAINE2”."##########,
    "page 60"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_082,
    r##########"10 x$=,,MRBRLUJERN"
23 NIDSCx$?3>4 )="XXXX"
30 PRINT x$
40 PRINT np$ ru ri MRXXXXJERN Attention : ne permet pas l'insertion ou la suppression de caractères mais seulement la substitution. Si “CHAINE2” est plus longue que “longueur à remplacer”, seuls les premiers ca­ ractères de “CHAINE2” sont pris en considération. Si “CHAINE2” est plus courte que “longueur à remplacer”, il n’y a substitution des caractères que sur une longueur égale à celle de “CHAINE2”. LEN (chaîne) -------------------------------------------------------------------------------- Donne la longueur d’une expression chaîne."##########,
    "page 60"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_083,
    r##########"10 nom$="DUPONT"
20 l=LEN(nom$)
30 PRINT 1 run 6 STR$(X) _________________________________________________________ Convertit une expression numérique X en une chaîne de caractères."##########,
    "page 61"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_084,
    r##########"10 x=123
20 xî=STR$(x >
30 PRINT x$,LEN(x$) run
123 4 Remarque : Le premier caractère de la chaîne est réservé pour le signe C'est un espace pour un nombre positif et un signe pour un nombre négatif. PRINT riID$(STR$( 123),2,1 > — > 1 VAL (chaîne) _____________________________________________________ Fonction inverse de STR$, elle donne la valeur numérique d une expression chaîne."##########,
    "page 61"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_085,
    r##########"10 xS="123 francs"
20 x=VRL<x$ )
30 PRINT x run 123 Si le premier caractère n’est pas un caractère décimal, un espace, un signe “+”, un signe ou le résultat est égal à zéro. IO PRINT VRLC” 123")"##########,
    "page 61"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_086,
    r##########"20 PRINT VRL<"+123")
30 PRINT VRL<"R123" ) run 123 123 0 ASC (caractère) __________________________________________________ Chaque caractère a un code interne (code ASCII) auquel on accède par la fonction ASC (caractère).
60 I BASIC AMSTRAD"##########,
    "pages 61, 62"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_087,
    r##########"10 x$="R"
20 x=RSC<x$>
30 PRINT x run 65 ASC (chaîne) _____________________________________________________ Donne le code ASCII du premier caractère d’une expression chaîne."##########,
    "page 62"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_088,
    r##########"10 Print a£c("BONJOUR"> — > 66 Une chaîne nulle comme argument provoque une erreur.
10 x$="":PRINT RSCCxî) ImProPer argument in 10 CHR$(X) ________________________________________________________ Fonction inverse de la fonction ASC, elle permet de générer des caractères ayant pour code ASCII la valeur de X. Cette valeur doit être comprise entre 0 et 255. X peut être une constante, une variable ou une expression.
10 FOR i=65 T0 65+25
20 PRINT CHRÎCi)j
30 NEXT i run RBCDEFGHIJKLMNOPQRSTUVHXYZ CHR$(X) est utilisé pour envoyer des “caractères de contrôle” aux périphériques (écran, imprimante). CHR$(8) provoque un retour arrière du curseur. CHR$(10) provoque un passage à la ligne (sans retour en début de ligne). CHR$(13) provoque un retour en début de ligne. Exemples divers : ■ Suppression d’un caractère à droite d’une chaîne :"##########,
    "page 62"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_089,
    r##########"10 x$="DUPONT"
20 x»=LEFT*Cx«,LENCx»)-l>
30 PRINT x« run DUPON"##########,
    "page 62"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_090,
    r##########"13 x«="DUPONT"
20 a«=RIGHT$<" "+x«,8>
30 PRINT y$,LEN(y$) run DUPONT 8 ■ Insertion d’un caractère dans une chaîne :"##########,
    "page 63"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_091,
    r##########"10 zS="RRRRR"
20 x«="B":P=3
30 zS=LEFTS<z$jP)+xS+RIGHTS< z«> LEN<zS)-P)
40 PRINT zS run RRRBRR ■ Remplissage par des zéros à gauche :"##########,
    "page 63"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_092,
    r##########"10 x=123
20 xÆ=RIGHTS(STR$(100000+x5)
30 PRINT x$ run 00123 INSTR (position départ, chaîne, chaine cherchée) Recherche la position d'une chaîne dans une autre. Par défaut, la position de départ est égale à 1 : □ Si la chaîne cherchée n’est pas trouvée, le résultat est égal à 0. □ Si la chaîne cherchée est nulle, le résultat est la position de départ spécifiée. □ Si la position de départ est supérieure à la longueur de la chaîne où s’effectue la recherche, le résultat est nul."##########,
    "page 63"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_093,
    r##########"10 x«="DUPONT.JERN"
20 P=INSTRCxî>)
30 yï=LEFT$(x$>P-l>
40 PRINT uS,P run DUPONT"##########,
    "page 63"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_094,
    r##########"10 PRINT INSTRC3>"RBRRBRRR">"B" ) —> 5 Le programme ci-dessous vérifie si un nom appartient à un ensemble.
62 I BASIC AMSTRAD"##########,
    "pages 63, 64"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_095,
    r##########"10 e$="JERN PIERRE PRUL JACQUES"
20 INPUT "Norn "morn$
30 P=INSTRCeS,nom«)=IF P=0 THEN PRINT "Erreur"=GOTO 20 Cette séquence d'instructions permet de répondre à une question “Mode ?” non pas par un chiffre, mais par une lettre (plus mnémonique)."##########,
    "page 64"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_096,
    r##########"10 INPUT "Node (C, fi, P, R, N) ";rn$ entrer C,H,P,..
15 ’
20 P = INSTRC " CRPRNN " > rn$ ) Position caractère fraPPe?
30 IF P<2 THEN 10 validité?
40 ON P-l GOTO 100,200,400,500,600
50 '
100 PRINT "liane 100";STOP
200 PRINT "ligne 200";STOP"##########,
    "page 64"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_097,
    r##########"10 c$=INKE’r'$ : IF c$="" THEN 10 ' attente caractère
20 P=INSTR< "0123456789. ", c$ ') ' Position caractère fraPPe?
30 IF RSC<cî)=13 THEN 90 ‘ ENTER?
40 IF P=0 THEN PRINT CHR$< 7 )> > GOTO 10 ' caractère invalide?
50 ligî=ligï+c$
60 PRINT c$;
70 GOTO 10
80 ■'
90 PRINT:PRINT lig$ STRINGS (nombre de fois, chaîne) ---------------------------------------------------- Génère une chaîne de caractères égale à la chaîne spécifiée, multipliée par le nombre de fois indiqué."##########,
    "page 64"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_098,
    r##########"10 xî=STRINGS<10,> ' chaine de 10 '
20 PRINT x« run SPACE$(X) ______________________________________________________ Génère une chaîne de X espaces. X peut être une expression."##########,
    "page 64"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_099,
    r##########"10 xî="DUPONT"+SPRCE$(5 )+ "JERN"
20 PRINT xS run DUPONT JERN"##########,
    "page 64"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_100,
    r##########"10 X$=HEX$(14):Yf=HEXI(14,2)
20 PRINT X$,Y$ RUN E 0E BIN$(expression) __________ _______________________________________ Fournit une chaîne représentant la valeur binaire de la valeur donnée."##########,
    "page 65"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_101,
    r##########"10 X$=BIN$(12):Y$=BIN$(12,8)
20 PRINT X$,Y$ RUN"##########,
    "page 65"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_102,
    r##########"10 norn$="roulet"
20 PRINT UPPERS norn$) run POULET LOWER$(chaîne) _________________________________________________ Convertit une chaîne en minuscules."##########,
    "page 65"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_103,
    r##########"10 nom$="R0ULET"
20 PRINT LOWERS* nom*) run rouiet Remarques sur les chaînes : Lorsqu’une chaîne voit sa longueur changée, elle est déplacée. La place occupée par l’ancienne chaîne reste perdue jusqu’à ce que l’espace mémoire soit réorganisé par BASIC. Pour éviter ces réorganisations (longues), on pourra utiliser MID$(x$,p,1)=“XX” qui modifie la chaîne sans la déplacer.
64 I BASIC AMSTRAD Le programme ci-dessous remplit une table de chaînes. PRINT FRE(“ ”) provoque le tassement des chaînes par BASIC."##########,
    "pages 65, 66"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_104,
    r##########"10 DIM a«<500)
20 hd=TIME
30 FOR i=l TO 500 i
40 a« i )="BRSIC"+STR$( i)
50 NEXT i
60 PRINT "TemPsTIME-hd)/300
70 hd=TIME"##########,
    "page 66"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_105,
    r##########"30 PRINT "EsPace 1 ibre ■■ " ; FRE( 0 )
90 PRINT FRE<""> reorganisation chaînes
100 PRINT "TemPs-"KTIME-hd>/300
110 PRINT "EsPace libre-"jFREC0 ) run Temps- 5.05666667 EsPace libre- 35441 37333 Temps- 28.2666667 Espace libre- 37333"##########,
    "page 66"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_106,
    r##########"10 x=123
20 PRINT x run 123 PRINT, _________________________________________________________ L’impression de plusieurs valeurs sur une même ligne peut se faire simplement en séparant dans l’instruction les noms des variables ou les expressions par des virgules."##########,
    "page 67"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_107,
    r##########"10 somme=200=nombre=10
28 PRINT somme,nombre,somme/nombre run"##########,
    "page 67"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_108,
    r##########"10 nom$="DUPONT"
20 Pren$="Jean"
30 PRINT nom#;
40 PRINT Pren$ run DUPONTJean"##########,
    "page 67"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_109,
    r##########"10 Print 123;456;-739 —> 123 456 -739
66 I BASIC AMSTRAD PRINT TAB(X) ----------------------------------------------------------------------------- La fonction TAB(X) permet de positionner directement le curseur à l’intérieur d’une ligne en colonne X, X peut être une expression."##########,
    "pages 67, 68"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_110,
    r##########"10 a=123;b=456
20 PRINT 3. TAEK15) b run"##########,
    "page 68"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_111,
    r##########"10 nom$= ” DUPONT " ; P r-«= " JEAN "
20 PR I NT P r$ ; SPC< 3 ) ; nornî run JEAN DUPONT PRINT USING ___________________________________________________ Considérons maintenant l'outil d’édition le plus puissant du BASIC : le “PRINT USING” ■ VARIABLES NUMÉRIQUES : sans le PRINT USING les valeurs numériques sont cadrées à gauche. Or, c'est généralement à droite qu’elles doivent être cadrées. ■ PRINT USING «#####»; expression numérique Un format défini par une chaîne de # nous permet de cadrer les nombres à droite. Chaque # représente la position d’un chiffre."##########,
    "page 68"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_112,
    r##########"10 x=123;y=1234 ------
20 PRINT USING ” #### "jx Format
30 PRINT USING " #### ";y Cadrage à droite Sans le PRINT USING, nous aurions obtenu des chiffres cadrés à gauche. 123 1234"##########,
    "page 68"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_113,
    r##########"10 x=123;9=1234
20 fmt$="####"
30 PRINT USING fmtîjx
40 PRINT USING fmt«;y run 123 1234 ■ PRINT USING « # # # #.# # » ; expression numérique : Le nombre de chiffres après la virgule qui doivent être imprimés est précisé dans le format par le nombre de # après le"##########,
    "page 69"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_114,
    r##########"10 x=123.456
20 PRINT USING x run _ \ 2 chiffres après 123--------------------------la virgule On remarque que l’arrondi est assuré automatiquement. ■ PRINT USING «LIBELLÉ ####.##», variable numérique Un libellé peut être inséré dans le format."##########,
    "page 69"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_115,
    r##########"10 somme-1 2.3. 456
20 PRINT USING " Total= ####.## FRANCS somme run Total : 123.46 FRANCS Formats multiples : Plusieurs formats peuvent être spécifiés dans une seule instruction."##########,
    "page 69"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_116,
    r##########"10 somme=1234.567:tva=17.6
20 PRINT USING "Total= ####.## Tva ##.## " isomme,tva run Total: 1234.57 Tva 17.60 Si le format est le même pour plusieurs variables, on fait :"##########,
    "page 69"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_117,
    r##########"10 x=123=9=456
20 PRINT USING "####.##";x,9 123.00 456.00
68 I BASIC AMSTRAD Le signe “+” n’est imprimé que s’il est prévu dans le program­ me."##########,
    "pages 69, 70"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_118,
    r##########"10 Print usin9 " ####">123 —-> 123
10 Print usina "+####">123 --> +123
10 Print usina "+####">-123 --> -123 Un signe en fin de format provoque l’impression du signe à la fin d’un nombre négatif.
10 Print usina "####-">-123 --> 123­
10 Print usina "####-">123 —> 123"##########,
    "page 70"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_119,
    r##########"10 Print usina "O####"> 123 --> 0*123
10 Print usina "O####"; 1234 --> 01234"##########,
    "page 70"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_120,
    r##########"10 Print usina "$$####">123 --> $123 “**$” combine les effets et
10 Print usina "**$####">123 —> 00123 Si le nombre de positions spécifié dans le format est insuffisant pour la valeur à imprimer, le message % est imprimé devant la valeur.
10 Print usina "###">1234 — > *1234 ■ CHAÎNES DE CARACTÈRES PRINT USING «\ \» ; expression chaîne : Le nombre d’espaces entre les “\” définit le nombre de caractères à imprimer-2.
10 x»="RENAULT" '
20 PRINT USING "X x">x$"##########,
    "page 70"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_121,
    r##########"10 nom$=" ROULET " ■■ PREN$= " NI COLAS "
20 PRINT USING "8< ! "inom#?Pren$ run RÜULET N Programme pour tester PRINT USING : Le programme suivant permet d'entrer par INPUT à la fois le format et le nombre à imprimer."##########,
    "page 71"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_122,
    r##########"10 INPUT "Format?nombre "?fmt$?nombre
20 PRINT USING +'mt$? nombre run Format?nombre? ####?123 123 POS/VPOS _______________________________________________________ Donnent les coordonnées du curseur."##########,
    "page 71"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_123,
    r##########"10 LOCATE 10?8
20 PRINT "NICOLAS"?
30 PRINT POS(#0)?VPOS(#0) run NICOLAS 17 8 WRITE expression 1, expression! ---------------------------------------------- —------ Procède comme PRINT ... mais sépare les valeurs par des virgules et imprime les chaînes entre guillemets."##########,
    "page 71"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_124,
    r##########"10 a=S0■b=100■x$="NIC0LAS"
20 WRITE a?b?x$ run 80?100?"NICOLAS" ZONE intervalle -------------------------------------------------------------------------— Définit l’intervalle d’édition standard lorsque les variables sont séparées par des vir­ gules.
70 I BASIC AMSTRAD"##########,
    "pages 71, 72"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_125,
    r##########"10 ZONE 13
20 PRINT 1,2,3
30 ZONE 10
40 PRINT 1,2,3 run"##########,
    "page 72"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_126,
    r##########"10 PRINT #3,"DUPONT BLABLA ÉDITIONS IMPRIMANTE/ÉCRAN : Un même programme peut aiguiller des résultats vers l’écran ou l’imprimante.
10 INPUT "Ecran ou imprimante’ <E/I) ";m$
20 '
30 IF m$="E" THEN cn=0
40 IF m$="I" THEN cn=8
50 '
60 PRINT #cn,"BLABLA"
70 GOTO 10"##########,
    "page 72"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_127,
    r##########"10 a=30'b=10 ,15 J___________
20 s=a+b
30 P=a*b
50 PRINT a<b4 "Somme ' "j s; "Produit • ")P
60 '
70 '
90 a=20'bs5l____ ________________ Même séquence
100 s=a+b d’instructions
110 P=atb
120 PRINT a< b> "Somme! 11 j sj "Produit1 " j P
130 '"##########,
    "page 73"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_128,
    r##########"20 GOSUB 500 provoque un branchement du programme en 500 (comme le ferait GOTO 500) mais l’instruction RETURN (RETOUR)) placée à la fin du sous-programme provoque un retour automatique après l’instruction qui suit GOSUB 500, c’est-à-dire l’instruction 30 sur l’exemple. Pour le deuxième appel du sous-programme en 100, le retour se fait en 110.
72 I BASIC AMSTRAD"##########,
    "pages 73, 74"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_129,
    r##########"500 .... 1000 .... 510.... 1010....
530 GOSUB 1000 1030 ....
540 .... 1040 ....
550 .... 1050 .... Pile des
560 RETURN 1060 RETURN adresses de retour Les instructions sont exécutées dans l’ordre suivant :"##########,
    "page 74"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_130,
    r##########"10 INPUT "Votre choix (1,2,3) ";ch
20 '
30 ON ch GOSUB 100,300>500
40 ' A *
50 STOP CH = 1 CH = 2 CH = 3
60 ____ _ _
100 PRINT "Choix 1"
120 STOP_____________ 130J____________"##########,
    "page 76"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_131,
    r##########"300 PRINT "Choix 2" 310STOP
330 ' __ _ [500 PRINT "Choix 3"|"##########,
    "page 76"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_132,
    r##########"10 INPUT "Votre choix (1,2,3) ";ch
20 '
30 ON ch GOSUB 100,300,500
40 PRINT "Suite"
50 GOTO 10
60 '
100 PRINT "Choix 1"
120 RETURN
130 '"##########,
    "page 76"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_133,
    r##########"300 PRINT "Choix 2"
310 RETURN
330 '"##########,
    "page 76"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_134,
    r##########"500 PRINT "Choix 3"
510 RETURN"##########,
    "page 76"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_135,
    r##########"10 ON ERROR LOTO 100 ' en cas d'erreur
15 '
20 INPUT "Diviseur"jd
30 PRINT 10/d
40 GOTO 20
90 '-------------------------------------------------------analyse erreur
100 PRINT "ERR=";ERR;"ERL=";ERL
110 IF ERR=11 AND ERL=30 THEN PRINT "Division Par zero inter­ dite" : RESUME 20
120 PRINT "Erreur non reconnue":STOP run Diviseur? 5 2 Diviseur? 0 Division Par zero interdite Diviseur?"##########,
    "page 77"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_136,
    r##########"10 tnte=0
15 nnte=0
20 '
30 INPUT "NOTE ";nte
40 IF nte=0 THEN 100
50 tnte=nte ’ Erreur! TNTE = TNTE + NTE
60 nnte=nnte+l
70 GOTO 30
80 '
100 PRINT "Moyenne:"jtnte/nnte"##########,
    "page 79"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_137,
    r##########"10 i=l
20 '
30 i = i + l
40 IF i=5 THEN 70
50 GOTO 40
60 ‘
70 END tron run C103C203C30JC403C503C403C503C40JE503C403C503C403C50J C403L503C403 trof f"##########,
    "page 81"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_138,
    r##########"10 RRD
20 PRINT SINCPI/2)
30 DEG
40 PRINT SINC90)
50 RRD run 1 1 ABS(X) Fournit la valeur absolue de X :
100 PRINT ABS(-35) -------*- 35 ATN(X) Donne en radians l’arctangeante de X. CINT(X) Convertit X en un entier avec arrondi :
100 PRINT CINT( 1.6) ---------► 2
110 PRINT CINT(—1.2) ---------► -1 COS(X) Donne le cosinus de X. CREAL(X) Convertit un nombre en réel. EXP(X) Donne l’exponentielle de X. FIX(X) Supprime les chiffres après la virgule :"##########,
    "page 83"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_139,
    r##########"100 PRINT FIX( 2.2) ----------► 2
110 PRINT FIX(—2.2) ----------► -2"##########,
    "page 83"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_140,
    r##########"82 I BASIC AMSTRAD FRE(O) Donne la place libre en mémoire centrale (l’argument 0 n’est pas utilisé). INT(X) Donne la partie entière de X avec arrondi :
100 PRINT INT( 2.2) ------------► 2
110 PRINT INT(-2.2) ------------► -3 LOG(X) Fournit le logarithme de X. LOG10(X) Fournit le logarithme en base 10."##########,
    "page 84"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_141,
    r##########"100 PRINT LOG10(100) -------------► 2 MAX(X,..,Z) Donne le maximum d’une liste.
100 PRINT MAX(15,4) --------------► 15 MIN(X,..,Z) Donne le minimum d’une liste.
100 PRINT MIN(15,4) ---------------► 4 ROUND(X,N) Donne l’arrondi de X avec N décimales.
100 PRINT ROUND (123.456,2) ----------*■ 123.46 SGN(X) Donne le signe X : on obtient respectivement -1,0, +1 pour les valeurs négatives, nulles, positives. SIN(X) Donne le sinus de X. SQR(X) Donne la racine carrée de X (X doit être positif). TAN(X) Donne la tangente de X. UNT(X) Convertit un entier sans signe en un entier entre -32768 et +32767. DEF FN(X,Y,Z) En plus des fonctions internes (telles que SQR, SGN, INT, etc.), l’utilisateur peut définir ses propres fonctions par : DEF FNXX (X,Y,Z,...) = EXPRESSION (X,Y,Z,...) où XX représente un nom choisi par l’utilisateur pour identifier sa fonction (les règles d’appellation sont les mêmes que pour les variables) et X,Y,Z,... les arguments de la fonction. Plus tard, cette fonction sera appelée par le programme avec les valeurs réelles des paramètres."##########,
    "page 84"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_142,
    r##########"18 DEF FNrnoy ( x .< y .■ z )=( x+y +z >/3
20 ■'
30 PR I NT FNrnoy ( 14,6,10 ) run 10 Une fonction ne peut être écrite que sur une seule ligne de 255 caractères au plus. C’est de préférence en tête de programme que sont écrites les fonctions de façon à être interprétées avant qu’elles ne soient appelées."##########,
    "page 85"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_143,
    r##########"10 FOR m=l TO 5
20 PRINT m,PEEK/ri)
30 NEXT m 137 127 O 237 73 •J POKE adresse mémoire, valeur _____________________________________ Range une valeur exprimée en décimal (0-^255) à l’adresse spécifiée. „ . 65"##########,
    "page 87"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_144,
    r##########"10 POKE 50000,65 ' 65 dans 50000 _
20 PRINT PEEK<50000) ' affiche le contenu de 50000 , run 50000 65 65 Naturellement, il faut que la mémoire adressée soit modifiable.
86 I BASIC AMSTRAD Le programme ci-dessous visualise le contenu de la mémoire à l’écran ou sur impri­ mante. Les adresses mémoires et les contenus sont spécifiés en hexadécimal. Pour les visualiser en décimal, supprimer HEX$."##########,
    "pages 87, 88"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_145,
    r##########"10 •----------------------------------------------dumP mémoire
20 MODE 2
30 INPUT "Adresse dePart (en decimal ou hexa (8<xxx>) ";ad
40 INPUT "Combien d'octets "in
50 INPUT "Imprimante (0/N) "> r$
60 IF r$="0" OR r$="o" THEN P=3 ELSE P=0
70 '
80 FOR am=ad TO ad+n STEP 3
90 PRINT #P , "8." ; HEX5( am TAB( 3 ) ;
100 19$=""
110 FOR k=0 TO’7
120 c=PEEK(am+k )
130 PRINT #P,HEX$(c);TAB( 12+k*4);
140 IF c>31 AND c<128 THEN I9$=l9$+CHRS(c> ELSE I9$=l9$+CHR$(32)
150 NEXT k
160 PRINT #P,l9«
170 NEXT am Adresse dePart (en decimal ou hexa (8.XXX)) ? 8.163 Combien d'octets ? 200 Imprimante (O/N) ? & 163 0 0 0 0 0 0 0 0 8.170 2A 0 A 0 1 C0 2D 2D X — 8.178 2D 2D 2D 2D 2D 2D 2D 2D ——------------ 8.180 2D 2D 2D 2D 2D 2D 2D 2D ———————— 8.188 2D 2D 2D 2D 20 64 75 6D --------dum 8.190 70 20 6D 65 6D 6F 69 72 P memoir 8.198 65 0 8 0 14 0 AD 20 e 8.1A0 10 0 3B 0 1E 0 A3 20 ; 8.1 A3 22 41 64 72 65 73 73 65 "Adresse 8.1B0 20 64 65 70 61 72 74 20 dePart 8. IBS 23 65 6E 20 64 65 63 69 (en deci 8.1C0 6D 61 SC 20 6F 75 20 63 mal ou h 8.1C8 65 78 61 20 20 28 26 73 exa ( 8<x 8.1D0 78 78 29 29 20 22 3B D XX > > "i ' 8.1D8 6 0 61 E4 0 1F 0 23 a ( 8.1E0 0 A3 20 22 43 6F 6D 62 "Comb 8.1E8 69 65 6E 20 64 27 6F 63 ien d'oc HIMEM _________________________________________________________ Fournit l’adresse mémoire maxi utilisée par BASIC."##########,
    "page 88"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_146,
    r##########"10 PRINT HIMEM MEMORY adresse ________________________________________________ Modifie l’adresse mémoire la plus haute de la mémoire pour BASIC.
10 PRINT HINEM
20 MEMORY 40000
30 PRINT HIMEM run 43143 40000 INP (n° entrée) ___________________________________________________ Lit sur l’entrée spécifiée une valeur comprise entre 0 et 255."##########,
    "pages 88, 89"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_147,
    r##########"10 FOR n=l TO 3
28 PRINT RNDC1),
38 NEXT n run 0.271940658 0.528612386 0.021330127 run 0.175138616 0.657773343 0.653729687 Pour obtenir des nombres entiers entre 0 et 9 par exemple, il faut : □ multiplier le nombre obtenu par 10, □ prendre la partie entière du résultat, avec la fonction INT (X)."##########,
    "page 91"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_148,
    r##########"10 FOR n=l TO 4
20 x=RND<. 1 )
30 y=x*10
40 z=INT<y)
50 PRINT x.a.z
60 NEXT 0.810653739 8.10653738 8 0.173633121 1.73633121 1 0.350786642 3.50786643 3 0.580428399 5.80428399 5
90 I BASIC AMSTRAD Pour obtenir un nombre entier entre 1 et 10, il suffit d’ajouter 1."##########,
    "pages 91, 92"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_149,
    r##########"19 FOR n=l T0 6 2Û PRINT INT(RND*10)+1;
30 NEXT n"##########,
    "page 92"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_150,
    r##########"10 FOR n=l TO 3
20 PRINT RND
30 NEXT n 4U PRINT RNDC0) 0.182864718 0.352758596 0.375612436 0.375612436 RANDOMIZE Initialise une série aléatoire qui dépend de X. La série est toujours la même pour X donné."##########,
    "page 92"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_151,
    r##########"10 RANDOMIZE 6 ' initialise une serie
20 ’
30 FOR i=l TO 3
40 PRINT RND(l)
50 NEXT i run 0.271940658 0.528612386 0.021330127 run 0.271940658 0.528612386 0.021330127 Pour générer des séries aléatoires différentes, faire : RANDOMIZE TIME."##########,
    "page 92"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_152,
    r##########"10 RANDOMIZE.TIME
20 FOR X=1 TO 3
30 PRINT, RND(l)
40 NEXT x 0.369625518 0.925758611 0.766109714 0.782020996 1.43015E-02 0.847328433"##########,
    "page 92"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_153,
    r##########"92 I BASIC AMSTRAD PLOT X,Y,stylo PLOTR DX,DY,stylo ---------------------------------------------------------------------- PLOT X,Y allume le point X,Y spécifié. n
100 100,100"##########,
    "page 94"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_154,
    r##########"10 MODE 2 A
20 PLOT 100,100,1 100 Allume le point de coordonnées 100,100. Si le mode spécifié est MODE 1, le point allumé est plus gros."##########,
    "page 94"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_155,
    r##########"10 MODE 1
20 PLOT 100,100,1 Mais l’affichage se fait au même endroit. PLOTR DX,DY allume un point par rapport au point courant :"##########,
    "page 94"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_156,
    r##########"10 MODE 2
20 PLOT 100,100,1
30 PLOTR 20,20,1 ' decalaGe de 20,20 <120,120 en absolu.) DRAW X,Y,stylo DRAWR DX,DY,stylo _____________________________________________ DRAW trace une droite entre le point courant et le point spécifié."##########,
    "page 94"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_157,
    r##########"10 MODE 2
20 PLOT 100,100
30 DRAW 200,200 ’ droite entre 100,100 et 200,200 200,200 100,100"##########,
    "page 94"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_158,
    r##########"10 MODE 2
20 PLOT 100,100,1
30 DRRWR 100,100,1 droite entre 100,100 et 200,200 Les programmes ci-dessous représentent un carré de 100,100 en absolu et en relatif :"##########,
    "page 95"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_159,
    r##########"10 ,-------------------------carre
20 MODE 1
30 PLOT 100,100,1 ' Positionne en 100,100
40 DRAW 200,100,1 ' droite avec le Point 200,100
45 DRAW 200,200,1
50 DRAW 100,200,1
60 DRAW 100,100,1 carre en relatif"##########,
    "page 95"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_160,
    r##########"10 MODE 1
20 PLOT 100,100,1 ' Positionne en 100,100
30 DRAWR 100,0,1 ' droite dx=100 dy=0
40 DRAWR 0,100,1 ' droite dx=0 da=190 100,100
50 DRAWR -100,0,1
60 DRAWR 0,-100,1 Le programme ci-dessous représente un rectangle plein :"##########,
    "page 95"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_161,
    r##########"10 '------------------rectangle
20 MODE 1
30 xa=10©•aa=100 ' origine
40 1=200 ‘ longueur
50 h=100 ' hauteur
60 FOR x=xa TO xa+l
70 PLOT x,ya,l
80 DRAW x,ya+h,l"##########,
    "page 95"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_162,
    r##########"10 INK O,1 : INK 1,24 MOVE X, Y, stylo MOVER dx, dy, stylo _____________________________________________ Positionne le curseur graphique (comme PLOT et PLOTR) mais sans affichage. XPOS/YPOS _____________________________________________________ Fournit la position du curseur graphique.
94 I BASIC AMSTRAD"##########,
    "pages 95, 96"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_163,
    r##########"10 MODE 1
20 MOVE 100> 100
30 DRAM 150?140
50 PRINT XPOS/t'PO!
150 140 CLG papier ______________________________________________________ Efface l’écran graphique. Positionne le curseur en bas de l’écran graphique. CLG 2 TEST (X,Y) TESTR (X,Y) ____________________________________________________ Donne le numéro de stylo qui a été utilisé pour le point X,Y."##########,
    "page 96"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_164,
    r##########"10 MODE 1 : PEN 1 : PAPER 0
20 PLOT 100,100,1
30 PRINT TEST(100,100) RUN ORIGIN X,Y, gauche, droite, bas, haut Redéfinit l’origine du curseur graphique. Les coordonnées spécifiées dans PLOT, DRAW et MOVE sont relatives à la nouvelle origine. Ci-dessous l’origine est redéfinie en 100,100. DRAW 30,30 trace une droite entre la nouvelle origine et le point 130,130."##########,
    "page 96"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_165,
    r##########"10 MODE 1
20 ORIGIN 100? 100 ■' redéfinit I'orisine
30 DRAM 30,30 ' trace une droite a Partir de 100,100 Si les paramètres gauche, droite, bas, haut sont spécifiés, une fenêtre graphique est définie."##########,
    "page 96"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_166,
    r##########"10 FENETRE GRAPHIQUE
20 MODE 1
30 INK 1,0:INK 0,26
40 Origine 440,100
50 ORIGIN 440,100,440,640,100,300
60 CLG 2
70 '-­
80 DRAW 200,200,3
90 MOVER -5,-2
100 FILL 3 CCPPCC666644 SSEEUULLEEMENT"##########,
    "page 96"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_167,
    r##########"20 TAG H ri o rn mv
30 FOR x=100 TO 200 STEP 10 IHO 1HHV
40 MOVE x,x
50 PRINT "AMSTRAD";
68 NEXT x TAGOFF annule l’effet de TAG ; l’affichage par PRINT se fait normalement à partir du curseur texte. Voici divers exemples de programmes utilisant la haute résolution. □ Ce programme affiche un histogramme de 4 valeurs définies en DATA."##########,
    "page 97"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_168,
    r##########"10 -------------------------histogramme
20 MODE 1
30 X0=20;30=100 ' or i 9 i ne
40 19=15 ' l ar9eur
50 i tv=20 ' intervalle
60 DATA 50,70,30,80
70 FOR i = l TO 4
80 READ h
80 xb=x0+<i-1>titv
100 ce=INT(RND<1 )*3 )+1 ' couleur
110 FOR dx=l TO 19 ' rectan91e
120 PLOT xb+dx,y0,ce;DRAW xb+dx,y0+h,ce
130 NEXT dx
140 NEXT i
150 '— axes
160 PLOT x0,y 0,1 : DRAW X0+100,y 8
170 PLOT x0,y0,1■DRAW X0,y0+100"##########,
    "page 97"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_169,
    r##########"30 ' essayer avec mode 1
40 '
50 MODE 0
60 INK 0,1=INK 1,24:PAPER 0:REN 1
70 x0=20:y0=100 ' or i 9 i ne S0 19=15 ' largeur
90 i tv=20 ' intervalle
100 FOR i=l T0 15
110 xb=x0+(i-1i tv
120 ce=i ’ couleur
130 FOR dx=l TO 19 ' rectangle
140 PLOT xb+dx,y0,ce:DRAW xb+dx,y0+100,ce
150 NEXT dx
160 NEXT i □ Anneau En traçant un cercle plein en couleur de fond sur un cercle plein, nous représentons un anneau."##########,
    "page 98"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_170,
    r##########"20 MODE 1
30 INK 0,1 =INK 1,24=PAPER 0 = PEN 1 O
40 x0=100=30=100=r=60=ce=1=GOSUB 80 ' 1er cercle
50 r=40 = ce=0 = GOSUB 80 ’ 2eme cercle
60 END
70 ’----------------------------------------cercle
80 r2=rtr
90 FOR dx=-r TO r
100 dy =SQR< r2-< dxtdx > '<
110 PLOT x0+dx,y0+dy= DRAW xO+dx,y0-dy,ce
120 NEXT dx
130 RETURN □ Croissant Avec le même principe que ci-dessus, nous représentons un croissant."##########,
    "page 99"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_171,
    r##########"10 ■'-----------------------------croissant
20 MODE 1
25 INK 0,1=INK 1,24=PAPER 0 = PEN 1
30 x0=100=y0=100=r=60=ce=l=GOSUB 70 ’ 1er cercle
40 x0= 160 = r=80 = ce=0 = GOSUB 70 ‘ 2eme cercle
50 END
60 ’----------------------------------------cercle
70 r2=rtr
80 FOR dx=-r TO r
90 =SQR( r-2-( dx*dx ) )
100 PLOT x0+dx,yO+dy=DRAW x0+dx,30-dy,ce
110 NEXT dx
120 RETURN □ Ours Le tracé de l’ellipse est réalisé en faisant pivoter une droite autour du centre. Le “pas” dépend de la grandeur de l'ellipse."##########,
    "page 99"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_172,
    r##########"10 >----------------------------------------ours
20 MODE 1
30 INK 0,1 = INK 1,24 = PAPER-0 =PEN 1
40 X0=100=y0=100
50 X1=X0 = 3 1=3 0 = r=60 = ce==l = GOSUB 180 ' tete
60 Xl=X0-40=Yl=Y0+50=R=40=GOSUB 180 ' oreilles
70 X1=X0+40 = Y1=Y0+5Ô = R=40 = GOSUB 180
80 xl=x0=31=30-10=r=10=ce=0=GOSUB 180 ' nez
90 xl=x0-20 = y 1=30+20 = r=6 = GOSUB 180 ' 3 eux
100 xl=x.0+20 = 3 1=3 0+20 = r=6 = GOSUB 180
110 PLOT x0-4,30-32=DRAW X0+2,30-32,0
120 '
130 FOR 3=-14 TO 14
140 PLOT X0-30,30-70+y = DRAW x.0+30,30-70-3 , 1
150 NEXT y
160 END
170 '---------------------------------------------------------------elliPse
180 h=rt2=l=rt2.3
190 FOR a=0 TO 2YPI+0.1 STEP 1.2/r
280 x=x1+<lz2 )*COS<a)"##########,
    "page 99"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_173,
    r##########"210 y=31+(h/2>*SIN<a)
220 PLÛT x1,y 1= DRAW x, y,ce
230 NEXT a
240 RETURN"##########,
    "page 99"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_174,
    r##########"10 ’----------------•-----------------------------Palmier
20 MODE 2
30 DEG
40 ORIGIN 300,20"##########,
    "page 100"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_175,
    r##########"30 MOVE 3,3
60 FOR a=l TO 20
70 FOR b= -30 TO 30 STEP 30
80 DRAW SIN< b)*LOG< 20-a+1)*8-10,COSC b)*8*L0GC20-a+1)+a*11"##########,
    "page 100"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_176,
    r##########"30 NEXT b
100 NEXT a
105 '----------------------------------------------
110 RAD
120 a$="Pascale"
130 b=LENCa$)
135 TAG
140 FOR c=l TO b
150 FOR d=30 TO 270 STEP 14
160 x=SINCd)*Cc*3)*3-10
170 y =COS(d)*Cc*2>*3+28*8
180 MOVE x,3:PRINT MID$Ca*,c,1);"##########,
    "page 100"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_177,
    r##########"130 NEXT d
200 NEXT c □ Araignée"##########,
    "page 100"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_178,
    r##########"20 MODE 1■INK 0,1 : INK 1,24
30 X0=300 ;Y0 =200
40 R=30
50 '
60 FOR M=1 TO 3 STEP 0.3
70 PLOT x0+r*m,30,l
80 FOR A=PI/4 TO 2*PI+0.1 STEP PI/4
90 X=R*M*COSC A )+X0
100 Y=R*M*SINCA)+Y0
110 DRAW X,Y,1
120 PLOT X0,Y0: DRAW X,Y
130 NEXT A
140 NEXT M
150 '
160 PLOT X0+R,YO+80,1DRAW X0+R,Y0+R
170 LOCATE 21,211 PRINT CHR.$C 225 )
180 LOCATE 21,22:PRINT CHR$C162 )"##########,
    "page 100"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_179,
    r##########"130 LOCATE 20, 22 PR I NT CHR$C 132); CHR$C 136); CHRÏC 133);
200 FOR a=l TO 100:NEXT
210 LOCATE 20,22:PRINT CHR$C133);CHRSC145);CHR$C132);
220 FOR a=l TO 100:NEXT
230 GOTO 130"##########,
    "page 100"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_180,
    r##########"20 MODE 2
30 INK 0,1:INK 1,24:PAPER 0 : PEN 1
40 hC 1 >=0.2 : h$C 1 >" tarte"
50 hC2)=0.3:h$C2)="croissant"
60 hC3)=0.1:h$C3 )="brioche" . . __
70 hC4)=0.3‘h$C4)="eclair" croissant^
80 h(5>0.1 :h«<5)="pain" / / .
90 ' l-------4--------i
100 xa=200: ya=200 . y \ J
110 r=60 brioche y Paln
120 ' "
130 PLOT xa+r,ya eclair
140 FOR a=0 TO 2*PI+0.1 STEP 0.1
150 x=xa+rtCOSCa)
160 y =y a+rtS I NC a ">
170 DRAW x,y,l
180 NEXT a
190 '----------------------------------------affichage texte
200 aa=8
210 FOR P=1 TO 5
220 a=aa*+P I *2thC P )
230 x=xa+rtCOSC a ): y =y a+r-tS I NC a ')
240 PLOT xa,ya:DRAW x,y,1
250 '
260 at=aa+PIthCP ) ' affichage texte
270 x=xa+rtl.3YC0SCat);y=ya+rtl.3#SINCat)
280 IF at>PI.-'2 AND at<3tPI/2 THEN x=x-8*LENC h$C P ) )
290 PLOT x,y;TAG:PRINT h$CP
300 3.3=3.
310 HEXT P □ Chronomètre"##########,
    "page 101"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_181,
    r##########"20 MODE 2
30 INK 0,1 = INK 1,24:PAPER 0 : FEN 1
40 X0=300 ;Y0=200:R=100 ' centre
50 ORIGIN x0,y0 ' redefinition origine
60 GOSUB 240
70 ----------------------------------------graduations
80 FOR A=0 TO 2*PI STEP P1/6
90 X=R*0. 8*C0S< A ■> : Y=R*0.8*S LNC A )
100 Xl=R*COSCA):Yl=RtSINCA)
110 PLÛT X,Y,1:DRAH X1,Y1,1
120 NEXT A
130 •'--------------------------------------------------mouvement ai9ui 11e
140 R=RY0.7
150 FOR A=100 TO 0 STEP -PI/30
160 x=rYCOSC a >: y =rtSINC a)
170 PLOT 0,0:DRAW x,y,l
180 SOUND 1,500,2
190 FOR TP=1 TO 120:NEXT TP
200 DRAW 0,0,0
210 NEXT R
220 END"##########,
    "page 101"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_182,
    r##########"233 '-----------------------------------------------------cercle
240 PLOT r,0
250 FOR R=0 TO 2*PI +0.2 STEP 0.05
260 DRRH rtCOSC artSINC a ">> 1
270 NEXT R"##########,
    "page 102"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_183,
    r##########"10 >----------------------------------------------triangle Plein
15 '
20 ’ On fait Pivoter une droite autour de PO entre PI et P2.
30 •’ Ru lieu de calculer Y=f<x),
40 ' on calcule X=f<D) et Y=f(D) avec D variant avec un Pas de 1
50 '
60 MODE 2
70 INK 0,1 = INK 1,24:PAPER 0 : PEN 1
80 x0=14O y 3=100 •’ 1er Pointe Pivot)
90 xl=210:y 1=230 ' 2eme Point
100 x2=120;y2=200 ' 3eme Point
110 '
120 d=SQR( < y2-y 1 )"’2+< x2-xl )'"'2 > ' distance entre PI et P2
130 IF d=0 THEN END
140 cx=( x2-x 1 )/d ■■ cy =< y 2—y 1 >/’d
150 FOR dd=0 T0 d ' on Parcourt la droite entre PI
160 x3=xl+ddtcx:y3=y1+dd^cy _ et P2
170 PLOT x0, yO, ce: DRRH x3,y3,’l"##########,
    "page 102"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_184,
    r##########"130 NEXT dd
190 RETURN"##########,
    "page 102"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_185,
    r##########"10 >--------------------------------------------------figures inscrites dans un cercle
20 ' 3Û MODE 2: INK 9,1 : INK 1,24 TAPER 9 TEN 1
40 xc=190:ac=109 ' centre
59 r=59 ' rayon
60 ncot«3 ’ nombre de cotes
70 GOSUB 140
80 ' ———————————————————
99 ncot=6’xc=200:GOSUB 140 ' hexagone
100 >------------------------------------------
119 ncot=15=xc=300=GOSUB 149 ' cercle
120 END
130 ,-------------------------------------------------
149 PLOT xc+r,yc
159 FOR c=l TO nccit
169 a=c£PIt2/ncot
179 x=xc+r^C0SCa)
180 y=yc+r*SIN(a)
190 DRAW x,y,l
200 NEXT c
210 RETURN □ Etoiles"##########,
    "page 103"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_186,
    r##########"18 -------------------------------------------------- trace d'etoiles
20 ■'
30 MODE 2 = INK 0, 1 = INK 1,24 TAPER 0 TEN 1
49 xc=190■yc=100 ' centre
59 r=50 ' rayon
69 ncot=5 ' nombre de branches
70 GOSUB 149
80 '------------------------------------------
99 ncot=7■xc=290:GOSUB 149
109 /------------------------------------------
119 ncot=15■xc=399:GOSUB 149
120 END
130 ,------------------------------------------------- sP9m
149 PLOT xc+r,yc
150 FOR c=l TO ncot
160 a=ct(ncot-l )*PI/ncot
179 x=xc+rtC0S< a )
189 y =y c+r$S I NC a )
190 DRAW x,y
290 NEXT c"##########,
    "page 103"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_187,
    r##########"210 RETURN
290 NEXT c"##########,
    "page 103"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_188,
    r##########"2000 '--------------------------------- recoPie d'écran sur imprimante RMSTRFID
2010 DEFINT a-z
2020 DIM 19(328)
2030 ln=399
2040 FOR 19=1 TO 30
2050 FOR x=0 TO 319
2060 l9(x)=0
2070 FOR P=0 TO 6
2080 IF TEST(x*2,ln-P*2)>0 THEN 19( x)=19( x)+(2ÆP )
2090 NEXT P
2100 NEXT x
2110 PRINT #3ÆHR$( 27 ); CHR*( 75 ); CHR$( 2 ); CHR$( 64 );
2120 FOR z=0 TO 319‘PRINT #8,CHR$(l9(z ) );=NEXT z
2130 PRINT #8,CHR$(19(320))
2140 ln=ln-14
2150 NEXT 19
2160 PRINT #8,CHR$(15) Pour obtenir une copie d'écran, faire : MERGE “HARDC” puis “RUN 2000” Il est prévu pour fonctionner avec PAPER 0. En revanche, la couleur d’écriture définie dans PEN n’a pas d'importance."##########,
    "page 104"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_189,
    r##########"16 + 8 + 4 = 28. Dans une instruction SYMBOL, on indique la suite des nombres obtenus dans l'ordre des lignes. I® '------------------— étoile
20 MODE 1
30 c=150 ' caractère a modifier
40 SYMBOL AFTER c
50 SYMBOL c,8,8,28,127,28,34,65,0
60 PRINT CHR$(c> SYMBOL AFTER ________________________________________________ “SYMBOL AFTER” spécifie le premier code des caractères à modifier. Ci-dessous une étoile traverse l’écran de gauche à droite, puis de droite à gauche. La vitesse s’adapte grâce à la temporisation en ligne 170. L'effacement se fait en impri­ mant un espace devant le caractère.
104 I BASIC AMSTRAD"##########,
    "pages 105, 106"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_190,
    r##########"10 •------------------ dePlacement etoile
20 MODE 1
30 INK 0.26 ' fond blanc
40 INK 1.0 ' encre noire
50 PAPER 0:PEN 1
60 c=145 ' caractère redéfini
70 SYMBOL AFTER c
80 SYMBOL c.8.8.23.127.28.34.65.0"##########,
    "page 106"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_191,
    r##########"30 •--------------------------- dePlacement etoile
100 y=10-xl=l;x2=36 ' bornes
110 s=l ' sens
120 '
130 FOR x=xl TO x2 STEP s
140 LOCATE x.3:PRINT CHR$<32CHR®(cCHR®<32
150 FOR tP=l TO 80-NEXT tP
160 NEXT X
170 x=xl:xl=x2:x2=x;s=-s ' inversion
180 GOTO 130 La forme ci-dessous est représentée avec deux caractères redéfinis."##########,
    "page 106"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_192,
    r##########"10 ’-------------------------chien JR
20 MODE 1
30 c=145
40 SYMBOL AFTER c
50 SYMBOL c.68.56.56.7.7.7.4. 12
60 SYMBOL c+1.0.4.3.240.240.240.16.48
70 ••
75 chienS=CHR$(c )+CHR«(c+1 )
80 LOCATE 10.10:PRINT chien® Utilisation de TAG : Pour positionner un caractère avec le curseur graphique, on utilise TAG. Naturelle­ ment, le positionnement est plus précis qu’avec LOCATE. Exemples : □ Ciel étoilé"##########,
    "page 106"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_193,
    r##########"10 •’----------------------ciel étoile
20 MODE 1
30 INK 0.26 ‘ fond blanc
48 INK 1.0 ' encre noire
50 PAPER 0 : PEN 1
60 c=125 ' caractère redefini
70 SYMBOL AFTER c
80 SYMBOL c.8.8.28.127.28.34.65.0
80 TAG
100 FOR e=l TO 100
110 x=RND<l)*500
120 3=RNDC1)*300
130 MOVE x.3
140 PRINT CHRïCc);
150 NEXT e"##########,
    "page 106"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_194,
    r##########"10 ’----------------------------------- dePlacement etoile (avec TAG)
20 MODE 1
30 INK 0,26 ‘ -Eond blanc
40 INK 1,0 ' encre noire
50 PAPER 0 = PEN 1
60 c=125 ' caractère redefini
70 SYMBOL AFTER c
80 SYMBOL c,8,8,28,127,28,34,65,0"##########,
    "page 107"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_195,
    r##########"30 '--------------------------- dePlacement etoile
100 TAG
110 FOR x=l TO 560 STEP 2
120 MOVE x,200
130 PRINT CHR$(32);CHR$(c),-
140 NEXT x □ Formes de 16 x 16 points Pour représenter des formes de 16 x 16 points, il faut redéfinir 4 caractères. Ci-dessous, nous représentons une locomotive : CHR$(10) provoque un saut de ligne, CHR$(8) décale le curseur vers la gauche."##########,
    "page 107"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_196,
    r##########"10 '----------------------------------------locomotive
20 •’
30 MODE l’INK 0,1=INK 1,24
40 C=145 ' 1er caractère a modifier
50 SYMBOL AFTER C
60 SYMBOL c,0,0,6,0,16,0,48,48
70 SYMBOL c+1,0,0,0,0,254,138,254,254
80 SYMBOL c+2,127,127,127,127,255,255,48,48
90 SYMBOL c+3,254,254,254,254,255,255,24,24
100 l$=CHR$(c)+CHR$( c+1)+CHR$(10 )+CHR$(8 )+CHR$( 8)+CHR$(c+2)+ CHRS(c+3 )+CHRS(11 )
110 LOCATE 10,2:PRINT 1$
120 t$=l$+l$ ' train
130 LOCATE 10,10:PRINT t$
140 ■■------chai ne effacement
150 ef$=CHR$(32 )+CHR$(10 )+CHR$(8 )+CHR$(32 )
160 '-------------------------3.-j3.r\ce locomotive
170 3=18
180 FOR x=24 TO 1 STEP-1"##########,
    "page 107"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_197,
    r##########"106 I BASIC AMSTRAD
200 FOR tP = l TO 100:NEXT tP
210 NEXT x
220 '-----------------------------avance train
230 3=23
240 FOR x=24 TO 1 STEP-1
250 LOCATE x,3=PRINT t«;ef«
260 FOR tP = l TO 100*NEXT tP
270 NEXT x Pour déplacer la locomotive plus progressivement qu’avec LOCATE, nous utilisons TAG. La chaîne “h$” représente la partie supérieure de la locomotive et “b$” la partie inférieure."##########,
    "page 108"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_198,
    r##########"10 ■---------------- dePlacement locomotive avec TPG
20 '
30 MODE 1: INK 0,1: INK 1,24
40 0=145 •' 1er caractère a modifier
50 SYMBOL AFTER C
60 SYMBOL c, 0,0,6,0,16,0,48,43
70 SYMBOL c+1,0,0,0,0,254,138,254,234"##########,
    "page 108"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_199,
    r##########"30 SYMBOL c+2,127,127,127,127,255,255,48,48
30 SYMBOL c+3,254,254,254,254,255,255,24,24
100 •'
110 h$=CHR«<c)+CHR$<c+l >+CHR$(32> ‘ haut
120 b«=CHR$(c+2 )+CHR$(c+3>+CHR$<32> ‘ bas
130 3=100
140 TAG
150 FOR x=500 TO 1 STEP -2
160 MOVE x,3‘PRINT h*;‘MOVE x,3-14PRINT b$;
180 NEXT x"##########,
    "page 108"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_200,
    r##########"10 '------------------GENERATEUR DE CARACTERES 16*16
20 MODE 1
30 DIM t<18.18)
40 FOR x=l T0 18-LOCATE x, 1‘PRINT "x"‘LOCATE X/18‘PRINT "x"'NE XT x ’
50 FOR y = l TO 18‘LOCATE La‘PRINT "x"‘LOCATE 18.y‘PRINT "x"‘NE XT a
60 LOCATE 1.20‘PRINT "FLECHES POUR DEPLACER"
70 LOCATE 1.21‘PRINT "L‘LEVER B =BAISSER F‘FIN"
80 x=10‘y=10
90 ‘------------------
100 cS=INK.EYS‘IF c$<>"" THEN 160
110 LOCATE x.a‘PRINT CHR$(143)
120 FOR tP=l TO 10‘NEXT tP
130 LOCATE x.y‘PRINT CHRS<32)
140 GOTO 100
150 ■'
160 IF cs=0 THEN LOCATE x.a‘PRINT CHRSK 143>‘t<x-1.a-1>=1
170 IF cs=l THEN LOCATE x.a‘PRINT CHR$(32)‘t<x-1.y-1>0
180 •’"##########,
    "page 109"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_201,
    r##########"130 c$=UPPER$<c$ >
200 c=ASC(c$)
210 IF c=242 THEN IF x>2 THEN x.=x-l
220 IF c=243 THEN IF x<17 THEN x=x+l
230 IF c=241 THEN IF Y<17 THEN y=y+l
240 IF c=240 THEN IF Y>2 THEN y=y-l
250 IF c«="L" THEN cs=l
260 IF c$="B" THEN cs=0
270 IF c$="F" THEN 300
280 GOTO 100"##########,
    "page 109"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_202,
    r##########"300 FOR CL=1 TO 2 '2 colonnes
310 FOR L=1 TO 16 ' 16 lignes
320 PS=<CL-1)*8
330 ND=0 ' valeur decimale
340 FOR X=1 TO 8 '1 caractère
350 A—0’IF T<X+PS,L>=1 THEN R=1
360 ND=ND+A*2~< 8-X)
370 NEXT X
380 LOCATE 20+CL*5,L> PRINT ND;SPC(1)
390 NEXT L
400 NEXT CL
410 GOTO 100 □ Le programme ci-dessous représente une maison."##########,
    "page 110"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_203,
    r##########"10 '----------------------------------------maison
20 ' ®
30 MODE 1 ’ INK 0,1’INK 1,24’PAPER 0’PEN 1
40 C=125 ' caractère a modifier
50 SYMBOL AFTER C
60 SYMBOL c, 12, 12,63,127,255,64,95,85
70 SYMBOL c+1,0,0,248,252,255,2,2,2
80 SYMBOL c+2,95,85,85,85, 95,64,64,127
90 SYMBOL c+3,122,74,74,106,74,74,74,254
100 mS=CHRS(c >+CHR$(c+1)+CHR$(10>+CHR$C8 >+CHR$(8>+CHRS<r+2 )+ CHRSKc+3)
110 PRINT mS □ Celui-ci des immeubles dont la hauteur est aléatoire."##########,
    "page 110"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_204,
    r##########"10 ,----------------------------- IMMEUBLES
20 '
30 MODE l’INK 0,1’INK 1,24’PAPER 0’PEN 1
40 C=125 ' caractère a modifier
60 SYMBOL AFTER C
70 SYMBOL C, 255,129,129,129,129,129,129,255
75 c»=CHRS<c)
80 '
90 XB=1’YB=23
100 ‘
110 FOR N=1 TO 5 ' 5 immeubles
130 H=RND<1>*5+5’L=RND<1>*3+3
140 FOR Y=YB TO YB-H STEP-1
150 FOR X=X8 TO XB+L
160 LOCATE X,Y’PRINT C$
170 NEXT X
180 NEXT Y
190 XB=XB+L+2
200 NEXT N"##########,
    "page 110"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_205,
    r##########"10 INPUT "Frequence "if
20 P=125000/f
30 SOUND 1.P.100
40 9oto 10
110 I BASIC AMSTRAD L’instruction SOUND ne bloque pas l’exécution du programme. Plusieurs sons peuvent être joués simultanément sur trois canaux (A, B, C)."##########,
    "pages 111, 112"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_206,
    r##########"10 SOUND 1,125,100 ' canal R 1300 HERTZ Pendant 1 seconde
20 SOUND 2,500,200 ' canal B 250 HERTZ Pendant 2 secondes
30 PRINT "SUITE" Avec une seule commande SOUND, on peut jouer la même note sur plusieurs canaux grâce à un système de codage. A-» 1 B—>2 C —> 4 Exemple : Pour jouer sur A et C, on code 5 (1+4) dans l’instruction SOUND. Chaque canal dispose d’une file d’attente pouvant comporter jusqu’à quatre commandes SOUND. La synchronisation entre des canaux se spécifie en codant : Synchronisation avec A 8 Synchronisation avec B 16 Synchronisation avec C 32 Attente 64 Libère 128 Période : La période spécifiée dans SOUND doit être comprise entre 0 et 4095. 0 spécifie l’ab­ sence de fréquence. Durée : La durée qui est par défaut 20 (0.2 sec.) doit être comprise entre -32768 et +32767. Si la durée spécifiée est nulle, la durée est fournie par la commande ENV. Une valeur négative donne le nombre de répétitions de l’enveloppe de volume. Volume : Il doit être compris entre 0 et 15. Sans enveloppe de volume, il est égal à 4 par défaut et doit être compris entre 0 et 7. Avec enveloppe de volume, il est égal à 12 par défaut. Enveloppe de volume : Un son peut être “modulé” en amplitude par une enveloppe (de période plus impor­ tante). Cf. instruction ENV. Enveloppe de ton : Cf. instruction ENT."##########,
    "page 112"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_207,
    r##########"2000 ENV 1, 5,2,20, 1,0,200, 10,-1,20
2010 SOUND 1,125,500,1,1 ENT numéro, nombre pas, pas période, temps, nombre pas, pas période, temps... _________________________________________________________ Cette instruction permet de faire varier la fréquence de l'instruction SOUND pendant son exécution."##########,
    "page 113"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_208,
    r##########"2000 ENT 1, 50,2,3, 50,-2,3
2010 SOUND 1,25,300,15,0,1 sec Exemples divers : □ Génère des sons aléatoires."##########,
    "page 114"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_209,
    r##########"10 SOUND 1,RND<1>*400,3,15
20 GOTO 10 □ Tir de laser."##########,
    "page 114"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_210,
    r##########"10 FOR P = 1 T0 100 STEF
20 SOUND 1,P, 1
30 NEXT P □ Sirène de pompier."##########,
    "page 114"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_211,
    r##########"10 '-------------------PomPier
20 SOUND 1,200,50
30 SOUND 1,100,50
40 GOTO 20 □ En appuyant simultanément sur A, S et D, la note “DO” est jouée avec 3 octaves différentes."##########,
    "page 114"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_212,
    r##########"10 Joue sur 3 canaux une note sur 3 octaves avec touches A,S,D
20 '
30 ' aPPuaer sur Fl seul,S seul,D seul
40 ' Puis R et S simultanément
50 ’
60 nt=473 ' note
70 d=5 ’ duree
80 •’
90 IF INKEYC 69 >0 THEN SOUND l,nt,d
100 IF INKE’ï’(60)=0 THEN SOUND 2,nt/2,d
110 IF INKEY(61)=0 THEN SOUND' 4,nt/4,d
120 GOTO 90 SQ (canal) _______________________________________________________ Donne l’état d’un canal."##########,
    "page 114"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_213,
    r##########"18 SOUND 1>125>100 1 seconde
20 PRINT SQ< 1 )>
30 GOTO 20"##########,
    "page 115"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_214,
    r##########"10 SOUND 1>125>100 1 seconde
20 IF <SQ(1) HND 128)0128 THEN SOUND 2.. 250 > 100 = END
30 GOTO 20 ON SQ (canal) GOSUB n° ligne_____________________________________ Provoque un branchement au numéro de ligne spécifié lorsqu'il y a une place libre dans le canal spécifié."##########,
    "page 115"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_215,
    r##########"10 SOUND 1>125>200
20 SOUND 1>250>100
21 SOUND 1>500>100
22 SOUND 1>1000>100
23 SOUND 1>2000>100
24 SOUND 1>4000>100
30 ON SQC1 ) GOSUB 50
40 GOTO 40
45 •'-------------------------------
50 PRINT "Il y a une Place libre Pour le canal 1"
60 RETURN RELEASE n° canal ________________________________________________ Libère un canal en attente. RELEASE 2 ► 2 RELEASE 3 ► 1+2 Exemples divers : □ Ci-dessous, des notes sont choisies au hasard.
114 I BASIC AMSTRAD"##########,
    "pages 115, 116"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_216,
    r##########"10 ‘---------------------------------MUSIQUE ALEATOIRE
20 DATA 4186,do
30 DATA 4698,re
40 DATA 5274,mi
50 DATA 5587,fa
60 DATA 6271,sol
70 DATA 7040,la
80 DATA 7802,si
90 >
100 FOR n=l TO 7
110 READ f<n),x$
120 NEXT n 130
140 nt=INT<RNDC1)*7>+l ’ note
150 dur®INT <RND(1)$60 )+10 •' duree
160 oct=INT<RND( 1 )t3)+l ■' octave 170
180 SOUND 1,125000/fC nt >#4/oct,dur
190 GOTO 140 Les fréquences données dans le programme sont celles du niveau d’octave 4. On trouvera en annexe un tableau complet des fréquences pour les niveaux d’octaves allant de -1 à 4. □ Le programme ci-dessous joue une note frappée au clavier. A chaque touche cor­ respond une note avec un niveau d'octave. La note jouée est de durée fixe. Nous avons augmenté le délai de répétition pour les touches afin d'éviter qu’une note ne soit jouée deux fois de suite malencontreusement."##########,
    "page 116"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_217,
    r##########"10 JOUE 1 NOTE A LA FOIS DE DUREE FIXE
20 MODE 2:DEFINT a-Z
30 SPEED KEY 30>20
40 LOCATE 1,20:PRINT "q w e r t 3 u OCTAVE 4"
50 LOCATE 1,21=PRINT "a s d f 9 h J OCTAVE 3"
69 '--------------------------------------octave 4
70 DIM f(21 >,cl$(21 )
80 DATA 4186,do,Q, 4698,re,H, 5274,mi,E
90 DATA 5587,fa,R, 6271,sol,T, 7040,la,Y
100 DATA 7902,si,U
110 '----------------------------------------octave 3
120 DATA 2093,do,A, 2349,re,S, 2637,mi,D
130 DATA 2793,fa,F, 3135,sol,G, 3520,la,H
140 DATA 3951, si,.J
150 ’
160 FOR n=l TO 14-READ fCn),x$,cl$<n) ‘f(n>=125000/f<n):NEXT n
170 '-------------------------------
180 c$=INKEY$:IF c$="" THEN 180
190 c$=UPPER$(c$)
200 FOR i = l TO 14
210 IF c$=cl$(I) THEN SOUND l,f(i),15
220 NEXT i
230 GOTO 180 Le temps de recherche de la note correspondant à la touche frappée peut être supprimé en utilisant les valeurs ASCII des touches pour déterminer l’adresse de rangement des notes dans la table f()."##########,
    "page 116"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_218,
    r##########"163 FOR n=l TO 14‘READ f, x$,c$;f(RSCCc$ ) )=125000/f■NEXT n
170 >-------------------------------
180 c5=INKEY$‘IF c$="" THEN 180
190 c«=UPPER$<C$>
210 c=ASC(cS)‘ SOUND l,f(c>, 15
230 GOTO 180 □ Le programme ci-dessous permet de jouer trois notes de durée variable simultané­ ment. Lorsque l’opérateur appuie sur une touche de façon permanente, toutes les commandes SOUND pour une touche pendant la durée d’une note doivent être envoyées vers le même canal. Un simple test de la disponibilité des canaux avec SQ n’est donc pas suffisant ; une même note serait envoyée sur plusieurs canaux."##########,
    "page 117"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_219,
    r##########"10 '---------------- JOUE 3 NOTES SIMULTANEMENT sur canal 1 et 2 et 4
20 MODE 2‘DEFINT a-z
30 LOCATE 1,20‘PRINT "4 w e r t y u. octave 4"
40 LOCATE 1,21‘PRINT "a s d f 9 h J octave 3"
50 '--------------------------------------octave 4
60 DIM f(21),cl(21),c<21)
70 DATA 4186,do,67, 4698,re,59, 5274,mi,58
80 DATA 5587,fa,50, 6271,sol,51, 7040,la,43
90 DATA 7902,si,42
100 ‘----------------------------------------octave 3
110 DATA 2093,do,69, 2349,re,60, 2637,mi,61
120 DATA 2793,fa,53, 3135,sol,52, 3520,la,44
130 DATA 3951,si,43
140 •
150 FOR n=l TO 14‘READ fCn),xi,cl<n)‘f(n)=125000/f(n)‘NEXT n
160 '-------------------------------
170 FOR i = l TO 14
180 IF INKEYCcl(i))=0 THEN 230
190 IF c(i)<>0 THEN cl ib( c< i > )=0 ‘ cC i )=0
200 NEXT i
210 GOTO 179
220 •’
230 IF c<i)<>0 THEN c=cCi)‘SOUND c,f(i ),15‘GOTO 200
240 IF clibCl>=0 THEN c<i )=1‘clib<1 )=1‘SOUND 1,f(i),15■GOTO 200
250 IF clib<2)=0 THEN c(i )=2‘clib(2)=1‘SOUND 2,fCi),15■GOTO 200
255 IF clib<3)=0 THEN c<i )=4■clib(3 >-l‘SOUND 4,fCi),15‘GOTO 200
260 GOTO 200"##########,
    "page 117"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_220,
    r##########"13 '----------------hor* l o9e
28 CLS
30 h=TIME
40 LOCATE 1,22
50 PRINT (TIME-h)/300
60 GOTO 40 L'exécution différée ou périodique de sous-programmes se programme avec les ins­ tructions AFTER (APRES) et EVERY (CHAQUE). C’est BASIC qui gère les chrono­ mètres associés aux sous-programmes. AFTER temps, n° chrono GOSUB n° ligne Provoque l'exécution d’un sous-programme spécifié après le temps spécifié en"##########,
    "page 119"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_221,
    r##########"10 >----------------AFTER
20 MODE 1
30 AFTER 100,0 GOSUB 30 ' aPr-es 2 secondes
40 '
50 LOCATE 1,2'PRINT i:i = i + l:GOTO 50 ' boucle d'attente
60 END"##########,
    "page 120"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_222,
    r##########"30 '------------------------------------------------
90 LOCATE 1,22:PRINT "coucou."
100 RETURN EVERY temps, n° chrono, GOSUB n° ligne Provoque l’exécution périodique du sous-programme spécifié. Ci-dessous, l’heure est affichée toutes les secondes."##########,
    "page 120"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_223,
    r##########"10 ---------------------EVERT
20 NODE 1
30 hd=TIME > heure depart
40 EVERT 50,0 GOSUB 90 •’ toutes l*=’S seron
50 ' - -­
60 GOTO 60 ' boucle d'attente
70 END
80 ' at'fichaâe heure
90 LOCATE 1,22:PR INT INTC<TIME-hd>/300)
100 RETURN Ci-dessous, un danseur est représenté dans quatre positions."##########,
    "page 120"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_224,
    r##########"10 MODE 1
20 EVERY 60,0 GOSUB 140 ' 3 secondes horloée 0
30 EVERY 200,1 GOSUB 118 ' 10 secondes horlo9e 1
40 '
50 FOR c=24S TO 251 '4 Positions
60 LOCATE 20,15-PRINT CHR$<c>
70 SOUND 2,INT(RND*1OO >+50,20
80 NEXT c
90 LOTO 50
100 ’-----------------------chrono 1
110 LOCATE 13, 15: PRINT "3rrlUrf dance"
120 RETURN
130 '-----------------------chrono 0
140 LOCATE 13,15:PRINT "smurf DANCE"
150 RETURN Toutes les 3 secondes, un message est affiché en majuscules. Toutes les 10 secondes, le même message est affiché en minuscules. REMAIN (chrono) ________________________________________________ Annule le chronomètre spécifié et donne le temps qui restait. PRINT REMAIN(O)"##########,
    "page 120"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_225,
    r##########"30 OPENOUT "FICH"
40 ■'
50 INPUT "Norn Cou FIN) 'Unornî
60 IF nomî="FIN" THEN CLOSEOUT:GOTO 130
70 INPUT "Telephone ";tPh$
80 '
90 PRINT #9,nom$
100 PRINT #9>tPh«
110 GOTO 50
122 I BASIC AMSTRAD Pour lire les enregistrements, on écrit :
180 INPUT #9,ïiom$
190 INPUT #9,tPh$ On peut également écrire : INPUT # 9,N0M$, TPH$. ATTENTION :"##########,
    "pages 123, 124"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_226,
    r##########"10 PRINT #9,nom$
20 PRINT #9,tPh« ne doit pas être remplacé par : PRINT # 9, NOM$, TPH$ ; les valeurs seraient seule­ ment séparées par des espaces et seraient considérées à la lecture comme une seule valeur. R R R DUPONT 044.22.63 MARTIN 955.19.01 C C C Avec INPUT # 9, une virgule dans une chaîne est considérée comme séparateur. Par conséquent, les chaînes comportant des virgules doivent être écrites par WRITE # 9 ou être lues par INPUT # 9 qui ne considère comme séparateur que le retour-chariot. EOF ____________________________________________________________ EOF teste la fin d’un fichier en lecture. Cette instruction doit être programmée avant la lecture par INPUT # 9."##########,
    "page 124"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_227,
    r##########"160 IF EOF=-1 THEN CLOSEIN:END
170 '
180 INPUT #9,nom$ CLOSEOUT CLOSEIN _______________________________________________________ CLOSEOUT provoque le transfert sur cassette de la mémoire tampon. CLOSEIN ferme un fichier en lecture. LINE INPUT # 9, chaîne __________________________________________ Lit une chaîne de caractères dans un fichier en ne considérant comme séparateur que le retour-chariot (code 13). La virgule n'est pas considérée comme séparateur."##########,
    "page 124"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_228,
    r##########"100 rueS="11.rue NOBEL";mt=164553
110 WRITE #9.rueS.mt “11,rue NOBEL", 164 553 Les chaînes comportant des virgules peuvent ainsi être lues par INPUT # 9. □ Le programme ci-dessous crée un annuaire téléphonique et le lit."##########,
    "page 125"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_229,
    r##########"10 '-----------------------------fichier
20 •'-------------------------------écriture
30 OPENOUT "FICH"
40 •'
50 INPUT "Nom Cou FIN) ";nom$
60 IF •nom$="FIN" "THEN CLOSEOUT•GOTO 130
70 INPUT "TelePhone "itPhS"##########,
    "page 125"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_230,
    r##########"30 •’
90 PRINT #9,nomS
100 PRINT #9,tPhS
110 GOTO 50
120 '-----------------------------lecture
130 PRINT "Rembobinez"
140 OPENIN "FICH"
150 •'
160 IF EOF=-1 THEN CLOSEIN’END ' fin de fichier?
170 '
180 INPUT #9.. noms
190 INPUT #9,tPhS
200 PRINT nom$,tPhS
210 GOTO 160 Ready run Nom (ou FIN) ? DUPONT Téléphone ? 044—22—63 Nom (ou FIN) ? MARTIN Téléphone ? 999-88-77 NOM (ou FIN) ? FIN Press REC and PLAY then any key : Saving FICH block 1 Rembob i nez Press PLAY then any key : Loading FICH block 1 DUPONT 044-22-63 MARTIN 999-88-77 □ Le programme ci-dessous sauvegarde une table sur cassette."##########,
    "page 125"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_231,
    r##########"10 ••-------------------------------SAUVEGARDE D'UNE TABLE
29 DIM At 200)
30 OPENOUT "TB"
40 '
50 FOR 1=1 TO 200
60 A<I)=I
70 PRINT #9, AC I>
80 NEXT I
90 CLOSEOUT"##########,
    "page 126"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_232,
    r##########"110 OPENIN "TB"
120 FOR 1=1 TO 200
130 IHPUT#9,ACI)
140 PRINT a(i)
150 NEXT I
160 CLOSEIN Un programme sauvegardé en ASCII peut être lu comme un fichier séquentiel."##########,
    "page 126"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_233,
    r##########"10 >-------------------------TRACE PAR SEGMENTS DE DROITES
20 ce=l:ct=0
30 MODE 1
40 INK 0,26:INK 1,0 : PAPER 0 : PEN 1
50 LOCATE 1,20’PRINT "PREMIER POINT ■ flèches Puis 'V'"
60 LOCATE 1,21: PR INT "AUTRES POINTS: P leches Puis •'D-'"
70 x=200:a=200 sa ,------------------------------------CURSEUR CLIGNOTANT
90 t=TEST(x,y>
100 '
110 c$=INKEY$:ip c$<>”" THEN 160
123 PLOT x,y,ce
130 PLOT x,y,c+"
140 GOTO 110
150 ''
160 PLOT x,y,t
170 c=ASC(c$)
175 c$=UPPER$<c$)
180 IF C=242 THEN X=M-2
190 IF C=243 THEN X=X+2
200 IF 0=240 THEN ï=v+2
210 IF C=241 THEN Y=Y-2
220 LOCATE 1,17‘PRINT c$
230 IF C$="V" THEN PLOT X,Y, CE‘GOSUB 270 ■■ XR=X = YR=Y
240 IF C$="D" THEN PLOT XR,YR,CE=GOSUB 270‘DRRW X,Y,CE‘YA=Y‘ XR=X
250 GOTO 90 2g0 >------------------AFFICHAGE X,Y
270 LOCRTE 1,14‘PRINT XjY‘RETURN DESSINATEUR __________________________________________________ Ce programme “dessinateur” permet de représenter des droites, des rectangles pleins, des cercles pleins ainsi que des triangles pleins. Pour dessiner une droite par exemple, il faut appuyer sur “V” pour valider le premier point puis déplacer le curseur avec les flèches (—>, , ! , î ) et appuyer sur “D” pour obtenir le tracé entre les deux points. Un cercle se représente en appuyant sur “V” pour définir le centre, puis en déplaçant le curseur jusqu’à un point de la circonférence et en appuyant sur “C” pour le valider. Le tracé du cercle est alors effectué."##########,
    "pages 128, 129"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_234,
    r##########"355 IF C$ = “S” THEN SAVE...
356 IF C$ = “L” THEN LOAD... C PREMIER POINT : fléchés pui ’ y 2EME POINT: fléchés puis: D : droi te R : rectan _________ Y: tri an g e<U pour 2 points) COULEUR: 1,2,3 ”F : FOND (GOMMER) Le signe “A” représente la flèche “î”."##########,
    "page 130"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_235,
    r##########"10 >---------------------------------DESSINATEUR
29 ce=l>cf=0 ' coal écriture et fond
30 INK 0,26;INK 1 , 0 = PAPER cf:PEN ce ‘ fond blanc/encre noire
40 SPEED KEY 10,3 ' rePetition rapide
50 MODE 1
60 LOCATE 1,20:PRINT "PREMIER POINT ■■ f leches Puis '7'"
70 LOCATE 1,21 :PRINT "2EME POINT: fleches Puis:"
80 LOCATE 3,22:PRINT "D=droite R;rectangle"
90 LOCATE 3,23:PRINT "C=cercle Y!trian91e<V Pour 2 Points)"
100 LOCATE 3,24:PRINT "COULEUR: 1,2,3 F : FOND (GOMMER)"
110 LOCATE 3,25:PRINT "A:annulation derniere droite"
120 xa=200:aa=300 ' Point Precedent
130 xb^xa: yb=ya :x=xa+20:y=ya
140 >------------------------------------ CURoEUR CLIGNOTANT
150 t=TEST(x,y)
160 '
170 c$=INKEY$:IF c$<>"" THEN 210
180 PLOT x,y,ce:PLOT x,y,cf
190 GOTO 170
200 '
210 PLOT x,y,t
220 c=ASC(c$)
230 IF C=242 THEN IF x>2 THEN X=X-2: GOTO 150
240 IF C=243 THEN IF x<638 THEN X=X+2:GOTO 150
250 IF C=240 THEN IF y<398 THEN Y=Y+2:GOTO 150
260 IF C-241 THEN IF y>2 THEN Y=Y-2: GOTO 150
270 c$=UPPER$(c$):LOCATE 1,17:PRINT
280 IF C$="7" THEN PLOT x,y,ce :xb=xa:yb=ya :xa=x: ya=y
290 IF C$="D" THEN PLOT xa,ya,ce:DRAW x,y,ce;xb=xa:yb=ya:ya=y; xa=x
300 IF C$="C" THEN GOSUB 430
310 IF c$="R" THEN GOSUB 380>xa=x>ya=y
320 IF cî="Y" THEN GOSUB 510 = x.b=xa : yb=y a : xa=x = y a=y
330 IF c$="A" THEN PLOT xb,yb:DRAW xa,ya,cf:x=xb;y=yb■xa=xb:ya =yb
340 IF C$="F" THEN CE=0
350 IF C$>="0" AND C$<="9" THEN CE=7AL(CS)
360 GOTO 150
370 '---------------------------rectangle Plein
380 FOR yl=ya TO y STEP SGN(y-ya)
390 PLOT xa, y 1, ce:DRAW x, y 1, ce
400 NEXT yl
410 RETURN
420 '----------------------------------------------Cercle Plein
430 r=SQR( C xa-x )z'2+< y a-y )^2 ): r2=rz'2
440 FOR dx=-r TO r
450 dy =SQR( r-2-< dxz'2 ) )
460 PLOT xa+dx,ya+dy,ce:DRAW xa+dx,ya-dy,ce
470 NEXT dx
480 RETURN
490 ‘----------------------------------------------triangle Plein
500 ‘ valider 2 Points avec 'V' Puis un troisième avec 'Y?
510 d=SQR< ( y b-y a )*2+( xb-xa )A2 )
520 IF d=0 THEN RETURN
530 cx=(xa-xb)xd:cy=(y a-y b )/d
540 FOR dd=0 TO d STEP 0.5
550 x3=xb+ddtcx:y3=yb+ddtcy
560 PLOT x,y,ce;DRAW x3,y3,ce
570 NEXT dd
580 RETURN"##########,
    "page 131"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_236,
    r##########"10 '--------------------------------------------------carte de france
20 MODE 2'INK 0?1:INK 1?24'PAPER 0 TEN 1
30 D'ATA 276?370
40 DATA. 328? 342? 398? 318? 380?280
50 DATA 380?260? 364? 240? 364?234
60 DATA 372?234? 376? 240? 386?240
70 ô ■X >- ■r co r- CO ? ou O-J G 384?204? 384?196
80 DATA 396?188? 406?186? 406?174
90 DATA 384?166? 366?156? 352?166
100 DATA 332?170? 318?164? 308?150"##########,
    "page 132"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_237,
    r##########"120 DATA 196?156? 208?188? 212?218
130 DATA 206?236? 188?250? 174?272
140 DATA 164?282? 146?290? 132?290
150 DATA 132?306? 168?310? 182?304
160 O 'X >- il en CO v.l ç? CM 186?314? 184?334
170 DATA 200?334? 204?328? 222?322
180 DATA 234?322? 238?330? 242?338
190 DATA 256?346? 262?362? 276? 370
200 DATA 999?999 210
220 READ xa ? y a'PLOT xa ? y a 230
240 READ x?y
250 IF x=999 THEN END 260
270 DRAW x?y?l
280 GOTO 240 Pour tracer une figure discontinue, nous spécifions une coordonnée fictive égale à 0,0."##########,
    "page 132"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_238,
    r##########"50 MODE 1
60 DATA 100?100
70 DATA 150?100? 150?120
80 DATA 0?0"##########,
    "page 132"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_239,
    r##########"30 DATA 200?200
100 DATA 250?200? 250?240
110 DATA 999?999
120 '
130 READ xa?ya ' Premier Point
140 PLOT xa?ya
150 '
160 READ x?y
170 IF x=333 THEN END
180 IF x=0 AND y=0 THEN READ xa?ya'PLOT xa?ya'G0T0 160
190 DRAW x?y
200 GOTO 160"##########,
    "page 132"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_240,
    r##########"10 >--------------------------------------------INTERROGATION GEOGRAPHIE NO 1
20 MODE 1
30 INK 0,4=INK 1,24=PAPER 0 = PEN 1
40 DIM XVC30 ) ,YV< 30 ),30 >
50 DATA 276,370
60 DATA 328,342, 398,318, 380,280
70 DATA 380,260, 364,240, 364,234
80 DATA 372,234, 376,240, 386,240
90 DATA 378,226, 384,204, 384,196
100 DATA 396,188, 406,186, 406,174
110 DATA 384,166, 366,156, 352,166
120 DATA 332,170, 318,164, 308,150
130 DATA 306,136, 300,1324 ,236, 148
140 DATA 196,156, 208,188, 212,218
150 DATA- 206,236, 188,250, 174,272
160 DATA 164,232, 146,290, 132,290
170 DATA 132,306, 168,310, 182,304
180 DATA 198,302, 186,314, 184,334
190 DATA 200,334, 204,328, 222,322"##########,
    "page 133"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_241,
    r##########"132 I BASIC AMSTRAD
200 DATA 234,322, 238,330, 242,338
210 DATA 256,346, 262,362, 276,370
220 DATA 993,993
230 '
240 DATA 276,302, PARIS-
250 DATA 286,348,LILLE
260 DATA 314,314,REIMS
270 DATA 242,324,LE HAVRE
280 DATA 196,328,CHERBOURG"##########,
    "page 134"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_242,
    r##########"230 DATA 142,302,BREST
300 DATA 186, 288, RENNES-
310 DATA 186,264,NANTES
320 DATA 230,254,POITIERS
330 DATA 214,206,BORDEAUX
340 DATA 336,214,LYON
350 DATA 334,180,AVIGNON
360 DATA 358,168,MARSEILLE
370 DATA 999,339,ZZZZ
380 '"##########,
    "page 134"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_243,
    r##########"330 READ xa,ya=PLOT xa,ya,l Quelle ville ?
400 '
410 READ x,y=IF x=333 THEN 450
420 DRAW x,y,l
430 GOTO 410
440 ' —
450 READ x,y,v$=IF x=399 THEN 430
460 nv=nv+1■xv(nv )=x=y v(nv)=y=v$(nv )=v$
470 GOTO 450
480 '--------"##########,
    "page 134"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_244,
    r##########"430 v=INT(RNDC 1 >*nv )+l ' ville au. hasard
500 IF v=av THEN 490
510 av=v
520 PLOT xv<v>,yv< v), 1
530 LOCATE 5,20=1 NPUT "Que lie ville " ; v$
540 v«=UPPER$(v$ )
550 LOCATE 5,21
560 IF v$(v)=v$ THEN PRINT "OK " ELSE PRINT "NON ,C'est ",;v$(v
570 FOR tP = l TO 2000=NEXT tP
580 PLOT xvCv),y v(v ), 0
590 LOCATE 5,21=PRINT SPCC30)
600 LOCATE 5,20 = PRINT SPCC30 )
610 GOTO 490"##########,
    "page 134"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_245,
    r##########"480 '--------
485 ' ajouter les instructions 10-470 du ProSramme Precedent 49Û x=276=y=302
500 SPEED KEY 10,2
510 '
520 v=INT<RND<1)*nv)+l ' ville au hasard
530 IF v=av THEN 520 ELSE av==v
540 LOCATE 5,20=PRINT "OU EST SITUE "jV$(V)
553 '---------------------------------------------------Gestion curseur
560 LOCATE 5,22=PRINT "Fleches Puis 'V'"
570 '
580 t=TEST(x,y)
590 '
600 c«=INKEY$=IF c$<>"" THEN 650
610 PLOT x,y,1
620 PLOT x,y,Q
633 GOTO 600
640 '
650 PLOT x,y,t
660 '
670 c=ASC<c$)
680 IF c=242 THEN x=x-2
690 IF c=243 THEN x=x+2
700 IF c=240 THEN y=y+2
710 IF c=241 THEN y=y-2 720
730 IF C$="V" OP. C$="v" THEN 760
740 GOTO 580
750 '--------------------------- calcul distance
760 dx=xv< v >-x ■■ dy =y v( v )-y
770 d=SQR(dx*dx+dy*dy>
780 LOCATE 5,23’PRINT "Vous etes a’";d*5 "Krn";SPC( 10)
790 PLOT xv<v )> yv(v)> l’PLOT x.v< v)+2> y vtv),1
800 FOR tP = l TO 2000’NEXT tP
810 LOCATE 5,29-PRINT SPCC30)
820 PLOT xv( v > > y v( v >, 0 ■ PLOT xv( w )+2 > y v( v ) > 9
830 GOTO 520 TRACÉ D’UN DESSIN EN RELATIF Une figure définie en coordonnées relatives (chaque point est défini par rapport au précédent) peut être représentée facilement avec une échelle."##########,
    "pages 134, 135"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_246,
    r##########"10 — “ — — — -- trace de la carte de FRANCE en relatif
20 MODE 1
30 INK 0,1’INK 1, 24
40 ech=0.5 f echelle
50 xa=200:ya=200 1er Point
60 '
70 DATA 52,-28, 70,-24, -18,-38
80 DATA 0,-20, -16,-20, 0,-6
90 DATA 8,0, 4, 6, 10,0
100 DATA -8,-14, 6,-22, 0,-8
110 DATA 12,-8, 10,-2, 0,-12
120 DATA -22,-8, -18,-10, -14,10
130 DATA -20,4, -14,-6, -10,-14
140 DATA -2,-14, -6,-4, -64,16
150 DATA -40,8, 12,32, 4, 30
160 DATA -6,18, -18,14, -14,22
170 DATA -10,10, -18,8, -14,0
180 DATA 0,16, 36,4, 14, -6
190 DATA 16,-2, -12,12, -2,20
200 DATA 16,0, 4,-6, 18,-6
210 DATA 12,0, 4,8, 4,8
220 DATA 14,8, 6, 16, 14,8
230 DATA 999,999
240 ■'
250 PLOT xa,ya
260 '
270 READ dx,dy’IF dx=999 THEN END
280 DRAWR ech^dx, ech*dy,1
290 LOTO 270"##########,
    "page 135"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_247,
    r##########"10 •*_________
20 DATA"##########,
    "page 136"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_248,
    r##########"30 DATA 328>342> 338>318> 380 .. 280
40 DATA 380> 260> 364>240.. 364>234
50 DATA 372>234> 376>240> 386>240
60 ■' suite carte de FRANCE
70 ■'
130 DATA 333 >39'3
200 '
210 READ xa.ya ' Premier Point 220
230 READ x>y
240 IF x=333 THEN END
250 dx-x-xa;dy =y-y a
260 PRINT #8>dx>dy ' imprimante
270 xa=x;ya=y
280 GOTO 230"##########,
    "page 136"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_249,
    r##########"10 0 ROTATION D’UNE FIGURE ______________________________________ Ce programme représente une figure définie dans la position demandée.
10 '--------------------------------------------------------- rotation d’une fi9ure
20 ‘
30 MODE 1
40 np=7 ' nombre de Points
50 DATA 200>200
60 DATA -250 > 200
70 DATA 250>120
80 DATA 220>120
90 DATA 220>160
100 DATA 200>160
110 DATA 200>200
120 '
130 FOR i=l TO nP.READ x<i)>y(i)=NEXT i
140 ’
150 x0=x(1):y0=y(1 ) ' centre de rotation
160 •’
170 LOCATE 1>2=INPUT "Quel an9le<de9re) " > a.
180 a=at2tP1/360
190 s=S I N< a "> ■ c=COSC a )
200 FOR P=1 TO nP
210 dx=x(P)-x0:dy=y<P )-y0
220 x1< P)=x0+dx*c+dy*s
230 y1< P >=y0+dytc-dxts
240 NEXT P
250 ce=l-GOSUB 280
260 GOTO 170
270 ----------------------------------------------------------- £P9rn trace
280 PLOT xl(1 ),yl( 1),ce
290 FOR P=2 TO nP
300 DRAW x 1 < P)jy 1(Pce
310 NEXT P
320 RETURN duel angle(degre) ? *Break* Break in 170 R ead y SQUASH ________________________________________________________ Vous devez faire rebondir une balle à l'aide d’une raquette que vous déplacez avec les deux flèches -» et . Nous avons utilisé la fonction INKEY(n° touche) de façon à obtenir un déplacement rapide de la raquette. Avec INKEY$, le délai de répétition des touches devrait être modifié par SPEED KEY. I X1 I X2 Floches pour raquette"##########,
    "pages 136, 137"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_250,
    r##########"10 >-----------------------------SQUASH
20 CLS: INPUT "Ni veau <1,2,3) ";nv
30 ‘----------------------- dessin terrain
40 MODE 1
50 x2=18+nv*2
60 xl=2’y 1=2’y2=22
70 CLS
80 LOCATE 1,24’PRINT "Fléchés Pour raquette""##########,
    "page 138"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_251,
    r##########"30 FOR x=xl TO x2’LOCATE x,yl’PRINT CHR$<143)’NEXT x
100 FOR y =3 1 TO y 2-1’LOCATE Kl,y’PRINT CHR$(143 )’LOCATE x2,y’P RINT CHR$<143)-NEXT y
110 ■'
120 rb=0 ' rebonds
130 r q $=CHR$< 32 )+CHR$<143)+CHR$<143 )+CHR$<143)+CHR$<32 )
140 dx=l=dy=-l
150 xb=5+INT<RND<1 )*5)’yb=10 ’ balle
160 xr=10;yr=y2 ' raquette
170 LOCATE xr-1,yr’PRINT r-q$ ISO '-----------------------------------------------------déplacement balle"##########,
    "page 138"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_252,
    r##########"130 LOCATE xb,yb’PRINT CHR$<32) ' effacement balle
200 xb=xb+dx’yb=yb+dy ' nouvelle Position
210 IF yb=yl+3 THEN yb=yb+INT<RNDT2 )*dy
220 LOCATE xb,yb’PRINT CHR$<231)
230 IF xb>=x2-l THEN dx=-dx ’ rebonds
240 IF yb<yl+2 THEN dy=-dy
250 IF xb<xl+2 THEN dx=-dx
260 '
270 IF xb>xr-2 AND xb<xr+4 AND yb>yr—2 THEN dy=-dy■rb=rb+l
280 '
290 IF yb>=YR THEN LOCATE 30,23’PRINT rb;"POINTS"’GOTO 380
300 '---------------------------------------------- dePlacement raquette
310 FOR tP=l TO 10’NEXT tP
320 '
330 IF INKEYC1)=0 THEN IF xr<x2-3 THEN xr=xr+l
340 IF INKEY<8)=0 THEN IF xr>xl+l THEN xr=xr-l
350 LOCATE xr-1, yr-’ PRINT r-q$
360 GOTO 150
370 ■’
380 FOR TP=1 TO 2000’NEXT TP’GOTO 50 TRACÉ DE COURBE _____________________________________________ Ce programme trace la courbe d’une fonction écrite en 400. Les échelles sont calculées par le programme."##########,
    "page 138"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_253,
    r##########"10 '---------------------------trace de courbe
20 MODE 2‘PAPER 0‘PEN 1
30 INK 0,1‘INK 1,24
40 hecr=300‘lecr=500 ■' hauteur /largeur écran
50 INPUT "Borne XI ";bl
60 INPUT "Borne X2 ";b2
70 INPUT "Pas ";Pas=IF Pas=0 THEN Pas=0.1
80 '--------------------------------- recherche mini/maxi
90 x=bl‘GOSUB 400^1=y>3 2=y
100 FOR x=bl TO b2 STEP Pas
110 GOSUB 400
120 IF'a<31 THEN a 1=3
130 IF 3>a2 THEN a2=3
140 NEXT x
150 ex=lecr/(b2-bl ) ' echelle x
160 ea=<hecr )/(a2-a1 )
170 ’-------------------------axe a
180 IF b2>=0 RND bl<=0 THEN x=-ex*bl‘PLOT x,l‘DRRN x,hecr,l
190 '-----------------------------3.XS X
200 IF a2>=0 RND al<=0 THEN a=-ylYea:PLOT 1,3‘DRAW lecr,a
210 '------------------------------------------courbe
220 FOR x=bl TO b2 STEP Pas
230 GOSUB 400
240 sx=(x-bl>tex‘sa=<a-a1 Xtea
250 PLOT sx,S3,l
260 NEXT x
270 '--------------------------------------affichage extremes
280 3=-altea+12=x=12‘b=bl‘GOSUB 340"##########,
    "page 139"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_254,
    r##########"230 y=-yltea+12■x=lecr‘b=b2■GOSUB 340
300 3=12=x=RBS<bl>*ex=b=31‘GOSUB 340
310 3=hecr‘x=RBS(bl )tex‘b=3 2‘G0SUB 340
320 END
330 ' —
340 IF x>lecr THEN x=lecr-/2
350 IF 3>hecr THEN 3=hecrz2
360 PLOT x,a
370 TRG‘PRINT B;
380 RETURN"##########,
    "page 139"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_255,
    r##########"330 •’-----------------------------------courbe a tracer
400 Y=SIN<X)^X
410 RETURN"##########,
    "page 139"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_256,
    r##########"10 >-------------------------------------------------------HISTOGRAMME
20 MODE 2
30 INK 0,1 =INK 1,24=PAPER 0=PEN 1
40 nm=7 ' nombre de mois
50 DATA jv,500, fv,400
60 DATA mrs,600, avr,700
70 DATA mai,600, jui,500
80 DATA J u. il, 700"##########,
    "page 140"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_257,
    r##########"30 ' —
100 FOR..i = l TO nm=RERD moisîti),vnte<i)=NEXT i
110 '
120 xa=20=ya=30 ' dePart axes
130 ix=50 ' intervalle X
140 hecr=300 ' hauteur écran
150 '------------------------------------------------------------------recherche maxi
160 mx=vnte<1>
170 FOR m=2 TO nm
180 IF vnte<m)>mx THEN mx=vnte(m )"##########,
    "page 140"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_258,
    r##########"130 NEXT m
200 ech==hecr/mx ' echelle
210 '----------------------------------------------------------—— axes
220 PLOT xa,ya=DRRW xa+nm^ix,ya,1
230 PLOT xa,ya=DRRN xa,hecr+ya,l
240 '----------------------------------------------------------------------affichage mois
250 FOR m=l TO nm
260 x=xa+8+i x*(m-1) = y=ya-12
270 PLOT x,y=TRG=PRINT moisS<m)j
280 NEXT m"##########,
    "page 140"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_259,
    r##########"230 ‘--------------------------------------------------------------------courbe
300 FOR m=l TO nm
310 xl=xa+10+ix$Km-1 ) = yl=ya+vnte<m >*ech
320 FOR dx=0 TC 5
330 PLOT xl+dx,ya=DRRW xl+dx,yl,l
340 NEXT dx
350 NEXT m
360 '----------------------------------------------------------------- affichage valeurs
370 FOR m=l TO nm
389 y =y a+vntefm Mech+16
390 x=xa+ix*Cm-l )-10
400 PLOT x,y=TAG-PRINT vnte(m);
410 NEXT m HISTOGRAMME 3D _____________________________________________ Ce programme d'histogramme en trois dimensions, écrit sur AMSTRAD, s’adapte sur tout matériel disposant du graphique haute résolution. Pour un matériel disposant de l’instruction BOXF (boîte pleine), tel que le MO5 ou MSX, le programme devient plus simple."##########,
    "pages 140, 141"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_260,
    r##########"10 '--------------------- histogramme 3D
20 MODE 2
30 xd=50-yd=50 ' origine
40 19=30 ' Ïar-Seur-
50 Pr=10 ' Profondeur
60 itv=30 ’ intervalle batons
70 '
80 nh=5 • nombre de batons
90 ’
100 h(1>=40
110 h(2>=30
120 h<3>=50
130 h(4)=10
140 ht 5>=60
150 '-------- ■
160 FOR h=nh TO 1 STEP-1
170 xb=x.d+( h-1 >titv ’ origine baton
180 yb=yd+<h-1>*itv
190 FOR dx=l TO Pr ' 1 baton
200 PLOT xb+dx , y b+dx+hCh ).1
210 IF dx>l THEN PLOT xb+dx+19,yb+dx,1 - DRAW xb+dx+19,yb+dx+h (h), 1-GOTO 250
220 FOR d1=1 TO 19 ' rectangle
230 PLOT xb+dx+dGyb+dx, 1-DRAW xb+dx+dl > yb+dx+h< h 1
240 NEXT dl
250 PLOT xb+dx+l,yb+dx+h<h)-DRAW xb+dx+19,yb+dx+hCh>,0
260 NEXT dx
270 PLOT xb+dx-1, yb+dx+hCh>,1-DRAW xb+dx-1+19,yb+dx+h<h>,1
280 TAG-PLOT xb+30,yb - PRINT h(h>;
290 NEXT h"##########,
    "page 141"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_261,
    r##########"10 •'--------------------------------------------saisie dans une table FICH$<>
20 MODE -2
30 nrub=4 • nombre de rubriques
40 nrub$(1>="N0M"
50 nrub$< 2 >"PRENOM"
60 nrub$(3>="RUE"
70 nrubS<4 )="VILLE"
80 '"##########,
    "page 142"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_262,
    r##########"30 GOSUB 130
100 PRINT’FÜR ln=l TO nrub’PRINT f ich« In ) = NEXT In
110 END
120 ’------------------------------------------------sous ProSrarwie saisie
130 CLS—
140 LOCATE 1.20:PRINT "fléché haut Pour zone arriéré"
150 FOR ln=l TO nrub
160 LOCATE l.ln+11PRINT nrub$(ln)
170 LOCATE 10jln+1■PRINT fich$Cln) ' ancienne valeur
180 NEXT In
190 '
200 FOR ln=l TO nrub
210 xs=10;as=ln+l
220 GOSUB 320
230 IF r=l THEN fich«In>=li9«
240 IF r=3 THEN IF ln>l THEN LOCATE xs. ys = PRINT-f ichSK In > ‘ ln= ln-l'G0T0 210 ELSE 210
250 LOCATE xs.ys;PRINT fich»<In>;SPCC20-LENCfrch«<In>))
260 NEXT In
270 PRINT:FOR ln=l TO nrub:PRINT fich«<In)=NEXT In ' Pour te st
280 END
290 •'---------------------------------------------------------sP9r*i saisie dans li9$
300 •' R=3 zone arriéré
310 ’
320 li9$=""
330 '
340 19=LENCIi9$);LOCATE xs+19,as : PRINT CHRÎC143) ■' 143‘curseu r
350 LOCATE Xs+19,3S
360 '
370 c#=INKEY$;IF c«="" THEN 370 ' attente fraPPe
380 C=ASC(C«>"##########,
    "pages 142, 143"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_263,
    r##########"330 '
400 IF c<>!27 THEN 430 ' code suppression
410 IF 19>0 THEN Ii9$=LEFT$<1i9$,19-1 ) = PRINT CHRÎC32 ) ;GOTO 340 ELSE 340
420 '
430 IF c=13 THEN 500 •• code ENTER?
440 IF c=240 THEN r=3 = LOCATE xs+19,as■PRINT CHR$<32 )•RETURN
450 IF c<32 OR 0127 THEN PRINT CHRSC 7 ); > GOTO 370
460 li9$=li9$+cS ' ajout caractère fr aPPe
478 PRINT c$ ' afficha9e caracter e fraPPe
480 GOTO 340"##########,
    "page 143"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_264,
    r##########"430 '
500 LOCATE xs+l9,as-PRINT CHRSC32)
510 IF li9$<>"" THEN r=l ELSE r=2
520 RETURN FICHIER D’ADRESSES ___________________________________________ Le programme ci-dessous permet d’introduire et de modifier des données indépen­ dantes du programme. Elles sont temporairement stockées dans des tables qui sont ensuite sauvegardées sur cassette. NOM$() RUE$() VILLESO RANG-» Sauvegarde NOM$() RUE$() La variable “RANG” donne l’adresse de rangement dans les tables. Le mode “C” permet à la fois de créer et de modifier des fiches. En mode modification, la valeur de chaque zone est affichée puis le programme attend une nouvelle valeur. Si vous ne voulez pas modifier une zone, appuyez sur “ENTER” sans entrer de valeur."##########,
    "page 143"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_265,
    r##########"1140 INPUT « Quelle ville », V$
1180 IF V$<> VILLE$(F) THEN 1200 TRI-MULTICRITÈRES La liste alphabétique des clients dans l’ordre des villes s’obtient en faisant : CLE$(F) = VILLE$(F) + NOM$(F) au lieu de CLE$(F) = NOM$(F)."##########,
    "page 144"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_266,
    r##########"10 '------------------------------------------------- fichier d'adresses
20 MODE 2:PRINT "CLAVIER MAJUSCULE"
30 DIM NOM*(100),RUE*(100),VILLE*(100),CPST*(100)
40 DIM CLE? ( 100) ,1X7.(100)
50 NF ICH=O 'nombre de fiches
60 INPUT "NOUVEAU FICHIER (0/N) " ; RI: IF R$="0" OR R$=”o" THEN 90
70 GOSUB 680
80 '
90 CLS:PRINT "MODES:":PRINT
100 PRINT TAB(3) "C: CREATION/MODIFICATION"
110 PRINT TAB (3) "LF: LISTE DU FICHIER"
120 PRINT TAB(3) "FIN: FIN DE SESSION(SAUVEGARDE) "
130 PRINT TAB(3) "LFN: LISTE DU FICHIER PAR NOM"
135 PRINT TAB(3) "LFV: LISTE TRIEE PAR VILLES"
140 PRINT:INPUT "MODE ";M$:M$=UPPERÎ(M$)
150 IF M$="C" THEN GOSUB 230
160 IF MI="FIN" THEN GOSUB 500:END
170 IF M$="LF" THEN GOSUB 810
180 IF MI="LFN" THEN GOSUB 930
190 IF M$="LFV" THEN GOSUB 1210
200 IF M$="S" THEN GOSUB 1450
210 GOTO 90
220 '============================*==“== creation/modification
230 PRINT
240 PRINT "<ENTER> pour fin de mode"
250 PRINT "<ENTER> pour zone inchangee(en modif)": PR I NT
260 LINE INPUT "NOM (ou <ENTER>) ? ",NOMS: IF NQM$ = "" THEN RETURN
270 ' pas d'espace entre "" de IF NOM$=""
280 IF NFICH=O THEN 330 —
290 FOR RANG=1 TO NFICH
300 IF NOMS=NOM$(RANG) THEN 380 ' nom existe t-il?
310 NEXT RANG
320 '-----------------------------nouveau nom
330 PRINT : INPUT "NOUVEAU NOM OK (0/N) ";RS:IF RS<>"0" AND RSO o" 'THEN 230
340 NF ICH = NFICH+1
350 RANG=NFICH
360 NOMS(RANG)=NOMS
370 - -- - entree/modification zones
380 PRINT
390 PRINT RUES(RANG);TAB(15); ' ancienne valeur
400 LINE INPUT "RUE? ",RUES: IF RUESO"" THEN RUES (RANG) =RUES
410 PRINT VILLES(RANG);TAB(15) ;
420 LINE INPUT "VILLE? ",VILLES
430 IF VILLESO"" THEN VILLES (RANG) =VILLES
440 PRINT CPSTS(RANG);TAB ( 1 5);
450 LINE INPUT "CODE POSTAL? ",CPSTS
460 IF CPSTSO"" THEN CPSTS(RANG)=CPSTS
470 GOTO 230
490 '================================== sauvegarde
500 OPENOUT “ADR"
510 '
520 PRINT #9,NFICH
530 FOR F=1 TO NFICH
540 PRINT #9,NOMS(F)
550 PRINT #9,RUES(F)
560 PRINT #9,VILLES(F)
570 PRINT #9 , CPSTS(F)
580 NEXT F
590 CLOSEOUT
600 RETURN
670 '= = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = lecture -fichier
680 OPENIN "ADR"
690 '
700 INPUT #9,NFICH
710 FOR F=1 TO NFICH
720 LINE INPUT #9,NOMS(F)
730 LINE INPUT #9,RUES(F)
740 LINE INPUT #9,VILLES(F)
750 LINE INPUT #9,CPST$(F)
760 NEXT F
770 CLOSEIN
780 PRINT:PRINT NFICH;"FICHES"
790 FOR TP=1 TO 2000:NEXT TP
800 RETURN
810 '=================================== liste du fichier
820 PRINT "LISTE DU F I CH I ERPR I NT
830 '
840 FOR F=1 TO NFICH
850 IF F MOD 20=0 THEN INPUT "APPUYER SUR ENTER";XS
860 PRINT NOMS(F); TAB(13);
870 PRINT VILLES(F) ;
880 PRINT
890 NEXT F
900 PRINT:INPUT "APPUYER SUR <ENTER> ";XS
910 RETURN
920 '=================================== liste triee par nom
930 FOR F=1 TO NFICH
940 CLES (F) =NOMS (F ) : I X7. (F) =F
950 NEXT F
960 '
970 NC=NFICH ' ne: nombre de des
980 GOSUB 1090 ' appel tri
990 '
1000 CLS:PRINT "LISTE TRIEE PAR NOM":PRINT
1010 FOR F=1 TO NC
1020 X = IX7. (F)
1030 PRINT NOMS(X); TAB ( 15);
1040 PRINT VILLES(X)
1050 NEXT F
1060 PRINT: INPUT "APPUYER SUR <ENTER> ";XS
1070 RETURN
1080 ‘----------------------------------------------------- tri shell
1090 ECART=NC
1100 ECART= I NT(ECART/2): IF ECART< 1 THEN RETURN
1110 10 = 0
1120 FOR K=1 TO NC-ECART
1130 J=K+ECART
1140 IF CLES(J)>=CLES(K) THEN 1170
1150 XS=CLES(K): CLES (K)=CLES(J): CLES(J)=XS:10 = 1
1160 X = IX7. (K) : 1X7. (K)=IX7. (J) : 1X7. (J)=X
1170 NEXT K
1180 IF 10=1 THEN 1110
1190 GOTO 1100
1200 '=============================== liste triee par ville
1210 INPUT "VILLE (ENTER pour toutes) ";CLES
1220 NC = O ' nombre de des
1230 LG=LEN(CLES)
1240 FOR F=1 TO NFICH
1250 IF CLES<>LEFTS(OILLES(F), LG) THEN 1270
1260 NC = NC + 1 : CLES ( NC ) =01LLES ( F ) : I X7. ( NC ) = F
1270 NEXT F
1340 GOSUB 1090 ' appel tri
1350 CLS:PRINT "LISTE TRIEE PAR VILLE":PRINT
1360 FOR F=1 TO NC
1370 IF F MOD 20=0 THEN INPUT "APPUYER SUR ENTER";XS
1380 X = IX7.(F)
1390 PRINT VILLES(X)î TAB(15);
1400 PRINT NOMS(X)
1410 NEXT F
1420 PRINT : INPUT "APPUYER SUR <ENTER> ";XS
1430 RETURN
1440 '================================== suppression
1450 PRINTLINE INPUT "NOM? ";NOMS:IF NOMS="" THEN RETURN
1460 '
1470 FOR RANG=1 TO NFICH
1480 IF NOMS(RANG)=NOMS THEN 1520
1490 NEXT RANG
1500 PRINTzPRINT "N'existe pasPR I NT : GOTO 1450
1510 '
1520 PRINT
1530 INPUT "SUPPRESSION OK (0/N) “;RS:IF RS<>"0" THEN 1450
1540 FOR J=RANG TO NFICH-1
1550 NOMS(J)=NOMS(J +1 )
1560 RUES(J)=RUES ( J+l )
1570 VILLES(J)= VILLES ( J +1 )"##########,
    "pages 145, 146, 147"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_267,
    r##########"1580 CPST*(J)=CPST$(J + l )
1590 NEXT J
1600 NON*(NFICH)RUE*(NFICH)VILLE*(NFICH)CPST*(NFICH) — Il II
1610 NFICH=NFICH-1
1620 GOTO 1450 Mode ? C <ENTER> pour fin de mode <ENTER> pour zone inchangee(modif) Nom (ou <ENTER>? DURAND Nouveau nom OK (0/N) ? 0 Rue? 13,RUE DE MILAN Ville? PARIS Code postal? 75000 <ENTER> pour fin de mode <ENTER> pour zone inchangee(modif) Nom (ou <ENTER>? Break in 260 Ok Liste triee par nom BALU RUE XXX ST CLOUD BESSE RUE DE MILAN TOULON DUPONT 11,RUE NOBEL MONTIGNY DURAND 13,RUE DE MILANPARIS MARTIN XXX KKKK RACLIN RUE XXX PARIS Appuyer sur ENTER ? GESTION DE FICHIER AUTOMATIQUE Avec ce programme, la description des rubriques (NOM,LONGUEUR,TYPE) est faite par l’utilisateur de façon conversationnelle. Cette description est sauvegardée dans le fichier. Une table FICH$(,) à 2 dimensions contient dans chaque ligne un “enregistrement”. NRUB$() TYP$() LG() NOM PR RUE VILLE CPST NOM C 12 DUPONT JEAN RUE XXXX MONTIGNY 78180 PR C 10 MARTIN DANIEL RUE C 15 BALU THIERRY VILLE C 15 CPST C 5"##########,
    "page 148"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_268,
    r##########"3000 FOR F=1 TO NFICH
3010 PRINT FICH$(F,1>; TAB(20)
3020 PRINT FICH$(F,2)
3040 NEXT F Si vous n’effectuez qu’une sauvegarde, faire :"##########,
    "page 150"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_269,
    r##########"10 ------------------------------- --------------------------------- GESTION DE FICHIER 20
30 Les noms des rubriques sont de-finis de façon
40 conversationnelle et sauvegardes avec 1 e fichier.
50 Une tabl e FICH$(,) a 2 dimensions contient dans
60 chaque ligne un 'enregistrement'.
70 -
80 ‘ NFICH: nombre maximum de fiches
90 ' NFICH: nombre de fiches
100 ‘ NRUB$(): noms des rubriques 110
120 NFICH=O ' nombre de fiches
130 MODE 2:PRINT "CLAVIER MAJUSCULE"
140 INPUT "Nouveau fichier (0/N) ";R$: IF R$=”0" OR R$="o" THEN G OSUB 320:G0T0 160
150 GOSUB 930
160 CLS: PR I NT "MODES:":PRINT
170 PRINT TAB(3);"C: CREAT ION/MODIFI CAT I ON"
180 PRINT TAB(3);"LF: LISTE DU FICHIER"
190 PRINT TAB(3);"FIN: FIN DE SESSION(SAUVEGARDE) "
200 PRINT TAB(3);"LFA: LISTE DU FICHIER"
210 PRINT TAB (3);"LFT: LISTE DU FICHIER TRIEE"
220 PRINT TAB (3);"S: SUPPRESSION FICHE"
230 PRINT: INPUT "MODE ":M$:M*=UPPER$(M$)
240 IF M$="C" THEN GOSUB 480
250 IF M$="FIN" THEN GOSUB 790:END
260 IF M$="LF" THEN GOSUB 1110
270 IF MT="LFA" THEN GOSUB 1310
280 IF M$="LFT" THEN GOSUB 1380
290 IF M$="S" THEN GOSUB 2130
300 GOTO 160
310 rubriques
320 PRINT
330 INPUT "Nombre maxi de fiches ";MFICH
340 PRINT
350 FOR K=1 TO 10 - ' 10 rubri ques maxi
360 LINE INPUT "Norn rubri que (ou <ENTER>) ";X$
370 IF X$="" THEN NRUB=K-1 : DIM FICH$(MF I CH,NRUB) ,CTRU(MFICH) , I X7. ( MF I CH ) : RETURN
380 NRUB$(K)=X$
390 LINE INPUT "Type (C=chaine/ N=numerique ) ? ",X$
400 IF X$O"C" AND X$O"N" THEN 390
410 TYP$(K)=X$
420 INPUT "Longueur maxi ";X
430 LG(K)=X
440 NEXT K
450 PRINT "TROP DE RUER I CUES": STOP
460 '================================ CREAT ION/MODIF I CAT I ON
470 ' En mode modification,entrer seulement les premieres lettre s de la c1e
480 PRINT
490 PRINT NRUB$( 1 ) ;
500 LINE INPUT " (ou <ENTER> ) ";CLE$:IF CLE$="" THEN RETURN
510 ' pas d'espace entre "" de IF CLE$="“
520 LG=LEN(CLE$)
530 IF NFICH=O THEN 620
540 FOR RANG=1 TO NFICH
550 IF CLE$ = LEFT$(FICH$(RANG,1),LG) THEN 680 ' nom existe?
560 NEXT RANG
610 '-------------------------- nouvelle cle
620 PRINT:INPUT "Nouvelle cle OK (0/N) ";R$
630 IF R$<>"0" AND R$<>"o" THEN 480
640 NF I CH = NF ICH+1
650 RANG=NFICH
660 FICHI(RANG,1)= CLE$
670 '------------------------------------------- entree/modif zones
680 PRINT
690 PRINT "R:ZONE ARRI ERE": PR I NT
700 FOR R=2 TO NRUB ' ou FOR R=1 TO NRUB
710 PRINT FI CH$ (RANG,R):TAB(151 ; ' ancienne valeur
720 PRINT NRUB$(R);TAB(22); ' nom de zone
730 LINE INPUT "?",X$
740 IF X$="R" THEN IF R>1 THEN R=R-1:GOTO 710
750 IF X$<>"" THEN FICH$(RANG,R)=X$
760 NEXT R
770 GOTO 480
780 '==============================*============= sauvegarde
790 OPENOUT "FICH"
800 PRINT #9,MF I CH : PR I NT #9,NFICH: PR I NT #9,NRUB
810 FOR R=1 TO NRUB
820 PRINT #9,NRUB$(R) :PRINT #9 , TYF’t ( R ) : PR I NT #9,LG(R)
830 NEXT R
840 '
850 FOR F=1 TO NFICH
860 FOR R=1 TO NRUB
870 PRINT #9,FICH$(F,R)
880 NEXT R
890 NEXT F
900 CLOSEOUT
910 RETURN
920 '===================================== lecture fichier
930 OPENIN "FICH"
940 '
950 INPUT #9,NFICH,NFICH,NRUB
960 DIM FI CHI ( MF I CH, NRUB ) , CTRI $ ( MF I CH ) , I X7. ( MF I CH )
970 FOR R=1 TO NRUB
980 INPUT #9,NRUB$(R),TYP$(R),LG(R)
990 NEXT R
1000 FOR F=1 TO NFICH
1010 FOR R=1 TO NRUB
1020 LINE INPUT #9,FICH$(F , R) ~*
1030 NEXT R
1040 NEXT F
1050 CLOSE IN
1060 PRINT:PRINT NFICH;"FICHES"
1070 FOR TP=1 TO 2000:NEXT TP
1080 GOSUB 2070
1090 RETURN
1100 '= = = = = = = = = = = = = = = = = = = =■ = = = = = = = = liste du -fichier directe
1110 CLS
1120 PRINT "LISTE DU FI CH I ERPR I NT
1130 '
1140 FOR F=1 TO NFICH
1150 IF F MOD 20=0 THEN INPUT "Appuyer sur ENTER ";X$
1160 PRINT FICH$(F,1); TAB ( 15); ' zone 1
1170 PRINT FICH$(F,2) ' zone 2
1180 NEXT F
1190 PRINT: INPUT "APPUYER SUR <ENTER> ";X$
1200 RETURN
1300 '======================== liste des fiches automatique
1310 ID(1) = 1 : ID(2)=3 ' numéros des zones (a adapter)
1320 '
1330 CLS:PRINT "LISTE DES FI CHES": PR I NT
1340 FOR F = 1 TO NF I CH : I X7. ( F ) =F : NEXT F
1350 NC = NFI CH :GOSUB 1650
1360 RETURN
1370 '=========================== tri/selection des fiches
1375 ' Pour clés commençant par 'BA' repondre 'BA'
1376 ' a la question 'Cle selection? '
1380 INPUT "Position tri (nom rubrique) ";X$:GOSUB 1490:IF R>NRU B THEN 1380
1390 PTRI=R
1400 LINE INPUT "Cle selection? (ENTER pour toutes) ",CLE$
1410 FOR J=1 TO NRUB+1
1420 ID(J)=O
1430 LINE INPUT "Nom Rubrique a éditer (ou ENTER ";X$:IF X$ = "" THEN 1540
1440 GOSUB 1490:IF R>NRUB THEN 1420
1450 ID(J)=R
1460 NEXT J
1470 GOTO 1540 1480
1490 FOR R=1 TO NRUB
1500 IF X$=NRUB$(R) THEN RETURN
1510 NEXT R
1520 RETURN
1530 '--------
1540 N C = 0 'ne: nombre de clés
1550 LG=LEN(CLE$)
1560 FOR F=1 TO NFICH
1570 IF CLE$<>LEFT$(FICH$(F,PTRI),LG) THEN 1590
1580 NC = NC+1 : CTRI$ (NC) =FICH$ (F , PTRI ) : IX7. (NC ) =F
1590 NEXT F
1600 GOSUB 1920 ' appel tri
1610 PRINT: PRINT "LISTE TRIEE DES FICHES ":PRINT
1620 GOSUB 1650
1630 RETURN
1640 '--------------------------------- spgm edition écran
1650 FOR W=1 TO 10
1660 IF ID(W)=O THEN 1700
1670 PS(W)=PS(W-1)+LG(ID(W))+2
1680 NEXT W 1690
1700 FOR F=1 TO NO
1710 F1 = IX7.(F)
1720 PRINT
1730 FOR R=1 TO NRUB
1740 IF ID(R)=0 THEN 1790
1750 PRINT FICH$(Fl,ID(R));
1760 PRINT TAB (PS(R));
1770 PRINT
1780 NEXT R
1790 PRINT
1800 IF F MOD 20=0 THEN INPUT "APPUYER SUR ENTER";X$
1810 NEXT F
1820 PRINTzINPUT "APPPUYER SUR <ENTER> ";X$
1830 RETURN
1910 ----------------------------------------------------------tri SHELL-METZNER
1920 ECART=NC
1930 PRINT:PRINT "JE TRIE POUR VOUS ":PRINT
1940 ECART= I NT(ECART/2): IF ECART<1 THEN RETURN
1950 J=1 :K=NC-ECART
1960 L = J
1970 M=L+ECART
1980 IF CTRI $(L)<=CTRI $(M) THEN 2040
1990 Xt = CTRI $(L):CTRI $(L)=CTRI $(M):CTRI $(M)=X$
2000 X = IX7. (L) : 1X7. (L) = IX7. (M) : 1X7. (M)=X
2010 L=L-ECART:IF L< 1 THEN 2040
2020 GOTO 1970 2030
2040 J = J + 1: IF J>K THEN 1940
2050 GOTO 1960
2060 '---------------------------- liste descri pteur
2070 FOR R=1 TO NRUB
2080 PRINT NRUBi(R)
2090 NEXT R
2100 FOR TP=1 TO 20001NEXT TP
2110 RETURN
2120 -================================== suppression fiche
2130 PRINT :PRINT NRUBt(1); :LI NE INPUT "?",CLE$
2140 IF CLE$="" THEN RETURN
2150 FOR RANG=1 TO NFICH
2160 IF CLE$ = FICH$(RANG, 1 ) THEN 2200
2170 NEXT RANG
2180 PRINTsPRINT "N'existe pas":GOTO 2130 2190
2200 PRINT: INPUT "ANNULE OK(O/N) ";R$:IF R$<>"0" THEN 2130
2210 FOR J=RANG TO NFICH-1
2220 FOR R=1 TO NRUB
2230 FICH$(J,R)=FICH$(J+1,R)
2240 NEXT R
2250 NEXT J
2260 FOR R=1 TO NRUB:FICHI(NFICH,R)=”"sNEXT R
2270 NFICH=NFICH-1
2280 GOTO 2130"##########,
    "pages 150, 151, 152, 153"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_270,
    r##########"10 --------------------------------------------------------- BIBLIOTHEQUE
20 MODE 2:PRINT "CLAVIER MAJUSCULE"
30 DIM TITRI(100),AUTf( 100),Cf(100,2)
40 DIM CLEf (200) , 1X7.(200) 50NFICH=0 nombre de fiches
60 INPUT "NOUVEAU FICHIER (0/N) ";Rf
70 IF Rf=“O" OR Rf="o" THEN 100
80 GOSUB 1110
90 '
100 CLS
110 CLS: PR INT "MODES:":PRINT
120 PRINT TAB(3) "C: CREAT ION/MODIFI CAT I ON “
130 PRINT TAB (3) " LT ITRE:LI STE PAR TITRE"
140 PRINT TAB(3) "LCLE: LISTE PAR MOT-CLE"
150 PRINT TAB(3) "FIN: SAUVEGARDE"
160 PRINT TAB(3) "S: SUPPRESSION FICHE"
170 PRINT:INPUT "MODE ";Mf:Mf = UPPERf (Mf)
180 IF Mf="C" THEN GOSUB 270
190 IF M$="LTITRE" THEN GOSUB 660
200 IF Mf="FIN" THEN GOSUB 980:END
210 IF Mf="LCLE" THEN GOSUB 1310
220 IF Mf=”S" THEN GOSUB 1550
230 GOTO 110
240 '=================s============== creation/modification
250 ' en mode modification,entrer les premieres lettres du titre
260 '
270 PRINT
280 PRINTLINE INPUT "TITRE? (OU ENTER) ";TITRî
290 IF TITRf="" THEN RETURN
300 '
310 LG=LEN(TITRf)
320 IF NFICH=0 THEN 370
330 FOR RANG=1 TO NFICH
340 IF TITRf = LEFTf(TITRf(RANG),LG) THEN 460
350 NEXT RANG
360 '-------------------------------------------------nouveau titre __
370 PRINT:INPUT "NOUVEAU TITRE (0/N) "jRI
380 IF RI<>"0" AND RIO"o" THEN 280
390 NF I CH = NFICH+1
400 RANG=NFICH
410 ■
420 TI TRI(RANG)=TI TRI 430
440 ' pour modification,appuyer sur <ENTER> si zone sans changera ent 450
460 PRINT :PRINT TITRI(RANG): PRINT
470 PRINT AUTI(RANG); TAB(15); ancienne valeur
480 INPUT "AUTEUR ";AUTI:IF AUTIO"" THEN AUTI (RANG)=AUTI
490 ■
500 FOR C=1 TO 2
510 PRINT Cl(RANG,C);TAB( 15) ; ancienne valeur
520 INPUT "MOT OLE ";CI
530 IF CIO"" THEN CI(RANG,C)=CI
540 NEXT C
550 GOTO 280
630 '============================== liste triee par titre
640 ' Pour liste des ouvrages commençant par 'BA' ,répondre 'BA ' a 'Cle?' 650
660 PRINT:INPUT "CLE (ENTER pour tous les titres) ";CLEI
670 LG=LEN(CLEI)
680 NC=O ' ncznombre de cles
690 FOR LV=1 TO NFICH
700 IF CLEIOLEFTKTITRKLV) ,LG) THEN 720
710 NC=NC+1:CLEI(NC)=TITRI(LV) : IX7.(NC)=LV
715 IF F MOD 20=0 THEN INPUT "APPUYER SUR ENTER";XI
720 NEXT LV
730 GOSUB 860 ' appel tri
740 ---------------------------------- edition
750 CLS:PRINT "LISTE TRIEE PAR TITRE ":PRINT
760 IF NC = O THEN RETURN
770 FOR LV=1 TO NC
780 X = IX7.(LV)
790 PRINT LEFT!(TITRI(X),20); TAB(22);
800 PRINT AUTI(X)
810 IF LV MOD 20=0 THEN INPUT XI
820 NEXT LV
830 PRINT:INPUT "APPUYER SUR <ENTER> ";XI
840 RETURN
850 ■---------------------------------------------------- tri shell
860 ECART=NC 870
880 ECART=INT(ECART/2): IF ECARTCI THEN RETURN
890 I V=0
900 FOR K=1 TO NC-ECART
910 J=K+ECART
920 IF CLEI(J)>=CLEI(K) THEN 950
930 XI=CLEI(K):CLEI(K)=CLEI(J):CLEI(J)=XI: IV = 1
940 X=IX7. (K) : 1X7. ( K ) = I X 7. (J) : 1X7. (J)=X
950 NEXT K
960 IF IV=1 THEN 890 ELSE 880
970 '================================== sauvegarde cassette
980 OPENOUT "BIB"
990 IF NFICH=0 THEN RETURN
1000 PRINT 49,NFICH
1010 FOR F=1 T0 NFICH
1020 PRINT #9,TITRt (F)
1030 PRINT #9,AUTt(F)
1040 FOR C=1 TO 2
1050 PRINT #9,Ct(F,C)
1060 NEXT C
1070 NEXT F
1080 CLOSEOUT
1090 RETURN
1100 '= = = ========= = ======== ==== = ====>======= lecture cassette
1110 OPENIN " BIB"
1120 INPUT #9,NFICH
1130 FOR F=1 TO NFICH
1140 INPUT #9,TITR$(F)
1150 INPUT #9,AUTt(F)
1160 FOR C=1 TO 2
1170 INPUT #9,Ct(F,C)
1180 NEXT C
1190 NEXT F
1200 CLOSE IN
1210 PRINTîPRINT NF I CH ;"TI TRES": PR I NT
1220 FOR TP=1 TO 2000:NEXT TP
1230 RETURN
1280 '================================ liste par mot cle
1290 ' Pour tous les mot-cle,appuyer sur <ENTER> pour la quest i o n 'Cle?'
1300 '
1310 INPUT "CLE (ENTER pour toutes ) ";CLEt
1320 LG = LEN(CLEt)
1330 N C = 0 'nombredecles
1340 FOR F=1 TO NFICH
1350 FOR C=1 TO 2
1360 IF Ct(F,C)=“" THEN 1390
1370 IF CLEK >LEFTt (Ct (F ,C) ,LG) THEN 1390
1380 NC = NC+1 :CLEt(NC)=Ct(F,C) : 1X7. (NC) =F
1390 NEXT C
1400 NEXT F
1410 '
1420 GOSUB 860 ' appel tri
1430 '----------------------------------------- edition
1440 CLSsPRINT "LISTE TRIEE PAR MOT-CLEPRINT
1450 FOR F=1 TO NC
1460 X = IX7.(F)
1470 IF CLEt(F-l)OCLEt (F) THEN PRINTiPRINT CLEt(F): PR I NT
1480 PRINT TAB(4);LEFTt(TITRt(X),20); TAB(25);
1490 PRINT AUTÎ(X)
1500 IF F MOD 8=0 THEN INPUT "APPUYER SUR ENTER ";Xt
1510 NEXT F
1520 PRINT:INPUT "APPUYER SUR <ENTER>";Xt
1530 RETURN
1540 '================================= suppression titre
1550 PRINT:INPUT "TITRE ";TITRt:IF TITRt=M" THEN RETURN
1560 LG = LEN(TITRt)
1570 FOR RANG=1 TO NFICH
1580 IF TITRt = LEFTt(TI TRt(RANG) , LG) THEN 1620
1590 NEXT RANG
1600 PRINTîPRINT "N'existe pas": PRINT:GOTO 1550
1610 '
1620 PRINT TITRt(RANG):PRINT
1630 INPUT "SUPPRESSION OK (0/N) ) ";Rt ~*"##########,
    "pages 155, 156, 157"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_271,
    r##########"1640 IF R$<>"0" THEN 1550
1650 FOR J=RANG TO NFICH-1
1660 T I TR$(J)=TITR$(J +1 )
1670 AL)T$ ( J ) =AUT$ ( J +1 )
1680 FOR C=1 TO 2
1690 C$(J,C)=C$(J+1,C)
1700 NEXT 0
1710 NEXT J
1720 TITR$(NFICH)AUT$(NFICH)=" "
1730 FOR C=1 TO 2 :C$(NF I CH,C)="":NEXT C
1740 NF ICH = NF I CH-1
1750 GOTO 1550"##########,
    "page 158"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_272,
    r##########"10 PRINT "AMSTRAD"
20 PRINT "CPC664" SAVE "ESSAI Ready NEW Ready LOAD "ESSAI Ready LIST"##########,
    "page 161"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_273,
    r##########"10 PRINT "AMSTRAD"
20 PRINT "CPC664" Le programme est sauvegardé avec le type “.BAS” et s’appelle en réalité “ESSAI.BAS”. Pour l’effacer, vous devrez spécifier le nom suivi du type. Lorsqu’un programme est sauvegardé une seconde fois, l’ancienne version est sau­ vegardée avec le type “.BAK”. Pour l’effacer, frappez : IERA, “ESSAI.BAK” Comme sur cassette, un programme peut être sauvegardé en ASCII par : SAVE “non-programme”, A (cf. chapitre commandes). Nous vous conseillons de dupliquer la disquette CP/M sur une autre disquette (cf. DISCCOPY plus loin). COMMANDES ACCESSIBLES SOUS BASIC"##########,
    "page 161"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_274,
    r##########"64 8 k 1 R/W A:ASM.COM"##########,
    "page 164"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_275,
    r##########"10 PRINT "AMSTRAD"
20 PRINT "CPC6128" SAVE "ESSAI" NEW LOAD "ESSAI" LIST"##########,
    "page 171"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_276,
    r##########"10 PRINT "AMSTRAD"
20 PRINT "CPC6128""##########,
    "page 171"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_277,
    r##########"10 ITAPE.IN
20 OPENIN “FICH” I TAPE. OUT _____________________________________________________ _ Pour sauvegarder un programme ou la mémoire sur cassette, frappez “TAPE.OUT” puis “SAVE “non-programme””. TAPE.OUT SAVE“PROG” ITAPE __________________________________________________________ Est équivalent à “TAPE.IN” plus “ITAPE.OUT”."##########,
    "page 174"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_278,
    r##########"10 ' ESSAI BANKMAN
20 '
30 ' FAIRE RUN "BANKMAN11 AVANT D'UTILISER CE PROGRAMME
40 '
50 MODE 2
60 •-------------------------------------------------------- ECRITURE
70 IBANKOPEN ,50 ' longueur=50
80 R7. = 0
90 A$="DUPONT Jean 044-22-63":N=1 ' enreg no 1
100 AT=LEFTT(AT+SPACET(30),30) ' normalisation
110 IBANKWRITE ,@R7.,AT,N
120 AT="MARTIN Daniel 333-44-56":N=2 ' enreg no 2
130 AT=LEFTT(AT+SPACET(30),30)
140 IBANKWRITE ,êR7.,AT,N
150 AT="BALU Thierry 555-44-77":N = 3 ' enreg no 3
160 AT=LEFTT(AT+SPACET(30),30)
170 IBANKWRITE ,@R7.,AT,N
180 '------------------------------------------- LECTURE
190 IBANKOPEN ,50
200 ‘
210 INPUT "QUEL NUMERO ENREG (0 POUR FIN) ";N
220 IF N = 0 THEN END
230 BT=SPACET(30)
240 R7. = 0
250 I BANKREAD ,@R7. ,BT,N
260 PRINT BT,R7.
270 GOTO 210 QUEL NUMERO ENREG (0 POUR FIN) ? 2 MARTIN Daniel 333-44-56 2 QUEL NUMERO ENREG (0 POUR FIN) ? 1 DUPONT Jean 044-22-63 1 QUEL NUMERO ENREG (0 POUR FIN) ? *Break* | BANKFIND ,@code retour,chaîne,début recherche,fin recherche Donne dans la variable “code retour” le numéro d’enregistrement contenant la chaîne cherchée, si elle existe. Si la chaîne cherchée n’existe pas, le code retour est égal à -3. □ 1 indique un dépassement du bloc de 64 k. o 2 indique une commutation de bloc."##########,
    "page 178"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_279,
    r##########"175 ' ajouter instructions 10 a 170 du prog precedent
180 '------------------------------------------- RECHERCHE AVEC BANKFIND
190 IBANKOPEN ,50
200 '
210 INPUT "QUEL Nom (enter POUR FIN) ";BT
220 IF LEN(B$)=0 THEN END
230 R7. = 0
240 IBANKFIND , @R7., B$ , 0,100 ' entre 0 et 100
250 IF R7.<0 THEN PRINT "N'EXISTE PAS" , R7.s GOTO 210
260 N = R7.
270 B$=SPACE$(30)
280 I BANKREAD ,@R7.,B$,N
290 PRINT B$,N
300 GOTO 210 QUEL Norn (enter POUR FIN) ? MART MARTIN Daniel 333-44-56 QUEL Norn (enter POUR FIN) ? DUP DUPONT Jean 044-22-63 QUEL Norn (enter POUR FIN) 7 SAUVEGARDE D’ÉCRAN La mémoire supplémentaire de 64 k peut être utilisée pour sauvegarder rapidement la mémoire écran de 16 k (1/2 seconde). Ainsi, quatre écrans différents peuvent être sauvegardés et restitués plus rapidement que s’ils avaient été sauvegardés sur disque. Avant d’utiliser le bloc mémoire de 64 k, pensez à faire RUN “BANKMAN”. I SCREENCOPY ,écran ou bloc dest,écran ou bloc source L’écran est référencé par le numéro 1. Les quatre blocs mémoire de sauvegarde sont repérés par les numéros 2, 3, 4, 5. Essayez en mode direct : RUN "BANKMAN" PRINT "ECRAN NUMERO 1" ISCREENCOPY ,2,1 ' sauvegarde écran dans bloc 2 CLS ' effacement écran ISCREENCOPY ,1,2 ' restitution bloc 2 dans écran Ci-dessous, les commandes sont incluses comme instructions dans un programme."##########,
    "pages 178, 179"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_280,
    r##########"10 ■------------------------------------- essai SCREENCOPY
20 ' faire RUN "BANKMAN" avant
30 ’
40 MODE 2
50 FOR 1=1 TO 20
60 PRINT "ECRAN NUMERO 1"
70 NEXT i
80 ISCREENCOPY ,2,1 ' sauvegarde écran dans bloc2
90 FOR tp=l TO 1000:NEXT tp
100 CLS
110 FOR tp=l TO 1000:NEXT tp
120 ISCREENCOPY ,1,2 ' bloc2 vers écran
178 I BASIC AMSTRAD ISCREENCOPY ,zone,écran ou bloc dest,écran ou bloc source Le paramètre “zone”, compris entre 0 et 63 peut être ajouté. Dans ce cas, seule la zone de 256 octets spécifiée est sauvegardée. Ci-dessous, nous utilisons une boucle pour sauvegarder les 64 zones de 256 octets de l’écran :"##########,
    "pages 179, 180"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_281,
    r##########"10 FOR Z=0 T0 63
20 ISCREENCOPY Z,3,1
30 NEXT Z Le programme ci-dessous dessine un cercle plein qui est sauvegardé dans le bloc 5. Un second cercle plein est sauvegardé dans le bloc 4. Les deux cercles sont ensuite affichés alternativement."##########,
    "page 180"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_282,
    r##########"10 ' essai screencopy
20 ' -faire run "bankman" avant
30 '
40 MODE 1 : INK 0,1:INK 1,20
50 XC = 100:YC = 300:R = 30:CE=1 : GOSUB 170 ' 1er cercle
60 ISCREENCOPY ,5,1 ' sauvegarde dans 5
70 CLS
80 XC = 400:YC=100:R = 30:CE=1 :GOSUB 170 ' 2eme cercle
90 ISCREENCOPY ,4,1 ' sauvegarde dans 4
100 '------------------------ affichage alterne
110 FOR TP=1 T0 1000:NEXT TP
120 ISCREENCOPY ,1,5
130 FOR TP = 1 T0 1000:NEXT TP
140 ISCREENCOPY ,1,4
150 GOTO 110
160 '-----------------------------------cercle
170 R2=R*R
180 FOR DX = -R TO R
190 DY=SQR(R2-(DX*DX))
200 PLOT XC+DX,YC+DY:DRAW XC+DX,YC-DY,CE
210 NEXT DX
220 RETURN I SCREENSWAP ,zone,écran ou bloc,écran ou bloc -------------------------------- Permet d’échanger l’écran et un bloc de 16 k. Essayez en mode direct : PRINT "ECRAN NUMERO 1" ISCREENCOPY ,2,1 CLS PRINT "ECRAN NUMERO 2" ISCREENSWAP ,1,2 ISCREENSWAP ,1,2"##########,
    "page 180"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_283,
    r##########"16 N S "A6" clr
16 C "AK"
18 C "AE" enter
66 N S Il A • 27'" esc
66 C "AC"
79 C "AX" del E #8C "'■'R" ctrl enter E #9E Il A C AB" E #9F "AFABAB" Si vous avez exécuté la commande SETKEYS KEYS.CCP, les touches ont les fonc­ tions suivantes : CONTROL/A ou <- curseur à gauche CONTROL/B ou CONTROL/<- curseur en début de ligne CONTROL/C ou CONTROL/ESC arrêt exécution CONTROL/E ou CONTRL/RETURN retour en début de ligne"##########,
    "page 184"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_284,
    r##########"10 --------------------TRANSFERT CASSETTE —-> DISQUE
20 INPUT "Nom fichier ";NF$
30 ITAPE.IN
40 OPEN IN ""
50 OPENOUT NF$
60 ■
70 IF EOF=-1 THEN CLOSEOUT : CLOSE IN : END
80 LINE INPUT
90 PRINT #9,1ig*
100 GOTO 70 Sur CPC 664, la disquette CP/M 2.2 est fournie avec FORMAT et DISCCOPY.
186 I BASIC AMSTRAD INSTRUCTIONS BASIC AMSTRAD CPC 6128 et 664 ■ PEN ■ MOVER ■ CUSOR ■ PLOT ■ FILL ■ COPYCHR$ ■ PLOTR ■ FRAME ■ DERR ■ DRAW ■ MASK ■ ON BREAK CONT ■ DRAWR ■ GRAPHICS PAPER ■ DECS ■ MOVE ■ GRAPHICS PEN Certaines instructions du BASIC CPC 464 ont été améliorées : PEN, PLOT, PLOTR, DRAW, DRAWR, MOVE, MOVER. Des instructions ont été ajoutées : FILL, FRAME, MASK, GRAPHICS PEN, GRAPHICS PAPER, CURSOR, COPYCHR$, DERR, ON BREAK CONT, DEC$. PEN # fenêtre,stylo,mode __________________________________________ Le paramètre “mode” a été ajouté à l’instruction PEN. Si “mode” est égal à 1, la fonction PAPER est inhibée. Ci-dessous, le message “AMSTRAD” est affiché sur la couleur de fond spécifiée par PAPER si “T” est égal à 0. En revanche, si “T” est égal à 1, la couleur de fond ne change pas."##########,
    "pages 187, 188"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_285,
    r##########"10 --------------------MODE TRANSPARENT
20 MODE 1
30 INK 0,1:INK 1,24:INK 2,20:INK 3,6
40 INPUT "0 OU 1 ";T
50 PAPER 2
60 PEN 1,T
70 PRINT "AMSTRAD 664"
80 PAPER 3
90 PEN 1 , T
100 PRINT "AMSTRAD 664"
110 PAPER 0 PLOT X,Y,stylo,opération PLOTR DX,DY,stylo,opération DRAW X,Y,stylo,opération DRAWR DX,DY,stylo,opération ____________________________________ Le paramètre “opération” a été ajouté. Une opération XOR,AND ou OR est effectuée avant le tracé."##########,
    "page 188"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_286,
    r##########"10 ■--------OR, XOR, AND
20 MODE 1
30 INK 0,1:INK 1,24
40 PLOT 100,100,i: DRAW 200,200,1
50 FOR TP=1 T0 1000:NEXT TP
60 PLOT 100,100:DRAW 200,200,1,1
70 GOTO 50 MOVE X,Y,stylo,opération MOVER DX,DY,stylo,opération ____________________________________ Les paramètres “stylo” et “opération” ont été ajoutés aux instructions “MOVE” et “MOVER”. FILL stylo _______________________________________________________ Remplit une figure avec la couleur du stylo spécifiée. Le curseur doit être positionné à l'intérieur de la figure avec MOVE ou MOVER. Le “stylo” spécifié dans MOVE doit être celui qui a servi au tracé de la figure. En revanche, le stylo spécifié dans FILL peut être différent."##########,
    "page 189"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_287,
    r##########"10 '---------------- FILL
20 MODE 1
30 CE=1 ' Couleur écriture
40 INK 0,26:INK 1,0:INK 2,20
50 PLOT 100, 100, CE
60 DRAW 200,100,CE
70 DRAW 200,200,CE
80 DRAW 100,100, CE
90 '--------REMPLISSAGE TRIANGLE
100 '
110 MOVE 160,120,CE ' Positionnement
120 C=1 ' Stylo remplissaqe
130 FILL C
140 1
150 ' La couleur de remplissage peut etre differente de la couleur du contour
160 ' ex:C=2
170 '
180 'Essayer en ajoutant:125 ink c,l,24
188 I BASIC AMSTRAD Le programme ci-dessous dessine un cercle et remplit la moitié inférieure."##########,
    "pages 189, 190"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_288,
    r##########"10 '--------------CERCLE
20 MODE 1
30 INK 0,26: INK 1.0:INK 2,20
40 XC=200:YC=200 ' CENTRE
50 R = 60 ' RAYON
60 '--------CERCLE
70 FLOT XC+R,YC
80 FOR A = 0 TO 2*P1+0.2 STEP 0.2
90 X=XC+R*COS(A)
100 Y=YC+R*SIN(A)
110 DRAW X,Y,1
120 NEXT A
130 '------------DIAMETRE
140 FLOT XC-R,YC:DRAWR R*2,0,l
150 '-------------- REMPLISSAGE DEMI CERCLE
160 MOVER -5,-2:FILL 2 Ci-dessous, nous représentons un demi-cercle plein en traçant un cercle “invisible” ; la couleur d’écriture du stylo 2 est la même que celle du papier. Ensuite, nous remplissons la moitié supérieure du cercle."##########,
    "page 190"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_289,
    r##########"10 '----------------DEMI CERCLE PLEIN
20 MODE 1
30 INK 0,1:INK 1,24:INK 2,1
40 XC=200:YC=200
50 R = 60
60 '------------CERCLE INVISIBLE
70 PLOT XC+R,YC
80 FOR a=0 TO 2*P1+0.2 STEP 0.2
90 X=XC+R*COS(A)
100 Y=YC+R*SIN(A)
110 DRAW X , Y, 2
120 NEXT a
130 ■--------------DIAMETRE
140 PLOT XC-R,YC:DRAWR R*2,0
150 '--------------REMPLISSAGE
160 MOVER —5,+2:FILL 1 Le programme “dessinateur” de la page 129 peut être amélioré en lui ajoutant l’ins­ truction 355. En positionnant le curseur à l’intérieur d’une figure et en appuyant sur “P”, vous la remplissez."##########,
    "page 190"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_290,
    r##########"355 IF C$="P" THEN MOVE X.,Y,CE:FILL CE
356 IF C$="S" THEN SAVE "DES " , B ,&COOO , S-4000
357 IF C$="L" THEN LOAD "DES" Attention ! Le curseur doit avoir la couleur de la figure. Pour mélanger texte et graphique, ajouter les lignes 270 et 275. En frappant vous passez alternativement en mode texte et en mode graphique."##########,
    "page 191"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_291,
    r##########"274 IF C$="#" THEN TT = ABS(TT-1): GOTO 150
275 IF TT = 1 THEN TAG:PLOT X,Y,1:PRINT C$;:X=X+16:TAGOFF:GOTO 150 CRü ISSii-'bW T FRAME _________________________________________________________ Lorsqu’un caractère est affiché avec TAG, les “points” du caractère ne sont pas af­ fichés simultanément. FRAME permet de synchroniser l’affichage des points."##########,
    "page 191"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_292,
    r##########"10 ■------------------FRAME
20 MODE 1
30 INK 0,1:INK 1,24
40 GRAPHICS PAPER 0
50 TAG
60 FOR X = 0 TQ 500 STEP 4
70 FRAME ' SYNCHRONISE
80 MOVE X, 200,1
90 PRINT CHR$(32);CHR$(143);
100 NEXT X Essayez le programme ci-dessus sans FRAME. MASK masque,premier point _______________________________________ Permet de tracer des pointillés suivant le “masque” spécifié. Si “masque” est égal à “11110000”, 4 points sur 8 seulement sont affichés.
190 I BASIC AMSTRAD"##########,
    "pages 191, 192"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_293,
    r##########"10 •------------------MASK
20 MODE 1
30 INK 0,26:INK 1,0
40 GRAPHICS PAPER 0 Initialisation 50
60 MASK 8<X1 1 1 10000,1
70 MOVE 100,100,1:DRAW 200,100,1
80 MASK Ml 1 110000,0
90 MOVE 100,120,1 : DRAW 200,120,1 100
110 ' L'instruction 40 est necessaire
120 ' si GPAPER a déjà ete utilise Si “premier point” est égal à 1, il est affiché. Si “premier point” est égal à 0, il n’est pas affiché. GRAPHICS PAPER papier _________________________________________ Détermine la couleur du papier pour les graphiques ou le texte affichés avec TAG. Ci- dessous, les pointillés sont affichés sur fond bleu."##########,
    "page 192"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_294,
    r##########"10 '--------------GRAPHICS PAPER
20 MODE 1
30 PAPER 0:PEN 1
40 INK 0,26: INK 1,0:INK 2,20
50 '
60 MASK Mil 1 00000
70 GRAPHICS PAPER 2
80 MOVE 100,100,1:DRAW 200,200,1 90
100 TAG:PR INT "COUCOU"; GRAPHICS PEN stylo,mode _______________________________________ Détermine le stylo par défaut pour les instructions graphiques. Si “mode” est égal à 1, l’effet de GRAPHICS PAPER est annulé. 10"##########,
    "page 192"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_295,
    r##########"20 MODE 1
30 PAPER O:PEN 1
40 INK 0, 1:INK 1,24: INK 2,20 50
60 GRAPHICS PEN 2
70 PLOT 100,100:DRAW 200,200 80
90 LOCATE 10,10:PRINT "TEXTE Le programme ci-dessous affiche “TEXTE” sur la couleur du papier 2. Essayez le programme avec 50 GRAPHICS PEN 1,1."##########,
    "page 192"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_296,
    r##########"10 ------------------------GRAPHICS PAPER ET PEN
20 MODE 1
30 INK 0,1:INK 1,0:INK 2,20
40 GRAPHICS PAPER 2
50 GRAPHICS PEN 1
60 MOVE 100,100
70 TAG
80 PRINT "TEXTE":
90 '
100 ' ESSAYER AVEC:50 GRAPHICS PEN 1,1 CURSOR mode ___________________________________________________ Lorsque vous utilisez INKEY$, le curseur n’apparaît que si vous avez programmé “CURSOR 1 “CURSOR O” fait disparaître le curseur."##########,
    "page 193"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_297,
    r##########"10 '----------------------CURSOR
20 CLS
30 LOCATE 10,10 CURSEUR EN 10,10
40 CURSOR 1 ' CURSEUR APPARENT
50 C$ = INKEYT: I F Cî="" THEN 50
60 IF ASC(C$)=13 THEN END
70 PRINT C$:
80 GOTO 40 COPYCHRS (# fenêtre)____________________________________________ Fournit le caractère à l’écran sous le curseur."##########,
    "page 193"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_298,
    r##########"10 ■----------------COPYCHRt
20 CLS
30 LOCATE 10,10:PRINT "ABCDE" 40
50 LOCATE 10,10:X$ = COPYCHR$(#O) 60
70 LOCATE 20,20:PRINT Xi Le programme ci-dessous recopie sur imprimante le texte affiché à l’écran (en MODE 2)."##########,
    "page 193"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_299,
    r##########"10 '--------------COPIE ECRAN(TEXTE)
20 FOR L=1 TO 24
30 FOR C=1 TO 80
40 LOCATE C,L
50 C$=COPYCHR$(#0)
60 PRINT #8,C$;
70 NEXT C
80 PRINT #8
90 NEXT L"##########,
    "page 193"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_300,
    r##########"10 ON ERROR GOTO 50
20 OPEN IN "XX"
30 END
40 '--------------Analyse erreur
50 PRINT " ER.R= " ; ERR ; " er 1 = " i ERL
60 PRINT "DERR=";DERR
70 IF DERR=146 THEN PRINT "FICHIER NON TROUVE":END ON BREAK CONT _______________________________________________ Empêche l’interruption de l’exécution d’un programme si l’opérateur appuie sur la touche ESC. Est annulé par ON BREAK STOP."##########,
    "page 194"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_301,
    r##########"10 ON BREAK CONT
20 '
30 FOR 1=1 TO 100
40 PRINT "APPUYER SUR ESC"
50 NEXT I DEC$ (nombre,format) ____________________________________________ Fournit une chaîne de caractères avec le format spécifié."##########,
    "page 194"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_302,
    r##########"10 '--------------D E C $
20 X = 5
30 X=67.8
40 X$= DEC$(X,"####.##")
50 PRINT XT
60 '
70 'CF PRINT USING 67. GO ADAPTATION DES PROGRAMMES P143,148,153 Le mode “FIN” ne provoque pas l’arrêt du programme après la sauvegarde des tables afin de permettre d’effectuer plusieurs sauvegardes sur cassette."##########,
    "page 194"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_303,
    r##########"142 : état du canal non valable
143 : fin de fichier physique
144 : nom de fichier incorrect
145 : le fichier existe déjà
146 : le fichier n'existe pas
147 : le catalogue est plein
148 : la disquette est pleine
149 : la disquette a été changée avec des fichiers ouverts
150 : le fichier peut seulement être lu
154 : fin de fichier logique Lorsqu’une erreur disque se produit, la variable ERR prend la valeur 32. Pour la fin de fichier, nous avons obtenu sur 6128 ERR=24 et DERR=0."##########,
    "page 195"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_304,
    r##########"44 4*5 46 4? 48 0 4 9 1
50 51 3 52 4 S3 & 94 6 55 7
56 8 57 9 58 59 60 61
62 >-63 ■9 64 0 65 À 66 B 67
68 D 63 Ê 70 F 71 72 H I"##########,
    "page 197"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_305,
    r##########"28 K II L 77 M 78 N 79 O P 81 Q R 83 S 84 T 85 U
86 U 87 W 88 X 89 V 90 2 91 C
92 93 ] 94 95 96 97
98 b 99 100 d 10T e 102 f 103
104 h 105 i 106 J* 107 k 1--0--8- 1 1 M
110 n o 112 P 9 114
116 t u 118 w 120 X
122 123 124 125 Q 126 127
128 129 130 » 131 132 133
135 136 137 138 I 139
140 142 143 144 145
146 147 14 8 14 9 150 7. 1
164 165 166 167 168 14 169
170 -4 171 172 173 174 175
176 <X 177 P 178 8 179 6 18© 181 €>
182 183 P 184 TT 185 <r 186 187
188 183 ca 190 E lil fi 192 193
194 195 196 Zs 198 199
200 201 202 2 203 204 205"##########,
    "page 197"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_306,
    r##########"212 213 214 215 216 217 £
218 219 g 2 20 221 222 222>
224 225 es 226 227 228 22:î"##########,
    "page 197"
);
basic_test!(
    test_basic_amstrad_cpc464_664_6128_1_methodes_pratiques_1985__acme__listing_307,
    r##########"248 jj 249 250 251 * 252 253
254 i 255"##########,
    "page 197"
);
