// Tests for BASIC listings from Amstrad_CPC6128_Manuel_de_lutilisateur_1985_AMSOFT_FR_text.pdf
// Each test parses a complete program with OCR error correction

mod common_book_tests;
use common_book_tests::test_basic_program;

/// Fix common OCR errors specific to this book
fn fix_ocr_errors(code: &str) -> String {
    code
        .replace("DRAM ", "DRAW ")
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

basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_001, r##########"10 print "bonjour" [RETURN] Vousremarquerez qu'apres avoir appuye sur [RETURN], bonjour nes'est pasaffiche a Pecran. IIs'estinscritdans la memoiredel'ordinateurpourformerun programmed'une ligne. Pourexecuterce programme, tapez run [RETURN] Vousverrezalors bonjourapparaitrea Pecran.Vouspouvez,sivousledesirez, rcmpla- cer PRINT par un point d'interrogation:
10 ? "bonjour" [RETURN] LIST Lorsqu'un programmeaetemisenmemoire,onpeutverifiercequ'onatapeendeman- dant sa «liste». Tapez list [RETURN] ...et vous voyez apparaitre
10 PRINT "bonjour" ...quiest le programme stocke en memoire. Chap.tre Page 23 Courselementaire 1
12 PRINT"honjour" [RETURN]
20 GOTO 10 [RETURN] ...puis run [RETURN] Vous voyez alors bonjour s'afficher sans interruption, ligne apres ligne, la ligne 20 du programmedemandantaI'ordinateurdereiournera la ligne1 etdepoursuivreI'execu- tion. Pourinterrompremomentanement le programme, appuyezsur [ESC]. Pourcontinuer, actionnez n'importequelleautre touche. Pour I'arretercompletement, actionnez [ESC] deux foisdesuite. Faites els [RETURN] ...pour effacer I'ecran. Pour voir le mot bonjour s'affichersurtoute la ligne, il sufBt de mettre un point-virgule a la fin de la ligne 10, apres lesguillemets. Tapez:"##########, "pages 34, 35");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_002, r##########"18 PRINT"bonjour"! [RETURN]
20 GOTO 10 [RETURNJ run [RETURN] Le point-virgule commande a I'ordinateur d'afficher le prochain groupe de caracteres immediatemeniapresleprecedent(a moinsqu'il nesoit troplongpourtenirsurla ligne). Oiapitre 1 Page 24 Courselementaire"##########, "page 35");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_003, r##########"10 PRINT"bonjour" [RETURN] f run [RETURN! Lavirguleadcmandea1'ordinateurd'afficherleprochaingroupedecaracteres 13colon- nes apres le debut de celui-ci. Cette fonction permet d'afficher lesdonnees en colonnes. C es e t p d e e n c d a a l n e t d , e si 1 1 3 c a n u o t m r b es re co d l e on c n a e r s a . ct a e f r i e n s d i e nd t u ou s j d o a u n r s sm u e n n g a r g o e u r pe un de c p s a p s a s c e e 1 e 2 n , t l r e e g l r e o s u g p r e ou s p ui e v s a d nt e caracteres. Latailledeceszonesde 13caracterespeutetremodifieeparla commandeZONE,decrtte plusloin dans le manuel. Pour sortirdu programme, appuyez deux fois sur [ESC]. Pour vidcr completemen, la memoire, remettez 1'ordinateur a Eero en appuyant sur [CONTROL], [SHIFT] et iem,j, dans cet ordre. INPUT Cettecommandesertainformer1'ordinateurqu'ildoitattendrequeTonaittapequetque chose avant de continuer. Par exemple : IB INPUT "Quel age avez-vous"sage [RETURN]
20 PRINT"Vous paraissez nettement moin& que vos";age; "|arts. ^ [RETURN] run [RETURN] Sur1'ecran on voit Quel age avez-vous? Si vousdonnez votreage, meltons 18, puis [RETURN], on voit alors s'afficher Vous parraissez netteaent moins que vos 18 arts. Cet exemple montre I'utilisation de la fonction input et d'une variable numerique. Le mot age est mis en memoire a la fin de la ligne 1 pour que l'ordinateur 11 associe a toutnombretape apres le pointd'interrogation,afin dc procedera I'affichagede la ligne 20. Bien quenous ayons utilise le mot agepourla variable age, nousaunonspuaussi bien prendre la lettrea ou b... Cours^mentaire Chapitre l Page ?S"##########, "page 36");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_004, r##########"10 INPUT "Quel est ton noiTjno** CRETURN]
20 PRINT"bonjour ";nofn$5" mon nom est Roland" [RETURN] run CRETURN] Sur I'ecran, on voit Quel est ton nom? Tapez votre nom puis [RETURN] Si votre nom est Fred, vous verrez sur I'ecran : bonjour Fred non no* est Roland Bien que nous ayons utilise nom$ comme variable chaine, nous aurions aussi bien pu utiliser a$. Nous allons maintenant combiner les deux exemples precedents en un seul programme. Faisons a nouveau [CONTROL] [SHIFT] et [ESC]. Puis tapons le programmesuivant"##########, "page 37");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_005, r##########"10 INPUT "Quel est ton nom";a* CRETURN]
20 INPUT "Quel est ton age";b CRETURN] ..
30 PRINT"Je dois dire ";a$;" que tu ne fais pas tes";b; "arts. " [RETURN] run [RETURN] Dans ce programme, nousavons utilisedeux variables, a$ pourle nom et b pour Page. Sur I'ecran on voit Quel est ton nom? Tapezvotre nom (Fred) puis [RETURN]. La question suivanteapparait Quel est ton age? Tapez maintenat votreage(18) puis [RETURN]. - , -'- -'•-": — • : . — -..—m — .in. • ; •-i.-1>i£.-itoi-^1,>ft;,ivv;^.-T'^--? Chapitre 1 Page 26 Ccmrs61§mentaire"##########, "page 37");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_006, r##########"18 INPUT "Quel est to nonT'sa* [RETURN]
20 INPUT "Quel est ton age";b [RETURN!
30 PRINT"Je dois dire"}a*i" que tu ne fais pas tes";b; "ans. " [RETURN] Troiserreurs se sont glissees dans le programme ci-dessus Dans la ligne 5, on a tape clss au lieu de els Dans la ligne 1 0, on a tape to au lieu de ton Dans la ligne30, on a oublie l'espacement entre direet lesguillemets(" IIexistetroismethodespourediterunprogramme. La premiereconsistearetaperentiere- mentla ligne. Quanduneligneest retapeeetentreeenmemoire,elleremplacela lignequi portait le meme numero. Ladeuxieme methodeconsiste a editera I'aide ducurseur. Laderaiere est appeleeCopy Cursor, autrement dit CopieavecI'aidedu Curseur. Methode d'edition a I'aide du curseur Pour corriger la ligne 5, tapez edit 5 [RETURN] La ligne 5 apparait sous la ligne 30, lecurseur place sur le c de clss. PourenleverleSen tropdans clss, appuyezsurla touchecurseurdroitejusqu'aceque celui-ci soitsur lederniers,puisappuyez surla touche [CLR]. Le s a disparu. Courselemenlaire Chapitre 1 Pag« 27"##########, "page 38");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_007, r##########"10 INPUT "quel est ton no*"sa* [RETURN]
28 INPUT "quel age as~tu";age [RETURN]
30 IF age < 13 THEN 60 [RETURN]
40 IF age < 20 THEN 70 [RETURN] : '
50 IF age > 20 THEN BO [RETURN]
60 PRlNT"donc ";a$;" tu n'est pas encore un adolescent avec te5";age;"an5,";£ND [RETURN]
70 PR INT "done ";a$;" tu es un adolescent avec te»";age; "ans. ":END [RETURN]
80 PRINT"eh bien ";a$;"tu n'est done plus un adolescent a vec tes"; age; "ans. " [RETURN] Pourverifierquece programmeest correct, faites list [RETURN] ...puis tapez run [RETURN] Vous pouvez maintenant repondre aux questions de Pordinateur et observer ce qui se passe. Vouspouvezconstater leseffetsdu IF (si)etdu THEN (alors)commecommandesdans un programme. Nousavionsaussi ajoute le mot END (fin)a la fin des lignes 60 et70. Ce mot reserve END est utilise pour mettre fin a un programme. S'il n'etait pas la, le programme continuerait a avancer et afficherait aussi les lignes 70 et 80. Courseiementaire Chapure 1 Page 29"##########, "page 40");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_008, r##########"10 FOR a=l TO 10 [RETURN]
20 PRINT"ACTION executee" }a; M foi s" [RETURN!
30 NEXT a [RETURN] run [RETURN! Vous remarquerezquel'instructiondela ligne 20 aeteexecuteedix fois, comme lespe- cific la commande FOR de la ligne 10. Vous constatez egalement que la valeur de la variableest incrementcede l a chaque passage dans la boucle. Lemotcle STEP peutservira definirlepasde la commande FOR NEXT, Parexemple, vous pouvez remplacer la ligne 1 par :"##########, "page 41");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_009, r##########"10 FOR a=10 TO 50 STEP 5 [RETURN] run [RETURN] Vous pouvez aussi fixer des pas negatifs
10 FOR a=100 TO STEP -10 [RETURN] run [RETURN] REM REM est l'abreviation de REMark. Cette instruction demande a 1'ordinateurd'ignorer REM toutcequisuitsurla ligned'instruction. peutainsi servira introduiredesinforma- tions : le titred'unprogramme ou 1'utilisation d'une variable parexemple :
10 REM Pan sur les envahisseurs [RETURN]
20 V=5:REM nosibre de vies [RETURN] Chapitre 1 Page 30 Courselementaire"##########, "page 41");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_010, r##########"10 ' Pan sur les envahisseurs [RETURN]
20 V=5' nombre de vies [RETURN] GOSUB Si un ensembledestructionsdoitetreexecuteplusieurs fois, vouspouvezeviterdereta- per les instructionsen creant un «sous-programme»dont 1'appel se faitgracea lacom- mande GOSUB. suiviedu numerode ligne requis. Lafin du sous-programmes'indique par RETURN. L'ordinateur passe alors a Instruction qui suit immediatement la com- mande GOSUB qu'il vient d'executer. (Les deux programmes suivants ne «font » rien d'autre qu'afficher des mots a I'ecran. Comme ils neservent qu'a illustrer Ie fonctionnementdes sous-programmes, il n'cst pas indispensablede lestaper). Par exemple, dans ce programme :"##########, "page 42");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_011, r##########"18 MODE 2 [RETURN]
20 PRINT"Darae souris trotte," [RETURN]
30 PRINT"Noire dans le oris du soir," [RETURN]
40 pRINT"Dame souris trotte," [RETURN]
50 PRINT"Grise dans Je noir." [RETURN]
60 PRINT [RETURN]
70 PRINT"Un nuage passe," [RETURN]
80 PRINT" 11 fait noir coffline en un four" [RETURN]
90 PRINT"Un nuage passe," [RETURN]
100 PRINT"Tiens,le petit jour !" [RETURN]
110 PRINT [RETURN]
120 PRINT"Dame souris trotte," [RETURN]
130 PRINT"Rose dans les rayons bleus," [RETURN]
140 PRINT"Dame souris trotte," [RETURN3 15B PRINT"Debout paresseux "' [RETURN]
160 PRINT [RETURN]
170 PRINT" P.Verlaine" [RETURN] - run [RETURN] Courselementaire Chapitte Page 31 1"##########, "page 42");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_012, r##########"10 MODE 2 [RETURN]
20 GOSUB 190 [RETURN]
30 PRINT"Noire dans le gris du soir," [RETURN]
40 GOSUB 190 [RETURN]
50 PRINT"Grise dans le noir," [RETURN]
60 PRINT [RETURN]
70 GOSUB 210 [RETURN]
80 PRINT"I1 fait noir coflste en un four" [RETURN]
90 GOSUB 210 [RETURN]
108 PRINT"Tiens,le petit jour !" [RETURN]
110 PRINT [RETURN]
120 GOSUB 190 [RETURN]
130 PRINT"Rose dans les rayons bleus," [RETURN]
140 GOSUB 190 [RETURN]
150 pRlNT"Debout paresseux !" [RETURN]
160 PRINT [RETURN]
170 PRINT" P.Verlaine" [RETURN]
180 END [RETURN]
190 PRINT"Darae souris trotte," [RETURN]
200 RETURN [RETURN]
210 PRINT"Un nuaqe passe," [RETURN]
220 RETURN [RETURN] run [RETURN] Nous avonsainsi gagne un temps precieux ! Lessous-programmes bien pensessont une partieessentielledela programmation. lisdebouchentsurdesprogrammes«structures» et sont un bon reflexede programmation. Lorsque vouscreez des sous-programmes, rappelez-vous qu'il vous est possible d'avoir plusieurs points d'entree. Par exemple, un sous-programme occupant les lignes 500 a"##########, "page 43");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_013, r##########"10 Turquoise 24 Jaune Vif
11 Bleu Ciel 25 Jaune Pastel
12 Jaune 26 Blanc Brillant
13 Blanc Tableau ! : Les numerosd'encre(INK)et lescouleurs Commenousledisionsauparavant,l'ordinateuresten mode 1 a Porigine. Pouryrevenir a partir d'un autre mode, tapez : mode [RETURN] I Chapitre 1 Page 48 Courselementaire"##########, "page 59");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_014, r##########"10 14 20 1
11 16 6 24
12 18 1 1
13 22 24 24
14 cIignotementI,24 20 1
15 clignotementl6,ll 6 24 Tableau2. ReferencePAPER/PEN/MODE/INK Chapitre 1 Page SO Courselementaire"##########, "page 61");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_015, r##########"10 MODE [RETURN]
20 vi tesse=600!REM Determine la vitesse du programme [RETURN]
30 FOR b=0 TO 26 [RETURN]
40 LOCATE 1,12 [RETURN]
50 BORDER [RETURN] b
60 PRINT"COULEUR DU CADRE";b [RETURN]
70 FOR t=l TO vitesse [RETURN]"##########, "page 65");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_016, r##########"30 NEXT t,b [RETURN]
90 CLG [RETURN] "*
100 FOR 0=0 TO 15 [RETURN]
110 PAPER p [RETURN]
120 PRINT"PAPIER"ip [RETURN]
130 FOR n=0 TO 15 [RETURN] .
140 PEN n [RETURN]
150 PRINT"STYLO";n [RETURN]
160 NEXT n [RETURN]
170 FOR t=l TO vitesse*2 [RETURN]
180 NEXT t,p [RETURN]
190 MODE [RETURN] 1
200 BORDER [RETURN] 1
210 PAPER [RETURN]
220 PEN 1 [RETURN]
230 INK 0,1 [RETURN]
240 INK 1,24 [RETURN] run [RETURN] IMPORTANT Dansce programme,commedansleschapitres suivantsetleslistagesdu manuel, les mots cles du BASIC apparaissent en majuscules. I! s'affichent ainsi sur la demande d'une instruction LIST. II est generalement preferable dc taper les instructionset les programmesen minusculespourmieuxdetecterlescrrcursde frappe lorsdes listages desprogrammes(lesmotsclesmalorthographiesnesontalorspasconvcrtisenmajus- cules). Jusqu'a la fin dececourselementaire, les programmessont listesen majusculeset en minuscules afin de vous famiiiariseravec ce procede. Un nom de variable, x ou S par exemple, ne sera pasconverti en majuscule lors du LISTage du programme, bienqu'il soit reconnu par le programmede toute facon. Attention A partirdemaintenant,nousnepreciseronsplusdetaper[RETURN!alafindechaque ligne. Nous supposonsquevousen avez pris le reflexe. Chapitre 1 Page 54 Cours616mentaire"##########, "page 65");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_017, r##########"10 FOR n=32 TO 255
20 PRINT n;CHR$(n))
30 NEXT n run Vous trouverez au chapitre «Pour information...» tous les caracteres accompagnes de leurs numeros. LOCATE (place lecurseur) On sesertdccette commandepourmettrelecurseura uncertainendroitdeI'ecran.Sans la commande LOCATE, lecurseursetrouveenhautetagauchedeI'ecran,cequicorre E s n - pondencoordonneesx,ya 1.1 (xetant la positionhorizontal,ylapositionverticale). Mode 1, il y a 40 colonnes et 25 lignes. Pour placer un caractere en haut au milieu de Fecran, il faut indiquerx = 20et y = 1. Essayons : mode 1 ...Fecran s'efface; lecurseurse trouveen haut, a gauche. IB LOCATE 20,1"##########, "page 66");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_018, r##########"10 CLS
20 FOR X=l TO 39
30 LOCATE X,20
50 PRINT CHR*(250)
60 NEXT X
70 SOTO 10 run Faites [ESC] deux fois pour interrompre le deroulement. Chapitre 1 Page 56 Cours elementaire"##########, "page 67");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_019, r##########"40 FRAME Lacommande FRAME synchronisele mouvementde1'objetaffichesurla frequencede balayage de la trame d'affichage. Si cette notion vous parait trop technique, rappelez- vous simplement que cctte commande sert a deplacer harmonieusement sur lecran des caracteresou des graphiques. On peut encore ameliorer ce programme en ajoutant des petites pauses et en utiltsant d'autrescaracteres. Pource faire, tapez : list Puisajouter les lignessuivantes au programme :
70 FOR n*l TO 300:NEXT n
80 FOR x=39 TO STEP -1 i
90 LOCATE x,20
100 FRAME
110 PRINT CHR*(25l)5" "
120 NEXT x
130 FOR n=l TO 300:NEXT n
140 GOTO 20 run Coursetementaire Chapiire 1 Page 57"##########, "page 68");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_020, r##########"10 PLOT 10,10
20 DRAW 10,390
30 DRAW 630,390
40 DRAW 630, 10
50 DRAW 10,10
60 GOTO 60 run Actionnez [ESC] deux foispoursortirduprogramme. (Dansce programme, I'ordinatcurboucle indefinimenta la Kgne 60jusqu'a ce que vous 1'interrompiez en appuyant deux fois sur la touche [ESC]. Cc type ^instruction evite 1'interruption automatiqucdu programme apres ladernicre ligne et 1 affichage du mes- sage Ready). AjoutezmaintenantleslignessuivantespourdessinerundeuxiemerectangleaI'interieur du premier
60 PLOT 20,28
70 DRAW 20,380
80 DRAW 620,380
90 DRAW 620,20
100 DRAW 20,20
110 GOTO 110 run Appuyezsurdeux fois [ESC] poursortirdu programme. MOVE La commande MOVE fonctionne comme PLOT, c'est-a-dire que le curseur graphique se place sur le point descoordonnees x,y sans tracerle pixel au nouvel emplacement du curseur. Tapez el* ove 639,399 BienquenousnelevoyonspasaI'ecran,lecurseurgraphiquesetrouvemaintenantdans Tangle superieurdroit. Mettonscettepositionenevidenceentracantuneligneapartirdecepointverslecentre de I'ecran. Tapez draw 320,200 Course,t.,ement „ a . i „ r „ e ChapiKtjre 1 Page 59"##########, "page 70");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_021, r##########"10 CIS
23 DEG 4
33 FOR a=l TQ 340
40 MOVE 320,200
50 DRAW 320+190ICOSUI ,200+190*SIN(a)
60 NEXT run Chapitre 1 Page 60 Courselementaire"##########, "page 71");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_022, r##########"20 en tapant 20 Pourdessineruncercleplein(undisque)avecdes lignestraceesa partirducentre, modi- He*(EDIT) la ligne 50 en remplacant le mot PLOT par le mot DRAW. Elledevient
50 DRAW 320M?0tCQSU),200+190»SINU> Essayez maintenant avec et sans la ligne 20. Notezquedansla ligne 60deceprogramme, NEXT n'est passuividea. On peutmettrc seulement NEXT : 1'ordinatcurdeterminealorsa quel FOR il se rapporte. Dans les pro- grammes contenantde nombreux FOR ct NEXT, il est toutefois preferabled'ajouterla variableapres NEXT afin dc mieuxs'y retrouver. ORIGIN MOVE Dansle programmeprecedent,ons'estservidela commande pourfixerlecentre du cercle, puis on a ajoute lescoordonnees x,y a cellcs du centre. Au lieu decela, nous pouvons utiliser la commande ORIGIN (attention, pas de E a ORIGIN, e'est un mot reserve). Ellaplacelescoordonneesxetydechaquepointen fonclion d'ORIGIN. Veri- fionsccci avecle programme : new"##########, "page 72");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_023, r##########"10 CLS
20 FOR a=l TO 360
30 ORIGIN 320,200
40 PLOT 190*COS(a) 190»SIN<a) ,
50 NEXT run Cours6tementaire Chapitre t Page 6!"##########, "page 72");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_024, r##########"10 CLS
20 FOR a= l TO 360
30 ORIGIN 196,282
40 PLOT 50*COS(a),50*SIN(a)
50 ORIGIN 442,282
60 PLOT 50*COSU),50*SIN(a>
70 ORIGIN 196, Hi
80 PLOT 50tCQSU},50*SIN(»>
90 ORIGIN 442, 116
100 PLOT 50*COSU),50*SINU> ,-
110 NEXT run Pourexperimenter une autre methodedecreation d'uncercle, tapez Ie programme : new"##########, "page 73");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_025, r##########"10 MODE 1
20 ORIGIN 320,200
30 DEG
40 MOVE 0,190
50 FOR a=0 TO 360 STEP 10
60 DRAW 190*SIN<a),190*COS(a)
70 NEXT run Cettefois, uneligneesttracee (DRAW)decoordonneeen coordonneesurPensemblede la circonference. Le tracageest nettement plus rapide que Ie positionnementdes points. Unefoisencore, observe? la consequencedu retrait de la commande DEG parsuppres- sion de la ligne 30 et relancez le programme. FILL La commande FILLsertaremplir unezoned'ecrandelimiteeparunecourbe, le bord de l'ecran, ou d'une fenetregraphique. Reinitialiseza l'aidedes touches [CONTROL] [SHIFT] et [ESC], puistapez new"##########, "page 73");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_026, r##########"10 CLS
20 MOVE 20,20
30 DRAW 620,20
40 DRAW 310,380
50 DRAW 20,20 run 3hapitre Page 62 Courselementaire 1"##########, "page 73");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_027, r##########"50 DRAW 50,50
60 MOVE 320,200
70 FILL 3 run Toute interruption de lignes sur Pecran laisse « passer» la couleurdu stylo. Ce phenomene est illustre par le remplissaged'un cercle positionne a Pecran, puisd'un cercle trace. Tapez new"##########, "page 74");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_028, r##########"10 CLS
20 FOR a=i TO 360
30 ORIGIN 320,200
40 PLOT l90tCOS(a), 190ISINU)
50 NEXT
60 MOVE -188,0
70 FILL 3 run Cowselementdire Chaprtre 1 Page 63"##########, "page 74");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_029, r##########"10 MODE 1
20 ORIGIN 320,200
30 DEG ; /"-
40 MOVE 0,190
50 FOR d= TO 360 STEP 10
60 DRAW 190*COS(d),190ISlN(d)
70 NEXT
80 MOVE -188.0
90 FILL 3 run Vouspouvezrendrelacirconferenceducercleinvisibleenutilisant la memecouleurpour le stylo etle papier. Ajoutezcette ligne :"##########, "page 75");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_030, r##########"10 MODE 0: BORDER 13
20 MOVE 0,200:DRAW 640,200
30 FOR x=80 TO 560 STEP 80
40 MOVE x,0:DRAW x,400
50 NEXTsMOVE -40,300
60 FOR c=0 TO 7
70 MOVER 80,0:FILL c
80 MOVER 0,-200sFILL c+8
90 MOVER 0,200:NEXT
100 GOTO 100 run Lescouleursdes zonesen plein peuvent etre modifiees apres leur remplissage.Tapez :
100 SPEED INK 30,30
110 BORDER RNDI26
120 INK RN0U5,RNDI26,RND*26
130 FOR t=t TO 500: NEXT:GOTO 110 run Chapitre 1 Paye 64 Cours61&nentaire"##########, "page 75");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_031, r##########"10 BORDER 0;GRAPHICS PEN t
20 m=CINT (RND*2)sM0DE m
30 il=RND«26ii2=RND*26
40 IF ABS(il~i2)<i0 THEN 30
50 INK B,il: INK 1, i2
60 s=RND*5+3;QRIGIN 320,-100
70 FOR x=-1000 TO STEP s"##########, "page 76");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_032, r##########"60 MOVE 0,0;DRAW x,300:DRA»l 0,688
90 MOVE 0,0:DRAW -x,300:DRAW 0,680
180 NEXTrFOR t-\ TO 2000:NEn :G0TO 28 run"##########, "page 76");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_033, r##########"10 rlODfe 1 1 BORDER 0IHAP6R 1
20 GRAPHICS PEN 2: INK 0,0
30 EVERY 2200 GOSUB 150
40 FLAG=0:CLG
50 INK 2.-14+RNDH2
60 B'/,=RND*5+1
70 C7.=RNDt5+l
80 ORIGIN 320,200
90 FOR A=0 TO 1000 STEP PI/30
100 X"/.=100*CQS(A)
110 MOVE XX, X"/.
120 DRAW 288tC0S(A/BX),2B8tSIN(ft/C7.)
130 IF FLAGM THEN 40
140 NEXT 1-58 FLAG=llRETURN run Courselementaire Chapiire Page 65 ]"##########, "page 76");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_034, r##########"10 MODE 1: BORDER 0:DEG
20 PR1NT"VEUILLEZ PATIENTER
30 FOR N=l TO 3
40 INK 0,0:INK 1,26: INK 2, 6: INK 3, IS
50 IF N=! THEN SA=120
60 IF N=2 THEN SA=135
70 IF N=3 THEN SA=150 B0 IF N=l THEN ORIGIN 0,-30,0,640,0,100 ELSE ORIGIN 0,0,0,64 0,0,400
90 DIM CX(5 CY(5),R(5),LC(5)
100 DIM NP(51
110 DIM PXX(5,81),PYX 5,81)
120 ST=1:CX(1)=320:CY 1J=200:R(U=80
130 FOR ST=1 TO 4
140 R(ST+l)=R(ST)/2
150 NEXT ST
160 FOR ST=t TO 5
170 LC(ST)=0:NP(ST)=0
180 NP(ST)=NP(ST)+1
190 PXX(ST,NP(ST))»R(ST)tSIM(LC(ST))
200 PY7.(ST,NP(ST))=R(ST)*C0S(LC(ST)!
210 LC(ST)=LC(ST)+360/R(ST)
220 IF LC(ST)<360 THEN 180 4
230 PX7.(ST,NP(ST) +l)=PXX(ST,l>
240 PY7.<ST,NP(ST)+1)=PY7. (ST, I)
250 NEXT ST 4
260 CLS:CJ=RESTE(i): CJ=RESTE(2)
270 CJ=RESTE(3) INK l,2:ST=l j
280 GOSUB 350
290 LOCATE 1,1
300 EVERY 25,1 GOSUB 510
310 EVERY 15,2 GOSUB 550
320 EVERY 5,3 GOSUB 590 - , . '
330 ERASE CX,CY,R,LC,NP,PXX,PYX!NEXT
340 GOTO 340
350 CX,/.=CX(ST!:CY7.=CY(ST):LC(ST)'=0
360 FOR XV. = 1 TO NP(ST)
370 MOVE CX7,,CYX
380 DRAW CXX+PXX<ST,XX) CYX+PYX(ST,XX),l+<ST MOD 3)
390 DRAW CX7.+PX7.(ST,X7.+ t n,CY7.+PY7.(ST,xy.+t! l+ , (ST MOD 3)
400 NEXT XX >
410 IF ST=5 THEN RETURN
420 LC(ST)=0
430 CX(ST+lt=CX(ST)+l'. 5IR(ST)*SIN(SA+LC(ST) )
440 CY(ST+1)=CY(ST)+1.5*R(ST)*CQS<SA+LC(ST>)
450 ST=ST+1
460 GOSUB 350
470 ST=ST-1
480 LC(ST)=lC(ST)+2tSA
490 IF (LC(ST) MOD 3601O0 THEN 430
500 RETURN Chapitre 1 Page 66 Courselementaire
510 1K(1)=1+RND*25
520 IF IKU)"IK<2) OR IK(1)«IK(3> THEN 510
530 INK 1,IK(1>
540 RETURN
550 IK(2)=1+RND»25
560 IF IK(2)=IK(1) OR IK(2)"IK(3> THEN 550
570 INK 2, IK(2)
580 RETURN
590 IK(3)=l+-RND*25
600 IF IK (3)=IK (1> OR EK(3)>IK<2) THEN 590
610 INK 3, IK(3)
620 RETURN Courselementaire Chapitre 1 Page 67"##########, "pages 77, 78");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_035, r##########"10 SOUND 1,239,200 run La meme notedo dure 2 secondes. >3*vt:i;;-.v Volume Ce parametredeterminelevolumededepartd'unenote. Cenombrc variedc a15. correspond au volume minimum et 15 au volume maximum. En I'absence de specifica- tion, I'ordinateurfixe un chiffredc12. Tapez :
10 SOUND 1,239,200,5 run Remarquez le volumedu son, puis tapez un numerodevolume superieur
10 SOUND 1,239,200,15 run Vousdevez obtenir un son plusfort. Cours61ementaire Ch«pttre ! Page 06"##########, "page 80");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_036, r##########"10 ENV 1,10,1,100
20 SOUND 1,142,1000,1,1 run Chapitre Page 71 Courselemenwire 1"##########, "page 82");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_037, r##########"10 pas, eievant chaque pas d'une unite de volume et durant une seconde (1 00 x 0,01 sec). Change? la ligne 10 de la maniere suivante et faites run chaque fois pour entendre la difference
10 env I, 100, 1,10
10 env 1,100,2,10
10 env 1,100,4,10
10 env 1,50,20,23
10 env 1,50,2,20
10 env 1,50, 15,30 Essayez finalement
18 ENV 1,38,2,18 Vouspouvezconstaterqueleniveaudeson resteconstantami-chemin. Eneffet,lenom- bre de pas (50) et I'ecart entre les pas (0,1 seconde) donnenl unevariation du son de 5 secondes seulement, alorsque la <duree> du sondans lacommande SOUND (la ligne 20)est de 1 000, soit 10 secondes. Experimentezles typesde sons que vous pouvezcreer. Si vous le destrez. vous pouvez creer des enveloppes de volume plus complexes. Les 3 parametres <nombre de pas>, <amplitude du pas> et <duree du pas> peuvent etre reprisjusqu'aquatrefoisala findelacommande ENV,permettant despecifierdespor- tionsd'enveloppedifferentes. Creation d'une enveloppe de tonalite La commanded'enveloppede tonaliteest ENT, Sous sa formedebase, ellea4parame- tres. Elle sc presente sous la forme : ENT <numerod'enveloppe>, <nombredepas>, <variationdeperiodesonore affecteea chaquepas>, <dureedu pas> Regardonsces parametres un par un. Numerod'enveloppe C'est lc numerod'appel pourla commandeSOUND. II peut prendre une valeurde a 15. Chapitre 1 Page 72 Courselementaire"##########, "page 83");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_038, r##########"10 ENT 1,100,2,2
20 SOUND 1,142, 200,15,, run La ligne 20 specifie un son ayant une periode de 142 (LA international), durant 2 secondes avec un volume de depart de 15 (max), depourvu d'enveloppe de volume (constatez l'omission du parametre : .,) maisdoted'une enveloppede tonalite N° 1 La ligne 10 affecte au N°1 uneenveloppe de tonaliteconstituee de 100 pas, augmen- tant la periode dc 2 unites a chaque pas et ayant chacun une duree de 0,02 seconde (2 x 0.01 s). Cours£ldmentaire Chapitre1 Page 73"##########, "page 84");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_039, r##########"10 ent 1,100,-2,2
10 ent 1, 10,4,20
10 ent 1,10,-4,20 Remplacez la commande SOUND et 1'enveloppe de tonalite en tapant :
10 ENT 1,2,17,70
20 SOUND 1,71,140,15,,!
30 60T0 10' run Appuyez deux fois sur [ESC] pour interrompre le programme. Vous pouvez maintenant combiner Penveloppe de volume, I'enveloppe de tonalite et la commande SOUND pour obtenir des sonsdifferents. Commencez par taper :"##########, "page 85");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_040, r##########"10 ENV 1, 100,1,3
20 ENT 1,180,5,3
30 SOUND 1,142,300,1,1,1 run 20 Puis remplacez la Iigne par"##########, "page 85");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_041, r##########"10 ENV 1,180,2,2
20 ENT 1,100,-2,2
30 SOUND 1,142,200,1,1,1 run Si vousdesirezcreerdesenveloppesdetonalitepluscomplexes, vous pouvezrepeterjus- qu'a quatre fois les trois parametres <nombre de pas>, <variation de periode sonore : affecteeachaquepas> et <dureedupas> a la findela commande ENT afindespecifier des portionsdifferentespourunememeenveloppe. Essayezplusieurs versions. Ajouteza la commande SOUND le parametre permettant d'ajouter du bruitage et des portions t supplementairesd'enveloppe de tonalite et de volume. 4 Le chapitre « Liste complete des mots cles du BASIC AMSTRAD 6128 » decril en detail les commandes de son. Si vous desirez decouvrir les possibilites sonores de votre ordinateur, reportez-vousauparagraphe «Sonset musique»du chapitre«A vosheures i de Ioisir...». 4 4 CSiapitre I Page 74 Courselementaire 9"##########, "page 85");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_042, r##########"18 REM repertoire telephonique Donner un titre n'est pas une obligation. II vous aidera pourtant beaucoup a vous y retrouverlorsque vousaurezaccumule unequantitede programmes. Nousvoulonsdoneenlrer(INPUT)unechainedecaracteres(unnom)dansunevariable. Cettevariable, nous l'appellerons NOM$. De meme, nous aurons une variable pour les numeros de telephone, que nous appelleronsTEL$. Vous vous souvenez que, dans les exemples de programmes du Cours Elementaire, I'mstruction INPUT vous avait permis d'affecter une valeura une variable. Ainsi, avec les Iignes
20 INPUT "entrez le nofli"?N0«*
30 INPUT "entrez le numero de telephone"}TELf run ...nous pourrons entrer un nom (disons « Paul »), puis un numero de telephone (par exemplc 1 206 66 60). Ces informations sont maintenant stockees par le programme, mais nous n'obtenons encoreaucunresultata Fecran.IInousfautdoneecrireunepartiedeprogrammepermet- NOM$ tantderetrouveret d'affichercesinformations. Pourobtenirla valeuractuellede et TEL$, on aura recoursaux instructions : PRINT NOM* ... et, . PRINT T£L* Chapitre2 Page 2 Passonsauxchosesserieuses."##########, "page 105");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_043, r##########"20 DIM NOM* (100)
38 DIM TEL* (1801 Nos variablesainsidefinies,ecrivonsun programmepermettantd'introduireles nomset numerosde telephonedans les tableaux (nousverrons plus tardcomment lesretrouver). Ajoutons les lignes
48 FOR x=l TO 180
50 INPUT "entrez le noa'iNQM* (x >
60 INPUT "entrez le nuaero de telephone"jTEtf(x>
70 NEXT run Tout celaest fort bien, mais nous n'avons pas Tintention d'entrer les 100 noms en une seule fois. Dc plus, la presentation du programmea l'ecran laisse beaucoup a desirer. II s'agit maintenant d'ymettreunpeud'ordre. Pourcommencesnousallons,avantchaque nouvelle entree, debarrasser l'ecran du tcxte anterieur devenu inutile. C'cst I'arTairc de CLS 1'instruction :"##########, "page 106");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_044, r##########"55 If- NQflStx )="" THEN 90
80 PRINT"entree des donnees terminee" Le programme doit lui-meme indiquer a I'utilisateur commentTinterrompre. Ajoutons done"##########, "page 107");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_045, r##########"47 PRINP'pour arreter appuver [ENTER]" Voyonsmaintenantcomment obtenirTimpressiondesinformationsenregistrees,d'abord sous forme de liste. Ecrivons
90 FOR X*I TO 180
100 PRINT N0M*(x)?" ";TEL$(x)
110 NEXT Maislaencore le programmenesait pascomment s'arreteravantlecentiemeelementdu tableau, Ajoutons done"##########, "page 107");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_046, r##########"95 IF N0M$(x)="" THEN 120
120 PRINT "liste terminee" La chaine vide est detectee en ligne 95 et le programme interrompt Pimpression en 00 sautant les lignes 1 et 11 0. Passons maintenanta notreobjectifsuivant ; nousvoulonsqueleprogrammesoitcapa- ble de rechercher un nom entre au clavier. Ecrivons : l',0 INPUT "nom a trouver : "'.RECHERCHE*
140 FOR x=l TO 100
150 IF lNSTR(NClM*fx).RECHEfcLHE*i=0 THEN 1.90 la0 PRINT MOMS k > ; " ":TEL*(k)
170 END"##########, "page 107");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_047, r##########"130 NEXT
190 PRINT "Lb n'jin r. >^i r-a< ren <?r t c;>i*? *" run Chapitre2 Page 4 Passonsauxchosesserieuses. ."##########, "page 107");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_048, r##########"32 PRINT"!, ajouter un correspondant "
33 PRI N T "2 . lister les correspondants"
34 PRINT"3, consulter le repertoire"
35 PR1NT"4. sauvegarder le repertoire"
36 PR INT"5 charger le repertoire" .
37 INPUT "votre choix (puis ENTER) "jch
38 ON ch GQSU8 40,98,138
85 RETURN
125 RETURN
178 RETURN
200 RETURN Commevous pouvezleconstater, le programmeaffichemaintenant le menudesoptions. puis introduit dans la variable ch le numero entre au clavier (INPUT). En passant sur Instruction ON ch GOSUB (ligne38),il lancera lepremiersous-programme(ligne38) si ch= 1, le deuxieme(ligne 90) si ch = 2etainsi desuite. Maintenant que chacune des fonctions est devenue un sous-programme, il faut obliga- toirement en indiquer la fm par une instruction RETURN, ce que nousavonsfait. Vous souvenez-vous du mode d'execution de RETURN ? A lafin du sous-programme, cetteinstruction BASICrenvoiele programmea la lignesuivantimmediatementl'instruc- tion GOSUB correspondante, c'est-a-dire, dans notre cas, a la ligne siluee apres 38 (notreprogrammesepoursuivraitdoneapartirdupointd'entreedesinformations,ligne 40). Pourevitercela, introduisons la ligne :"##########, "page 109");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_049, r##########"10 REM repertoire telephonique
20 DIM NfJM*<100>
30 DIM TEL4U00)
32 PRINT" ajouter un correspondent 1>
33 PRINT112. lister les correspondants"
34 PRINT".', consulter le repertoire"
35 PRINT"4. sauvegarder le repertoire" Chapilre2 Page 6 Passonsauxchosesserieuses...
36 PRINT'S, charger le repertoire"
37 INPUT"votre choix (puis £NTER)"sch
38 ON ch GDSUB 40,90,130
39 GOTO 32
40 FOR x=L TO 100
45 CLS
47 PRlNT"pour arreter appuyer [ENTER]"
50 INPUT "fiom";NQM*(xl
55 IF N0M$(x!="" THEN 80
60 INPUT "telephone"«,TEL$(x)
70 NEXT B0 PRINT"entree des donnees terminee."
85 RETURN
90 FOR k=1 TO 100
95 IF NQM$(x)="" THEN 120
100 PRINT NQM*(x)j" "}TEL*(x)
110 NEXT
120 PRJNT"-fin de liste"
125 RETURN
130 INPUT "now a trouver "; RECHERCHE!
140 FOR x=t TO 100
150 IF INSTR(NOM*(x),RECHERCHE*!=0 THEN 180
160 PRINT NOM* (x) ; " "}T£L*(x)
170 RETURN
180 NEXT
190 PRINT"ce nom n'est pas repertory."
200 RETURN Vousremarquezqu'acertainsendroitsnouscommenconsa manquerdeplace pourinsu- RENUM- rer de nouvellcs lignes. Nous allons en crcer et remettre un peu d'ordre en erotant les lignes. Faites : RENUM LIST Vousdevez maintenant obtenir"##########, "pages 109, 110");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_050, r##########"18 REM repertoire teiephonique
20 DIM NOM$U80)
30 DIM TEU$(100)
40 PRINT" ajouter un correspondant i .
50 PRINT"2. lister les correspondants"
60 PRINT"3. consulter le repertoire"
70 PRINT"4. sauvegarder le repertoire"
80 PRINT"5. charqer le repertoire"
90 INPUT"votre choix (puis ENTER)"|ch Passonsauxehosesserieuses... Chapitre2 Page 7
100 ON ch GaSUB 120,210,270
110 GOTO 40
120 FOR x= l TO 100
130 CLS
140 PRINT"pour arreter appuyer CENTER]"
150 INPUT "nom";NQM$(x!
160 IF NQM*(x>="" THEN 190
170 INPUT "telephane";TEL*(x)
180 NEXT
190 PRINT"entree des donnees terminee."
200 RETURN
210 FOR x=l TO 100
220 IF NOM$(x)="" THEN 230
238 PRINT NOM$ (x ) ; " "; TEL$<x>
240 NEXT
250 PRINT"*in de liste"
260 RETURN
270 INPUT "nam a trouver";RECHERCHE*
280 FOR x=l TQ 100
290 IF INSTR(NOM*<x),RECHERCHE*)=0 THEN 320
300 PRINT NOM*(x) " "}TEl»<x) ;
310 RETURN
320 NEXT
330 PRINT"ce no* n'ett pas repertorie."
340 RETURN Voila qui est mieux. Continuons ! II nous faut maintenant une instruction qui fera en sortequechaque nouvelleinformationenregistreesoit rangeea Fmterieurdela premiere case vide disponible dans le tableau. Nous allons pour cela nous servir d'une nouvelle intruction : LEN. Celie-ci permetdecalculerla longueurd'unechaine. Voicicequ'il faut indiquera 1'ordinateur Si (IF)lalongueur(LENgth)de NOMS(x) estsuperieurca0,autrementditsicettecase est deja occupee, il faut alors (THEN) passer directement a la ligne 180 (qui donnera la referencede la case suivante). Nul besoin. on le voit, d'etredoueen anglais pour parler BASIC. Toutefois,c'est avant tout une question de bon sens !"##########, "pages 110, 111");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_051, r##########"350 0PEN0UT "donnees"
360 FOR x=i TO 100
370 WRITE »9,NQn*(x),TEL«<x)
380 NEXT .. .
390 CL0SE0UT
400 PRlNT"donriees sauvees"
410 RETURN Unefoislaligne410entree par[RETURN],faites [ESC] pourinterromprelanumerota- AUTOmatique. tion N nu o m u e s ro ve d n a o n n s s l d a ' l i i n s t tc ro s d i u t i u r e e ea u p n r e es o l p ' t i i n o s n tru s c u t p i p on le O me N nt c ai h re G:O i S l U n B ou . s l f i a g u n t c 1 d 0 o 0 ne . a R j a o p u p t e e l r o u n n s la ligne1 00 pour operercette modification, par EDIT :
420 QPENIN "donnees"
430 FOR x=l TO 100
440 INPUT #9,NDM$(x),TEL*(y>
450 NEXT
460 CLQSEIN
470 PRJNT"donnees chargees"
480 RETURN Chapitre2 Page 10 Passonsauxchoses«£rieuses.."##########, "pages 112, 113");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_052, r##########"34 MODE 1 Ceci determine le mode d'afirchage: 1'ecran sera efface a chaque nouvelleexecution du programme. Ecrivezensuite
36 WINDOW #1 7,36, 18, 14 Cette instruction un peu obscure ne doit pas vous effrayer elle consiste simplement a : dessiner sur1'ecran une petitefenetre pourencadrerle menu. Lecodesitueapres1c mot WINDOW indiquea1'ordinateursurquelcanalcettefenetreseradirigee(souvenez-vous que nousen avons 8 a notredisposition, de UQ a *7). Sachant dc plusque l'ordinatcur, s'il n'a pas rccu d'indication particulicre, choisit automatiquement lecanal a©, on voit bien qu'il faut cviter de dinger notre petite fenetre sur ce canal, sous peine de voir s'y afficher toutes les sorties duprogramme. II faudra donechoisirunautrecanal entre #1 et jj7, d'ou l'indication #1 sur notre exemple. Les quatre nombres qui viennent ensuite indiquent,dcmanicreon nepeut plus simple,lesdimensionsdelafenetre :ilsdonnent la position des bords gauche, droit, superieuret inferieurde la fenetre, en se rcferant aux numcros decolonneset de ligncsde1'ecran (commepour1'instruction LOCATE). Ainsi dans noireexemple.apresavoirprecisequenousutilisionslecanal 1, nousdeclaronsque lebord degauchecommenceen colonne 7,quecelui dedroitc finitencolonne36,quele bord superieurcommence en lignc 1 ct que le bord inferieurfiniten ligne 14. Si nous voulons maintenant que le menu s'affiche dans cette fenetre, il va nous falloir
40 80 rectifier les lignes a :
40 PRINT #1,"1. ajouter un correspondant"
50 PRINT *1,"2. lister les correspondants"
60 PRINT t*l,"3. consulter le repertoire"
78 PRINT HI, "4. sauvegarder le repertoire"
80 PRINT HI, "5. charger le repertoire" Ajoutonsencore la ligne
85 LOCATE 7,25 Passoiis«ixchosesserieuses... Chapitre2 Page )1
95 CLS . Ajoutons pour finirlestrois lignessuivantes, qui mettront lordinateurenattente avant de reveniraumenu : 1B3 LOCATE 7,25
105 PRlNT"appuyez une louche pour le »«nu
107 IF INKEY$="" THEN 107 La ligne 1 03 indique l'endroitauquel I'ordinateurdevraafficherlemessagecontenuen ligne105. La ligne1 07cherchequellechainedecaracterevientd'etreentreeauclavier. Tant que celle-ci sera vide, e'est-a-dire tant que l'utilisateur n'aura pas actionne une touchequelconqueduclavier,le programmebouclerasurcetteinstruction.Cetteinstruc- tion a bien pour effet de mettre le programme en attente : celui-ci attend effectivement qu'une touche soit enfonceepour passera la lignesuivante. Le voila done lerminc, ce programme ! Termine, vraiment? ...vous pourriez encore lui demanderdecorrigeroud'effacerdesnomsetdesnumerosde telephone, de trierlaliste dansl'ordrealphabelique.oudevousla«sortir»surimprimante,oubienencore,si vous etes tres ambitieux, d'emettre des signaux permettant d'appeler automatiquement votre correspondant en entrantsimplement son nomau clavier,non sansavoir, bien entendu, demande aux PTT I'autorisation deconnecter votre ordinateur au poste telephonique ! Ces pcrfectionnements sont pourtant de l'ordre du possible. A vrai dire, on peut ame- liorcr et pcauflner ainsi scs programme a Tinfini, surtout lorsqu'on dispose d'un outil inibrmalique aussi puissant que le 6128. II Taut pourtant savoir s'arreter et nous allons laisserce«repertoiretelephonique»acepointenesperantquevousen savezdcsormais un peu plus sur l'art d'ecrire un programmeen partanl de zero. II ne vous reste qu'a le remettre un peuen ordre en tapant unedernicre instruction : RENUM ...puis ale stockersurdisquette,ouvousen debarrasser. Maisqui sait, il pourrait peut- etre vous etre utile... pournoter les numerosde telephone de vosamis Chapitre2 Page 1 Passonsauxohosess6rieuses -"##########, "pages 114, 115");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_053, r##########"18 REM repertoire telephonique
20 Dili NOM* (100)
30 DIM TEL$(100)
40 MODE 1
50 WINDOW #1,7,36,10,14
60 PRINT #1,"1. ajouter un correspondant"
70 PRINT Hi, "2. lister lee correspondants"
80 PRINT *i,"3. consulter le repertoire"
90 PRINT #i,"4. sauveqarder le repertoire"
100 PRINT ll, "5. charger le repertoire"
110 LOCATE 7,25
120 INPUT"votre choix (puis ENTER)";ch
130 CLS
140 ON ch GOSUB 190,290,350,430,500
150 LOCATE 5,25
160 PRINT"appuyer une touche pour le menu"
170 IF INKEY$="" THEN 170
180 GOTO 40
190 FOR x=l TO 100
200 CLS
210 IF LEN(NOM$(x))>0 THEN 260
220 PRINTnpour arreter appuyer CENTER]"
230 INPUT "nom";N0M*(x>
240 IF N0M*(x>="" THEN 270
250 INPUT "telephone";TEL$(x)
260 NEXT
270 PRINT"entree des donnees terminee."
280 RETURN
290 FOR x= l TO 100
300 IF NQM$(x)="" THEN 330
310 PRINT N0M«(x);" "jTEL$(x)
320 NEXT
330 PRINT"*in de liste"
340 RETURN
350 INPUT "noot a trouver"jRECHERCHE*
360 FOR k=1 TO 100
370 IF lNSTR(NOM*(x),RECHERCHE*)=0 THEN 400
380 PRINT N0M*(x);" ";TEL$(x)
390 RETURN
400 NEXT
410 PRINT"ce nom n'est pas repertorje."
420 RETURN
430 OPENOUT "donnees" Passonsauxchosess6rieuses... Chapitre2 Page 13
440 FOR x= l TO 100
450 WRITE #<?,NOM*(x),TEL*(x)
460 NEXT
470 CLQSEOUT
480 PRINT'dannees sauv»ai"
490 RETURN
500 OPENIN "donnees"
510 FOR x= i TO 100
520 INPUT #9,NOM*(x),TEl$tx>
530 NEXT
540 CLOSEIN
550 PRINT"donnBes thargees"
560 RETURN run Chapitre2 Page 14 Passonsauxchosesaerieuses."##########, "pages 116, 117");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_054, r##########"10 AFTER 250 SOSUB 60:CLS
20 PRINT"Devine urte lettre en 5 secondes"
30 a*=INKEY*:IF Hag= THEN END l
40 IF a*OCHR*(INT(RND*26 +97) ) THEN 30
50 PRINT a$;" est exacte.tu as gagne !"
55 SOUND 478: SOUND l,358sEND"##########, "page 121");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_055, r##########"60 PRINT"Trop tard.J'ai gagne !"
70 SOUND l,2000:Uag=l:RETURN run COMMANDE Appelle un sous-programme apres (= AFTER en anglais) un certain : delai. Le <delai du chronometre> indique la duree de I'attente en multiples de 0,02 seconde. Le <numero de chronometre> (qui peut etre 0, 1, 2 ou 3) precise lequel des quatrechronomelres d'attenteil faut utiliser. Chacun des4chronometrespeut etreassociea un sous-programme. Pourplusdedetails A concernant les interruptions, reportez-vous a la partie 2 du chapitre « vos heures de loisir... ». Mots clesassocies EVERY, REMAIN, RETURN : AND AND <argument> <argument> IF "alain" < "bernard" AND "chien" > "chat" THEN PRINT "vrai" ELSE PRINT "Uux" vrai IF "bernard" < "alain" AND "chat" > "chien" THEN PRINT "vrai" ELSE PRINT "faux" faux IF "alain" < "bernard" AND "chat" > "chien" THEN PRINT "vrai" ELSE PRINT "-faux" faux PRINT 1 AND 1 1 PRINT AND PRINT AND 1 OPERATEUR Executedesoperations booleennesparbitssurdesnombresentiers. Est : egal a sauflorsque lesdeux bitsd'arguments sontegaux a I. Pourtoute information complemeniaire sur la Iogique, consultezla partie 2duchapitre A « vos heuresde loisir... ». Motsclesassocies OR. NOT. XOR. : Chapitre3 Page 4 MotsclesduBASIC"##########, "page 121");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_056, r##########"10 REM 729 comb inai sons de bordure
20 SPEED INK 5,5
30 FOR a=0 TO 26
48 FOR b=8 TO 26
50 BORDER a.biCLSsLOCATE 14,13
60 PRINT"bordern$a;",";b
70 FOR t= l TO 500
80 NEXT t,b,a run COMMANDE Pour changer la couleur de la bordure d'ecran. Si deux couleurs sont : indiquees, elles atternent a la vitesse determinee par la commande SPEED INK, le cas echeant. Les valeurs vonl de a 26. Motsclesassocies: SPEED INK BREAK (Voir ON BREAK CONT, ON BREAK GOSUB, ON BREAK STOP) Ctiapitre3 Page 6 MotsclesduBASIC"##########, "page 123");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_057, r##########"18 FOR x=32 TO 255
28 PRINT x;CHR$(x),
38 NEXT run FONCTION Convertit un <nombre entier> compris entre et 255 en une chaine de caracteres equ : ivalent a 1'aide dujeu de caracteres de 1'AMSTRAD 6128, decrit dans la partie 3 du chapitre « Pourinformation... ». Lescaracteres a 31 sont descaracteres decontrole.C'estpourquoi1'exempleci-dessusaffichelesentierscomprisentre32et255. ASC Motscles associes : CINT CINT(<expression numeriquo)"##########, "page 125");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_058, r##########"10 n=1.9999
28 PRINT CINT(n) run 2 FONCTION Convertit unevaleurnumeriqueen unentierarrondicomprisentre-32768 : et 32767. Mots cles associes CREAL. FIX, INT, ROUND, UNT : Chapitre3 Page 8 Motsc!6sduBASIC"##########, "page 125");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_059, r##########"10 CL5
20 PRINT" tapez plusieurs lettret aintenant !"
30 FOR t=l TO 3000
40 NEXT
50 CLEAR INPUT run COMMANDE EffacetoutesIesdonneesentreesa partirduclavier, se trouvantdans le : tampon. Pour experimenter cetle commandc, lancez le programme ci-dessus et tapez Ies lettres lorsque vous y etes invite. Supprimez ensuite la ligne 50 du programme, relancez-le et voyez la difference. Mots cles associes : INKEY, INKEY$, JOY CLG CLG [<encre>] LOCATE 1,20 CLG 3 COMMANDE Efface Pecran graphiqueet le rameneasacouteurdefond. Si <encre> : est specifiee, le fond est de la couleur fixee en accord. Motsclesassocies CLS. GRAPHICS PAPER, INK, ORIGIN : J Motsc:esduBASIC Chapitre3 Page 9"##########, "page 126");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_060, r##########"10 PAPER #2,3
20 CLS #2 run COMMANDE: Efface la fenetre d'ecran specifiee par le <numero de canal> et lui donne sa coulcurde papier. En l'absence dc <numero decanal>, est pris pardefaut. WINDOW Mots cles associes CLG, INK, PAPER, : CONT CONT -: CONT COMMANDE CONTinue1'executiondu programmeapresunSTOP,oudeuxactiva- : tionsdela touche [ESC],si le programme n'aeteni modifieni protege. Descommandes directes peuvent etre tapeesavant reprisedu programme. Motscles associes : STOP Chapitre3 Page 10 MotsclesduBASIC"##########, "page 127");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_061, r##########"20 PRINT"coin superieur"
30 LOCATE 1, 1
40 a$=COPYCHR*(#0)
50 LOCATE 1,20
60 PRINT a* run FONCTION Copie un caractere a partirde la position du curseurdans le canal (qui : DOIT etre specific). Le programme ci-dessus copie un caractere de 1'emplacement 1,1 (anglesuperieurgauche)et le reproduiten.1,20. Si lecaracterelu n'estpasreconnu, une chaine nulle est renvoyee. LOCATE Mots cles associes : COS COS (<expre$sion numeriquo) DEG PRINT C0SU5) 0.707106781 FONCTION : Calcule le COSinusde F<expression numeriquo. DEG ct RAD peuventserviraexprimer Pargument en degresouen radians, respective- ment. Motscles associes : ATN, DEG. RAD, SIN CREAL CREAL(<expression numeriquo)"##########, "page 128");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_062, r##########"10 a=Pl
20 PRINT CINT(a)
30 PRINT CREALU) run 3 3.14157265 FONCTION Convertit P<expression numeriquo en nombre reel. : NT Motscles associes CI : MotsclesduBASIC Chapitre3 Page 11"##########, "page 128");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_063, r##########"10 t=TIHE/30e . ..
20 DEF FNchronQ*INT(TIHE/300-t)
30 EVERY 100 60SUB 50
40 GOTO 40
50 PRIN7"Le programme tourne depuis";
60 PRINT FNchrona; "secondes" i_
70 RETURN fc run COMMANDE Le BASICpermetauprogrammedeDEFinirune FoNction retournant : une valeur unique et de 1'utiliscr. DEF FN est la partie definition du mecanisme de creationd'unefonctionspecifique,travailIan I dunemanieresimilaireauxfonctionsexis- tantes du BASIC (COS, SIN, ATN. etc.). (Dans Pexcmple ci-dessus la valeur dc la fonction FNchrono est constamment mise a jour, meme si lc programmeest suspendu par [ESC] ou arrete pardouble [ESC], puis relance. Motscles associes: Aucun MotsclesduBASIC Chapitre3 Page 13"##########, "page 129");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_064, r##########"10 CURSOR 1
20 PRlNTquestion ?"}
30 a$=INKEY*:IF a*="" THEN 30
40 PRINT a$
50 CURSOR run COMMANDE : Activeou desactive l'indicateur systeme ou utilisateur. Lesparametres <indicateursysteme> et <indicateurutilisateur> doiventetresur (inactif)ou 1 (actif)- Dansla commandeINKEY$ ci-dessus,lecurseuraeterenduvisibleparfixationdel'indi- cateur systeme sur 1 (a ia ligne 10). Lecurseurs'affiche lorsque lesdeux indicateurssontsur 1. Lecurseursystemeestauto- matiquement active pour la commande INPUT et desactive pour IMKEY$. II est preferable de desactiver lecurseur pour affichage d'un texte a Pecran. Vous pouvez omettre 1'un des indicateurs mais pas les deux. Si un parametre est omis, son etat est inchange. Motsclesassocies LOCATE : DATA DATA <liste de:<constante>"##########, "page 130");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_065, r##########"10 FOR x=l TO 4
20 READ nooi*,prenoi$
30 PRINT"Mr. "jnofflt;" ";preno((i«
40 NEXT
50 DATA DUP0NT,01ivier,DURAND, Francois
60 DATA LAMIE, Frederic,MOULIN, Daniel run COMMANDE DeclaredesdonneesconstantesaTinterieurd'unprogramme. Cesdon- ; neespeuventetreaffecteesa unevariablepar la commande READ,apresquoi lepointeur passe a 1'element sui^/ant de la liste DATA. La commande RESTORE peut servir a deplacer le pointeur sur uneposition specifieede DATA. Pour de plus amples informations, consultez la partie 2 du chapitre « A vos heures de loisir... ». Motsclesassocies READ, RESTORE : Chapitre3 Page 12 MotsclesduBASIC"##########, "page 130");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_066, r##########"20 DEF FNchrono*INT(TIME/300-t)
30 EVERY 100 60SUB 50
40 GOT 40
50 PRINT"Le programme tourne depuis";
60 PRINT FNchrona; "secondes" H»
70 RETURN run COMMANDE Le BASICpermetau programmede DEFinirune FoNctionretournant une valeur uniq : ue et de I'utiliser. DEF FN est la partie definition du mecanisme de creationd'uncfonctionspecifique.travaillantd'unemanieresimilaireaux fonctionsexis- tantes du BASIC (COS, SIN. ATN. etc.). {Dans rcxemplc ci-dessus la valeur de la fonction FNchrono est constamment mise a jour, memc si le programme est suspendu par [ESC] ou arrcte par double [ESC], puis relance. Motscles associes : Aucun MotsolesduBASIC Chapitre3 Page 13"##########, "page 131");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_067, r##########"18 DEFINT n
20 no*bre=123.45o
30 PRINT nodibre run 123 COMMANDE : Defmit letypedeva % riableparDEFaut,letypeetantentier. Lorsqu'une variableintervientsansmarqueur (! $), letypepardefautestautomatiquement misen ceuvre. Cette commande defmit le type par defaut desvariables seion la premiere lettre du nom de la variable. Elle peutetre suivied'une listed'initiales. Parexemple : DEFINT a,b,c ...ou d'une fourchette d'initiales ; DEFINT a-z Motscles associes DEFREAL, DEFSTR : DEFREAL DEFREAL <listede:<Iettresconcemees DEFREAL x,a-f COMMANDE : Defmit le type de variable par D % EFaut, le typeetant reel. Lorsqu'une variable intervient sans identificateur de type (! $), le type par defaut est automati- quementmisen auvre. Letypede la variableseradeterminesuivant la premierelettredu nom de la variable. Elle peutetre suivied'une liste d'initiales DEFREAL a,b,c ...ou d'une fourchette d'initiales : DEFREAL a-z Motscles associes DEF1IMIT, DEFSTR : &* Chapitre3 Page 14 MotsclesduBASIC"##########, "page 132");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_068, r##########"10 DEFSTR n 20 nam="A*stradn
30 PRINT naa run Anstrad COMMANDE : Definitle typedevariable par D % EFaut, le typeetant unechaine. Lors- qu'une variable intervient sans identificateur (! *) le type par defaut est automati- quement misenceuvre. Letypedela variableestdetermineselonlapremierelettredeson nom. La commande peut etresuivied'unelisted'initiales : DEFSTR a,b,c ...ou d'une fourchette d'initiales: DEFSTR a-2 Motscles associes DEFINT, DEFREAL : DEG DEC DEG COMMANDE : Etablit le mode de calcul en DEGres. Par defaut, les fonctions SIN, COS, TAN et ATN considerent que I'argument qui leur est transmis est exprime en radians. La commande reste valable jusqu'a ce qu'on utilise les commandes RAD ou NEW, CLEAR, LOAD. RUN. etc. Motscles associes : ATN. COS, RAD, SIN, TAN MotsctfeduBASIC Chapitre3 Page 15"##########, "page 133");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_069, r##########"10 CLSiTAQtEVERY 10 GOSUB 90
20 xl=RND*320sx2=RND*320
32 y=200+RND1200:c*=CHR*(RND*255)
40 FOR x=320-xl TO 320tx2 STEP 4
50 Dl
60 MOVE 320,0, l:MOVE x~2,yiMOVE x,y
70 PRINT" ";c*;:FRAME"##########, "page 135");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_070, r##########"30 EltNEXTsGQTO 20
90 MOVE 320,0:DRAW x+8,y-t6, I l RETURN run C re O ac M ti M ve A e d N i D re E cte : m D e e n s t ac p t a i r ve un u e ne co I m nt m e a rr n u d p e tio E n l ( o a u ut i r n e di q r u e e ct [ e E me S n C t ] pa ) r ju u s n qu [ 'a RE ce T q U u R 'e N l ] le a soi l t a GOSUB. find'un sous-programmed'interruption L'entree dans un sous-programme d'interruption desactive automatiquement les inter- ruptions de priorite egale ou inferieure. On I'utilise quand le programme doit s'executer sans interruption, par exemple quand deux sous-programmes sont en competition pour utiliser les ressources de I'ordmateur (les ressources graphiquesdans le programmeci-dessus, parexemple). Pourde plus amples informations sur les interruptionsconsultez lapartte 2 duchapitre «A vos heuresdc loisir... ». Mots cles associes : AFTER, El, EVERY, REMAIN MotsclesduBASIC Chapitre3 Page 17"##########, "page 135");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_071, r##########"20 DIM ami$(S),tel*(5)
38 FOR n= l TO 5
40 PRINT"telephone No";n
50 INPUT "entrez le nofli";a«u$(n)
60 INPUT "entrez le nusero de tel ";tel»(n>
70 PRINT
80 NEXT"##########, "page 136");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_072, r##########"70 FOR n=l TO 5
100 PRINT n;a»ii(n),telt(n)
110 NEXT run COMMANDS DIMensionne un tableau. Cettecommande alloue l'espace requis aux tableaux et speci : fic les valeurs d'indices maximales. Le BASIC doit connaitre l'espace reserve pour un tableau, en fabsence de specification il prend 10 comme valeur par defaut. Untableauestidentifieparune <variableindiceo, asavoirunnomdevariableaccom- pagned'unensembled'indicesafinquechaque«element»dutableauaitsaproprevaleur d'indice. Uneboucle FOR NEXT peutserviracontrolerletableauen traitantchaqueelementdu tableau a tour de role. La valeurminimaled'un indiceestzero (e'est lepremierelement d'un tableau). Les tableaux peuvent etre multi-dimensionnels et chaque element est reference par sa position. Par exemple, dans un tableau dimensionne par DIM position$*20,20,20> ...un element du tableau sera reference de la facon suivante : position* (4, 5, 6) ERASE Motscles associes : Chapitre3 Page 18 Motsci*sduBASIC"##########, "page 136");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_073, r##########"10 MODE BiBORDER 0:PAPER 0: INK 0,0
20 x=RND*640:y=RND*400:z=RNDU5
30 DRAW x,y,z
40 GOTO 20 run COMMANDE : Traceunelignesurl'ecranentrela positionducurseurgraphiqueetune positionabsoluespecifieeparlescoordonneesxety. L'<encre> detracage peutetrespe- cifiee (entre et 15). Le <moded'encro facultatifdetermine1'interaction de 1'encresur I'affichagepresent a l'ecran. Lesquatre <modes d'encro sont les suivants : 0: Normal I:XOR(OUexclusif) AND 2: (ET) OR (OU) 3: Motsclesassocies : DRAWR. GRAPHICS PEN, MASK DRAWR DRAWR <decalage x>,<decalagey>[,[ <encre >][, <moded'encre>]]"##########, "page 137");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_074, r##########"10 CLS;PRINT"tu montes au premier !?"
20 MOVE 0,350:FQR 0*1 TO 8
30 DRAWR 50,0
40 DRAWR 0,-50
50 NEXTsMOVE 348 0iFlLL 3 ( .
60 GOTO 60 run COMMANDE Traceunelignesurl'ecrangraphiqueapartirducurseurgraphiquejus- : qu'alapositionspecifieeparles <decalagesxety>. L'<encre> dutracepeutetrespeci- fiee (entre et 15). MotsclesduBASIC Chapitre3 Page 19"##########, "page 137");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_075, r##########"10 ENV 1,15,-1,10,15,1,10
20 SOUND 1,200,300,15,1 run COMMANDE : Definit PENveloppe de Volume cdrrespoodant au <numero d'enve- loppe> (entre 1 et 15). utilise avec la commande SOUND. Elle peut contenir 2 ou 3 parametres Pour 3 parametres <nombre de pas>,<amplitude du pas>,<dureedu pas>. Parametre 1 <nombredepas> : Specifie Ic <nombrede pas> devolumequ'un sondoit traverserdans lasection d'enve- loppe. Parexemple,dansunesectionde 10secondes, vouspouvezfixer 10pasdevolume duneseconde. Le parametre <nombredepas> est egal a 10. Le parametre peut varierde a 127. Parametre2 <amplitudedu pas> : Peutfairevarierlevolumede a 15parrapport au pas precedent. Les 16volumesdiffe- rentssont les memesqueceux de la commande SOUND. Le parametre <amplitudedu pas> peutcependant varier de -128 a + 127. le volume revenant a apresavoir atteint 15. Parametre 3 <dureedu pas> : Specifieladurecd'un pasenunitesde0,01 seconde. Peut varierde a 255(0a la valeur 256), soit 2,56 secondes. MotsctesduBASKJ. Chapitre3 Page 23"##########, "page 141");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_076, r##########"18 OPENIN "exl.bas"
20 WHILE NOT EOF Uixt Jar.
30 LINE INPUT #9,a»
40 PRINT a«
50 WEND
60 CLOSEIN run FONCT10N : Pourtesterl.'etat d'un fichierouvert. Donne - 1 (vrai)si Tonesten fin de fichier(EnfOfFile)ou si aucun fichier n'estouvert, sinon donne (faux). Motscles associes : OPENIN, CLOSEIN ERASE ERASE <liste de:<nomde variable> DIM a(100>,b$(100> ERASE a,b$ COMMANDE Quand un tableaun'estplusnecessaire,il peutetreefface(ERASE)afin : deliberer la memoire pourd'autresutilisations. DIM Motscles associes : ERL ERL"##########, "page 143");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_077, r##########"10 ON ERROR BOTO 30
20 BOTO 1000
30 PRINT"L'erreur est en ligne";ERL
40 END run FONCTION Donnele numcrodelignedeladerniereerreurrencontree. Dans1'exemple ci-dessus TERr : eur de la Ligne 20 est indiquee par la fonction ERL. Mots cles associes : DERR, ERR, ERROR, ON ERROR GOTO, RESUME MotsclesduBASIC Chapitre3 Page 25"##########, "page 143");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_078, r##########"10 IF INK£Y*="" THEN 10 ELSE ERROR 17 run COMMANDE : Decide uneaction consecutivea uneerreurnumerotee. Voir la listedes messages d'erreur I a 32 au chapitre « Pour information... ». L'action est la meme que celle prevuc par le BASIC en cas d'erreur reelle, faisant appel a un sous-programme de traitementd'erreur, lecasecheant,et rapportant lesvaleursapproprieesd'ERR et ERL. ERROR accompagned'un <nombreentier> comprisentre33et255peut serviracreer des messages d'erreur personnalises tels que :
10 ON ERROR GOTO 100
20 INPUT "entrez.un caractere";a*
30 IF LEN(a*K>l THEN ERROR 100
40 GOTO 20
100 IF ERR=100 THEN 110 ELSE 130
110 PRINT CHR*(7)
120 PRINT"j'ai dit UN caractere !*
130 RESUME 20 run Motsclesassocies ERL, ERR, ON ERROR GOTO, RESUME ; Chapitre3 Page 26 MotsclesduBASIC"##########, "page 144");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_079, r##########"10 EVERY 58,1 GOSUB 30
20 GOTO 20
30 SOUND 1,20
40 RETURN run '',"' ., ".'' , " , . ' COMMANDE: Appelle un sous-programme du BASIC a intervalles reguliers. La <periodeduchronometro specifie I'intervallepar unitesde0,02seconde. Le <numero de chronometre> (compris entre et 3) specifie lequel des quatrechronometres utiliser. Lechronometre3correspond alaprioritesuperieureetle ala prioriteinferieure. Cha- que chronometre peut etre associe a un sous-programme. Pourde plusamples informationssurlesinterruptions.consultc* la partie 2du chapitre A « vos heures de loisir...». Motsclesassocies AFTER, REMAIN : EXP EXP(<expression numeriquo PRINT EXP(6.876> 968.743625 FONCTION : Calcule «e» a la puissance donnee par l'<expression numeriquo ou «e»est egal a2,7182818 environ, le nombredont le logarithme naturel est !. Motsclesassocies LOG : FILL FILL <encrc>"##########, "page 145");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_080, r##########"10 MODE
20 FOR n=l TO 500
30 PRINT "Q"i
40 NEXT
50 coulstvlo=2+RNDtl3
60 FILL coulstylo
70 GOTO 50 run COMMANDE : Remplit unezonearbitrairede I'ecran graphique. Lesbordsde la zone sontdelimites par les lignes dessinees avec 1'encre du stylo en cours ou avec l'encre du fond(compriseentre et 15). Leremplissagepart delapositionducurseurgraphique.Si celui-ci se trouve sur un bord. rien n'est rempli. Mots cles associes GRAPHICS PEN : Motsc)4sduBASIC Chapitre3 Page 2?"##########, "page 145");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_081, r##########"10 FOR 0=2 TO 8 STEP 2
20 PRINT n;
30 NEXT n
48 PRINT", qui va augaenter !" run COMMANDE : Executela partiedu programmesetrouvantentreiesmotscles FOR et NEXT autant de fois que Ton peut ajouter 1*<increment> a la <variable simple> en partantdu <debut>jusqu'ala <fln> . Si r<increment> estomis,il prend implicitement la valeur 1 La valeur de r<mcrement> peut etre negative. Dans ce cas la valeur du parametre <debut> doit etre superieure a celle du parametre <fin>, faute de quoi la variable ne peut etre incrementee. Les boucles FOR NEXT peuventetre imbriquees. L'affectation du nom de variable a la commande NEXT est facultative car le BASIC determine automatiquement la commande NEXT a Iaquelleest associee unecommande FOR. Motsclesassocies NEXT, STEP,TO : Chapitre3 Page 28 Motsc!6sduBASIC"##########, "page 146");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_082, r##########"20 PRINT"5ans FRAME"
30 TAG
40 MOVE 0,208
50 FOR x=0 TO 500 STEP 4
60 IF f=l THEN FRAME
70 MOVE x,200
80 PRINT" ";CHR5(M3)5
90 NEXT
100 IF i-\ THEN RUN \ :| K
110 CLS ' -
120 TAGOFF
130 FRINT"avec FRAME"
140 *»1
150 GOTO 30 run COMMANDE Synchronisel'ecrituredesgraphiquesaveclestramesvideo. II enresulte un mouvement p : lus harmonieuxdescaracteres oudes graphiquesa 1'ecran sans distor- sionet sans scintillement. Mots cles associes : TAG, TAGOFF FRE FRE ^expressionnumeriquo) FRE (<:chaincalphanumerique>) PRINT FRE(0) PRINT FREC") FONCTTON Indiquel'espacedisponibleenmemoire.Laforme FRE(" ")forceI'ordi- : naieur a mettre de lordre avant dc donner la valeurde l'espace disponible. REMARQUE Le BASIC n'cxploiteque iebloc de la memoirc. : HIMEM, MEMORY Motsclesassocies : Motsclesdu BASIC Chap.tre3 Page 2"##########, "page 147");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_083, r##########"10 NODE
20 MASK 15
33 GRAPHICS PAPER 3
40 DRAW 640,0 run COMMANDE Determine l'<encre> du fond graphique. Lorsdu tracagede Iignes,le fond n'est pasvi : sible. Dans I'exempleci-dessus, la commande MASK permet letracage d'unc ligne en tirets et la visualisation du fond graphique. L'encredufond(entre et 15)sertalazone«paper»surlaquelles'affichent lescaracte- res, Iorsquc TAG fonctionne,etfait office de valeurpar defaut lors de l'effacement des fenetres graphiques a l'aidede CLG. Motsclesassocies CLG, GRAPHICS PEN, INK. MASK, TAG, TAGOFF : Chapitre3 Page 30 M°'sc^sduBASIC"##########, "page 148");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_084, r##########"10 MODE a
20 GRAPHICS PEN 15
30 MOVE 200,0
40 DRAW 200,400
50 MOVE 63?,
60 FILL 15 run COMMANDE : FixeF<encre> (entre et 15)pourledessitidesligneset lepositionne- mentdes points. Le <modedu fond> peut egalementetre fixe ; 0: Fond opaque 1: Fond transparent TAG (Lefond transparenta uneinfluencesurlefondgraphiquedescaracteresecritsavec etles espacesen lignes pointillees). Vous pouvez omettre 1'un des parametres mats pas les deux. Si Fun des parametresest omis, la valeurspecifiee reste inchangee. Motscles associes GRAPHICS PAPER, INK, MASK, TAG,TAGOFF : HEX$ HEX$ ( <nombreentiersans signe> [, <largeur de zone>] ) PRINT HEX$<255,4) 08FF FONCTION Change un nombre entier en un nombre HEXadecimal equivalent en : accordaveclenombredechiffreshexadecimaux indiqueparla <largeurdezone> (entre et 16). Si ce nombre est trop grand, Texpression finale est completee par des zeros a gauchedu nombre.S'ilesttroppetit,ellenesera PAStronquccmaisle nombredechiffres produit sera egal au nombre requis. Le <nombre entier sans signe> a convertirsous forme hexadeeimale doit produire une valeurcompriseentre -32768 et 65535. Mots des associes BIN$, DEC$, STR$, UNT : M3HT Motsclesdu BAKC Chapitre3 Page 3"##########, "page 149");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_085, r##########"10 MODE 1
20 x=CINT(RND*100>.
30 PRINT"devinez un chHfro (0 a 100)"
40 INPUT n
50 IF n<K THEN PRINT nj"est trop petit..."
60 IF n>x THEN PRINT n;"est trop grand..."
70 IF n= x THEN 88 ELSE c*c+lsG0TQ 40
80 PR2NT"Bien vu! ";"Trouve en";c+l; "fois !" run COMMANDE : Determine si l'<expression logiquo est vraie pour executer, le cas echeant, la premiere <option>. Si F<expression logiquo estfausse, l'<option> placee apres ELSEs'execute. En absencededeuxieme <option>,le BASICpassea la Iignesui- vante. LescommandesIF THEN peuvent etreimbriqueesmaisseterminent a la finde la ligne. On ne peut done PAS avoir de declarations independantes de IF THEN sur la meme Iigne. Lorsque le resultat de ]"expression logiquo necessile un saut de ligne, la com- mande peut, parexemple, se formuler : IF a=l THEN 100 ...ou : IF a=l GOTO 1B0 ...ou IF a=l THEN GOTO 100 Motsclesassocies ELSE, GOTO, THEN : Chapilre3 Page 32 Motscl6sduBASIC"##########, "page 150");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_086, r##########"10 MODE liPAPER BtPEN I
20 FOR p=0 TO 1
38 FAR i=0 TO 26
40 INK. p i
50 LOCATE f 16, 12l PRINT" INK" j p; H »"S
60 FOR t=l TO 400:NEXT t,i,p
70 INK B, 1 : INK i,24:CLS run COMMANDE Affecte la ou Ies couleurs a une encre donnee. Le parametre <encre> : fournitlareferencedeI'encre(par unentiercomprisentre et 15).al'intentiondescom- mandes PEN ou PAPER correspondantes. Le premier parametre <numero decouleur> (entier) donne une valeurde couleur comprise entre et 26. Si le second parametre de couleurfacultatifestspecifie,I'encrepassed'unecouleura1'autreselonunevitessedefinie par la commande SPEED INK. Motsclcs associes GRAPHICS PAPER, GRAPHICS PEN. PAPER, PEN, : SPEED INK INKEY INKEY (<nombre entier>)"##########, "page 151");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_087, r##########"10 IP INKEY <55><>32 THEN 10 *<
20 PRlNT"vous venez d'appuyer [SHIFT] et V"
30 CLEAR INPUT run FONCTION Interrogeleclavier pourindtquer les touchespressees. Leclavierestana- : lysetouslescinquantiemesdeseconde. Cettefonctionsertadetecterlapositionhauteou basse d'une louche par detection de la valeur -1 (independante de t'etat des touches [SHIFT] et [CONTROL]). Dans 1'exempleci-dessus. lcsystemedetectel'actionnement simultanede [SHIFT] et V (numerodetouche55)avantd'arreterle programme.Lesnumerosdetouchesontdortnes dans le diagramme situe en haut a droite du boitier de 1'ordinateur (voir egalement le chapitre « Pour information... ». \ iMvit ,: \jK. MotsclesduBASIC Chapitre3 Page 33"##########, "page 151");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_088, r##########"10 CLS
20 PRINT"choisissez QUI ou NON (0/N>
30 a*=INKEY*
40 IF a$="" THEN 30
50 IF a*="o"QR a*="0" THEN 80
60 IF a$="n"0R a$="N" THEN 90
70 GOTO 30
80 PRINT"Voub avez chaisi our .END
90 PRINT"Vous avez choisi NON' yi»jfvfj run FONCTION : Interroge le clavier pour introduire dans le programme toute chafne de caracteres entree. Si aucune touche de clavier n'est actionnee, INKEY$ renvoie une chamevide. DansTexempleci-dessusles lignes 40 et 70commandent au programmede revenir a la ligne 30 apres interrogation du clavier parla fonction INKEY$. Motsclesassocies : CLEAR INPUT, INKEY INP INP (<numero du port>) PRINT INPUFF77I 255 FONCTION : Lit la valeur contenue dans un port d'entrees-sorties dont Padresse est transmise par 1'argument de cetle fonction. Motscles associes OUT. WAIT : Chapitre3 Page 34 Motsctesdu BASIC"##########, "page 152");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_089, r##########"10 MODE
20 INPUT 1 "donnez-moi deux nontbres a multiplier (separes p ar une virgule} ";a,b
30 PRINT a;"foi5";b;'*'font"5a*b
40 GOTO 20 run COMMANDE: Recoit les donnees en provenance du canal precise (canal tl en l'absence de specification). Un point-virguleapres INPUTsupprimelepassagea la ligneapresexecutiondelacom- mande. Le <separateur> peutetreun point-virguleouunevirgule. Un point-virguleplaceapres la chainefait apparaltre un pointd'interrogation ; unevirgule lesupprime. Si uneentree erronecest effectuee, un Opour un parexemple, BASIC repond : ?Redo from start ...ou tout autre messaged'erreurprogramme par vos soins. Toute reponseau clavierdoit se terminer par (RETURN]. Motsclesassocies : LINE INPUT MotsclesduBASIC Chapitre3 Page 35"##########, "page 153");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_090, r##########"10 CLSiFOR n=l TO 26
20 alphabet$=alphabet$+CHRt<n+64>
30 NEXT
40 INPUT "entrez une lettre";a«
50 bi=UPPER$<a$l 6U PRINT b*i" est en position";
70 PRINT INSTRUlphabet$,b$)5
80 PRINT"dans r alphabet, "s PRINT
90 GOTO 40 run FONCTION Cherche dans la <chamecontenanto Poccurence de la <chaineconte- : nue> et indique la position de la premiere occurence de la chaine recherchee. En son absence, la fonction indique la valeur 0. La position du debut de la recherche est facultative, elle est specifiee par le parametre <position de depart> sous ia formed'unentiercomprisentre 1 et 255. Encasde recherche infructueuse, la fonction retoumela valeur0. Motsclesassocies Aucun : INT INT(<expression numeriquo) PRINT INK-1.995) -2 FONCTION Arronditaupremierentierinferieur,enlevant lapartiefractionnaire. Iden- : tique a FIX pour les nombres positifs, il donne 1 de moins que FIX pour les nombres negatifsqui ne sont pasdes entiers. ROUND Motsclesassocies CINT, FIX, : !-•' :' «•(«, ^ Chapitre3 Page 36 MotsclesduBASIC"##########, "page 154");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_091, r##########"18 PRINT"Pour arreter 1b programme - ";
20 PRINT" actionnez la manette de jeu"
32 IF JOY<8><>0 THEN END
40 BOTO 10 run FONCTION : Lafonction JOY lit I'etat de la manettedejeu specifiee par le <nombre entier> (0 ou1). Leresultat n'a designification qu'en binaire. Bit Decimal 0: Haut 1 Bas 2 I: 2: Gauche 4 3: Droite 8 4: Tir2 16 5: Tir 1 32 Ainsi, lorsque vous appuyez sur le bouton de «tir» (Tir 2) de la premiere manette en deplacant celle-ci versla gauche,la fonction JOY (0)envoie unevaleurdecimaleegalea 20, soil 16(Tir 2) + 4 (Gauche). Pourde plus amples informations, consultezlechapitre« Pourinformation...». Motsclesassocies CLEAR INPUT, INKEY : KEY KEY <numero logiquede touche>,<chaine alphanumeriquo KEY 11, "border 13:paper 0:pen 1 ink 0,13:ink l,0:*odo 2s list "+CHR*<13i Appuyez sur la touche [ENTER). COMMANDE : Associeuncchainea latouche(KEY)correspondantau <numero logi- que de touche> specifiee. II existe 32 numeroslogiquesdetouche(de a 31). occupant les touches 128 a 159. Lcs touches 128 (0 sur le claviernumerique) a 140 ([CONTROL] [ENTER] surlcclavier numerique) som associeespardefaut auxchiffres O a9, aupoint decimal, a [RETURN] et a RUN" [RETURN] - (pourla cassette), mais peuvent etreasso- cieesad'autreschainessinecessaire. Lesnumeroslogiquesdetouche 13a3 1 (touches 141 a 159)sont affcciesadeschainesvidespardefaut mais peuventetrectendusetassociesa des touches, a ['aide de la commande KEY DEF. Motscl6sduBASIC Chapitre3 Page 3?"##########, "page 155");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_092, r##########"10 CLS
20 a$="AMSTRAD"
30 FOR n=l TO 7
40 PRINT LEFT*U*,n>
50 NEXT run FOMCTION• Extrait un certain nombre de caracteres (entre et 255) a gauche d'une <chaine alphanumerique> . Si iachaineest plus courte quela longueur requise, elle est utilisee entierement. Motscles associes : MID$, R1GHT$ LEN LEN (<chainealphanumerique>)"##########, "page 157");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_093, r##########"10 LINE INPUT "entrez une phrase";at
20 PR1NT"U phrase est longue de"}
30 PRINT lENU*);"caracteres." run FONCTION Donnelenombredecaracteresdela <chalnealphanumerique> , lesespa- : cescompris. Motsclesassocies : Aucun LET LET <vanable> = <expression LET x=!00 COMMANDE Un restcdes BASIC historiques, pour Icsquels on devait annonccr ses variables. Seule : ment utile pour la compatibility avec des programmes anteneurs. En AMSTRAD BASIC, il suffil d'ecrire x=100 Motsclesassocies : Aucun Motscl,,esd,u „ BA „ S . I . C ChapFitreS Page 3£"##########, "page 157");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_094, r##########"10 LINE INPUT "tapez une ligne de texte ponctuet. "?»«
28 CLS
30 PRINT"la variable a* est bien egale a :-"
48 PRINT a$ run COMMANDE Recoit une ligne entiere en provenance du canal indique (# en : ['absencedespecification). Lepremier [;] point-virgulefacultatifsupprime leretourcha- riot/sautde ligne qui intervient normalement aprcsexecution de ia commande. Le <separateur> peutetreunpoint-virguleouunevirgule. Lepoint-virguleentraineI'af- fichaged'un point d'interrogation ; la virgule le supprime. L'entreede LINE INPUT au clavierse termineparractivation de la touche [RETURN]. LI NE I NPUT enprovenanceducanal§9deIadisquette(oudelacassette)seterminepar un retourchariotou parl'affectationdeplusde255caracteresala <variableenchaine> Motscles associes INPUT : LIST L I ST [ <ensemblede lignes> ][, <numerodecanal> ] LIST 100-1000, #1 COMMANDE : Ltstele programmesurle canaldesire, n est 1'ecran, ft 8 est Pimpri- mante. Le LISTage peut etre provisoirementinterrompu sivous appuyez unefoissurla touche [ESC], pouretreensuite reprisa l'aidedelabarred'espacement. Sivousappuyez deux fois sur [ESC], vousarretez le listageet revenez au modedirect. Vouspouvezomettre le premierou ledernier numerode lignedu parametre <ensemble de lignes> pour lister le programme depuis le debut, ou jusqu'a la fin. Exemples LIST -200 Chapitre3 Page 40 MotsclesduBASIC"##########, "page 158");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_095, r##########"10 MODE 1
20 FOR n=i TO 20
30 LOCATE n,n
40 PRINT CHR«(143);"pofiition";
50 PRINT n;\"jn :-.-
60 NEXT run COMMANDE : Deplacele curseurde texte vers une nouvelle position relative aucoin superieur gauche de la fenetre (WINDOW). // represente lecanal pardefaut. WINDOW Mots cles associes ; Motscl6a duBASIC Chapitre3 Page 41"##########, "page 159");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_096, r##########"10 a*="REGARDEZ COMMENT LES LETTRES SUNT CHANGEES "
20 PRINT LQWER*U* +"EN TYPE MINUSCULE") run FONCTION Changetouteslesmajusculesd'unechainealphanumeriqueenminuscules. : Utilequand on attend desreponsescomposeesd'un melangedemajusculeset deminus- cules. Motsclesassocies UPPERS : iftHV Chapitre3 Page 42 MotsctesduBASIC"##########, "page 160");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_097, r##########"20 MOVE -100*RND,400tRND
30 WHILE XPQS<640
40 FDR x=l TO 8
50 MASK. 2t(8-x)
60 DRAWR 32,0,x,l:MOVER -32,0
70 NEXT
80 MOVER 34,0
90 WEND: GOTO 20 run COMMANDE : Definitlemodelea utiliserpourletracedeslignes. La valeurbinairedu <nombreentier> compriseentre et 255active(1)oudesactive(0)lesbitsdanschaque groupeadjacent de 8 pixels. Leparametre <tracedu premierpoint> determinesilepremierpointdela lignedoitetre trace (I)ou non (0). Vous pouvez omettre Tun des parametres mais pas les deux. Si vousen omettez un, sa specification demeure inchangee. DRAW. DRAWR. GRAPHICS PAPER, GRAPHICS PEN Motsclesassocies: MAX MAX ( <listede:<expression numeriquo)"##########, "page 161");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_098, r##########"10 n=66
20 PRINT r1AX(l,n,3,6,4,3) 'hi-: run 66 FONCTION : Donne la valeur la plusgrande(MAXimale)de la liste. M Motsclesassocies : I MotsclesduBASIC Chapitre3 Page 43"##########, "page 161");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_099, r##########"10 MODE 1: ZONE 3
20 a*="ENCYClOPEDIE"
30 PRINT"Regardez comment tpeler ";a$ Mhfi
40 PRINT .
50 FOR n= l 1,0 LENU*}
60 PRINT MID«(a* n l), f l
70 FOR t*l TO 700SNEXT t,n
80 PRINT: PRINT
90 INPUT "entrez un nouveau mot":a$
100 GOTO 50 run Chapitre3 Page 44 Motsc!6sduBASIC"##########, "page 162");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_100, r##########"18 a*="bonjour"
28 rHD*(a*,3,2)»"XX"
30 PRINT a* run boXXour COMMANDE/ Inseredans la chaine specifiee une <nouvellechainealphanumeriquo d'un nombre de caracteres donne, a la <position d'insertion:*. Lorsque vous utilisez MIDS en tant que COMMANDE, vous devez faire appel a une <variablechaino, tel queA$. et non PAS a uneconstante comme« bonjour». Motscles associes : LEFT$, RIGHTS MIN M I N ( < listede:<expression numerique> PRINT r1IN<3,6,2.999 8,9,) I 2.999 FONCTION Donne la valeur la plus petite (MINimale) de la <liste de:<expressions : numeriques>. MAX Motsclesassocies : MotsclesduBASIC Chapitre3 Page 45"##########, "page 163");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_101, r##########"10 D't+liIF a>2 THEN «*0
20 MDDE in
30 PRINT"Ceci est le «ode";m
40 PRINF'Pressez une touche"
50 IF INKEY*s»« THEN 6OT0 50 ELSE 10 run COMMANDE : Modifielemoded'ecran(0, 1 ou2)etretablitsurPecranl'encre0, meme si l'encreactuellement utiliseepar le papierest differente. Toutes les fenetreset curseurs sont reinitialises. Mots cles associes : WINDOW, ORIGIN . . MOVE MOVE <coordonnee x>,<coordonnee y>[,[<encre>][,<mode d'encre>]]"##########, "page 164");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_102, r##########"20 x=RND*800-100:y=RND*430
30 MOVE x,y
40 PRI N T "je suis ici"s
50 SOTO 28 -.-:,:> run COMMANDE : Positionne lecurseurgraphiqueau pointabsolu specifie. Le parametre facultatif <encre> (compris entre et 15) permet de changer la couleur du stylo graphique. % Chapitre3 Page 46 MotsclesduBASIC"##########, "page 164");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_103, r##########"20 PRINT"la vie a set";
30 FOR n=l TQ 10
40 MOVER -45,16
50 PRINTuhauts"i:NEXT!PRINTH tt"j '
60 FOR n=l TO 10
70 MOVER -64,-16
80 PRINT"bas";:NEXT run COMMANDE : Positionne le curseur graphique en coordonnees relatives(par rapport a lapositionactuelle). Le parametrefacultatif<encre> (comprisentre et 15)permetde changer la couleur du stylo graphique. Leparametrefacultatif<moded'encre> determinel'interactiondeI'encresuri'affichage en place a Pecran. II existe4 <modes d'encro : 0: Normal 1: XOR (OU exclusif) AND 2: (ET) 3: OR (OU) Motscies associes MOVE, ORIGIN, XPOS, YPOS : tu> Motsc!esduBASIC Chapitre3 Page 47"##########, "page 165");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_104, r##########"10 FOR a=l TO 3
28 FOR b= TO 26
30 MODE 1
40 PEN a: BORDER b
50 PRINT"PEN"}ai"BORD£R";b
60 FOR c= i TO 500 •-:;J?q { : ;
70 NEXT c,b,a run COMMANDE: Marque la fin d'une boucle commencee avec FOR. La commande NEXT peut etre anonyme ou peut se rapporter au FOR concerne. Dans t'exemple ci- dessus, la <listede;variable> doitapparaitreensensinversedela specificationdescom- mandes FOR, afin d'eviter lechevauchement des boucles imbriquees. Motscles associes FOR, STEP, TO : NOT NOT <argument> IF NOT "alain" < "bernard" THEN PRINT "vrai" ELSE PRINT Maux" faux IF NOT "chat" > "chien" THEN PRINT "vrai" ELSE PRINT "faux" vrai PRINT NOT -1 PRINT NOT -1 OPERATEUR : Executedesoperationsparbitsurdesentiers. Inversechaquebitde['ar- gument. Pour dc plus amples informations, consultez la partie 2 du chapitre « A vos heures de loisir... ». Motscles associes AND. OR, XOR : Chapitre 3 Page 48 Mot*cWsduBASIC"##########, "page 166");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_105, r##########"10 ON BREAK CONT
20 PRINT"Le programme CONTinuera si vou* tssayez o« fai. rt IB U reak* avec touche CESC]":PRINT .30 FOR t= l TO 1000:NEXT:GOTO 20 run COMMANDE AnnuleTactiondela touche [ESC],empechant rarretdu programme. : Cettecommandeest a utiliseravecprecautioncar)e programmenepeutalorsetreinter- rompu que parreinitialisationcompletede l'ordinateur(vousdevezdonesauvegarder le programme avant de le lancer). Vouspouvezdesactiver ON BREAK CONT par ON BREAK STOP a 1'interieurd'un programme. ON BREAK GOSUB, ON BREAK STOP Motscles associes : ON BREAK GOSUB ON BREAK GOSUB <numero de ligno"##########, "page 167");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_106, r##########"10 ON BREAK GOSUB 40
20 PRINT"le programme tourne"
30 GOTO 20
40 CLS;PRINT"Appuyer 2 fais C ESC 3 , "
50 PRINT"appeUe le sous-programme"
60 FOR t=l TO 2000:NEXT
70 RETURN run COMMANDE: Demande au BASIC de passer an sous-programme specific par le <numero de ligne> lorsque vous appuyezdeux foissur [ESC]. Motscles associes : ON BREAK CONT. ON BREAK STOP, RETURN MotsclesduBASIC Chapitre3 Page 49"##########, "page 167");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_107, r##########"10 ON BREAK 60SUB 40 *
20 PRINT"le programme tourne*
30 GOTO 20
40 CLS:PRINT"Appuyer 2 fois [ESC],";
50 PRINT"appel le le sous-programme" -.-
60 FOR t=l TO 2000:NEXT
65 ON BREAK STOP
70 RETURN run COMMANDE: Desactive les commandes ON BREAK CONT et ON BREAK GOSUB pour permettrel'arretdu programmesuractivationdela touche [ESC]. Dans l'exemplcci-dessus,lacommandeON BREAK GOSUB nefonctionncqu'uneseulefois carelleestdesactivee a la ligne65dans 1c sous-programme ON BREAK. Motsclesassocies : ON BREAK CONT, ON BREAK GOSUB ON ERROR GOTO ON ERROR GOTO <numerodeligne>"##########, "page 168");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_108, r##########"10 ON ERROR GOTO 60
20 CLS:PRINT"Si une erreur est trouvee, ";
30 PRINT"alors LISTer le programme"
40 FOR t= l TO 4000J NEXT
50 GOTO L00
60 PRINT"Erreur detectee a la ligne";
70 PRINT ERL: PRINTiLIST run COMMANDE : Passe a la ligne specifiee aussitot qu'uneerreurest detectee. La commande ON ERROR GOTO desactive le deroutement du programme sur erreurel retablit le traitement normal des erreurs par le BASIC. Voiregalement la commande RESUME Motscles associes DERR, ERL ERR, ERROR, RESUME : Chapitre3 Page 50 MotsclesduB«3C"##########, "page 168");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_109, r##########"10 PAPER 8:PEN t: INK 0, 1
20 CLS:PRINT" MENU " P U RINT
30 PRINT" 1 - Changer cadre" : PRINT
40 PRI NT "2 - Changer le styl o": PRINT
50 PRINT"3 - Changer de mode":PRINT
60 INPUT "Votre choix"5X
70 ON x BOSUB 90,110,130
80 6DT0 20
90 b=b-l:IF b<0 THEN b=26
100 BORDER b;RETURN
110 p=p-l:IP p<2 THEN p=26
120 INK l,piRETURN
130 **«ri:IF m<0 THEN «*2
140 MODE nt RETURN ..:.=;-:- »•....!>!>> run COMMANDE Selectionne une ligne de sous-programme en fonction de la valeur du : <selecteur> (nombreentiercomprisentre et255).L'ordredesvaleursdes <selecteurs determine le numero de ligne a extraire de la <liste de:<numeros de lignes>. Dans I'exempleci-dessus 1 provoquele passage a la ligne90,"##########, "page 169");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_110, r##########"10 CLSiPRINT" MENU ":PRINT
20 PRINT"! - LISTe le programme":PRINT
30 PRINT"2 - EDI Te pour corriger ": PRINT
40 PR1NT"3 - Fait 2e CATalogue" :PRINT
50 INPUT "Votre choix"in
60 ON n GOTO 80,90, 100
70 GOTO 10
80 LIST
90 AUTO
100 CAT run COMMANDE Selectionneunelignea laquellele programmedoitsauterenfonctionde : la valeurdu <selecteur> (nombreentiercomprisentre et255). L'ordredes valeursdu <selecteur> determinelenumerodeligneaextrairedela <l«stede:<numerosdelignes> Dans Texempleci-dessus"##########, "page 170");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_111, r##########"10 ENV 1,15,-1,1
20 ON SQdl GOSUB 60
30 MODE 0: ORIGIN 0,0,200,440,100,300
40 FOR x = l TO 13:FRAMEiM0VE 330, 200, x
50 FILL x:NEXT:60T0 40
60 READ s: IF s=0 THEN REST0RE:G0T0 60
70 SOUND 1,5,25, 15,
80 ON SQU) GOSUB 60: RETURN
90 DATA 50,60,90,100,35,200,24,500,0 run Chapiire3 Page 52 Motscl6sduBASIC"##########, "page 170");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_112, r##########"10 REM Ouvre ft Rscoit le fichier en provenance de la disquette
20 OPENIN "NDMFICH"; INPUT #9,a,a$
30 CLOSE IN; PR INT"les 2 valeurs sont:"
40 PRINT:PRINT a, a* run COMMANDE Ouvreun fichierexistant surladisquetteafin d'yliredesdonneesdesti- : neesauprogrammeen memoire. Lefichieraouvrirdoitetreun fichierASCII.Cetexem- ple ne fonctionneque si vous avezcreele fichier selon l'exemplede la commande OPE- NOUT. Motscles associes : CLOSEIN. EOF OPENOUT OPENOUT <nomfich>"##########, "page 171");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_113, r##########"10 REM Ouvre et Sort le fichier sur la disquette
20 INPUT "Dornez moi un noabre";a
30 INPUT "Donnez raoi un MotB}a*
40 OPENOUT "NOMFICH"
50 WRITE »9,a,a«
60 CLOSEDUT PRINT"les donnees sont sauvees sur disquette" : run COMMANDE Ouvre sur la disquette un fichierde sortie utilisable par le programme : en memoire. CLOSEOUT Mots cles associes : Motscl6sduBASIC Chapitre3 Page 53"##########, "page 171");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_114, r##########"20 ORIGIN 0,0,108,540,300, 100
30 GRAPHICS PAPER 3:CLS
40 FOR x=550 TO -340 STEP -10
50 MOVE x,206
60 PRINT"Voici une fenetre graphique ";
70 FRAME:NEXT:G0T0 40 run COMMANDE : Etablit lepoint d'origineducurseurgraphiqueauxcoordonnees <x>, <y> specifiees. Vous pouvezegalement fixerlesdimensionsdela fenetre graphique par specificationdesquatredernicrsparametresfacultatifs.Si lescoordonneesspecifieespour lafenetregraphiquesetrouventendehorsdePecran, lesbordsdeTecransontalorsconsi- derescommc les limitesde la fenetre. Motsclesassocies : CLG Chapitre3 Page 54 MotsclesduBASJC"##########, "page 172");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_115, r##########"18 MODE 0:PEN 19: INK 0,13
20 FOR p=l TO 15
30 PAPER p:CLS
40 LOCATE 7, 1 2: PR I NT PAPER" ; p
50 FOR t=l TO 5001NEXT t,p run COMMANDE : Etablit la couleur du fond pour les caracteres. Lors de raffichage des caracteres sur l'ecran, sa matrice (la grille) est remplie par P<encre> correspondant au papier {PAPER INK)avantqu'il nesoit lui-memeaffiche(saufencasdemode transpa- rent). Siie <numerodecanal> n'est passpecifie, lecanal fl est prispardefaut. Le nombredecouleursdisponiblesdepend du modechoisi. Motsclesassocies: INK, GRAPHICS PAPER. PEN MotsclesduBASIC Chapitre3 Page 55"##########, "page 173");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_116, r##########"10 MODE Is ZONE 7
20 WINDOW 1 , 48, 1 , 2: WINDOW #1,1,48,3,25
30 PRINT"Adresse memoire"
40 LOCATE 20, i PRINT"Contenu memoire"
50 FOR n=0 TO 65535
60 p=PEEK(n)
70 PRINT #l n "(l<"}HEX*(n>5")";
80 PRINT #l, l TA>B(20)jp,"<i<";HEX«(p>;")"
99 NEXT run e F n O tr N e C l T es I p O a N ren : t L h i e t se l s e . co C n et t t e e nu ad d r e es l s a e c d a o s i e t m e e t m r o e i c r o e mp Z r 8 i 0 se do e n nt t re l' & < 0 a 0 d O re 0 ss et e> &F es F t F i F ndi ( q 0 ue e e t 65535). PEEK n'opere que sur la memoire vive (RAM), jamais sur la memoire morte (ROM), et fournit des valeurscomprisesentre &00 et &FF (0ct 255). POKE Motscles associes : PEN PEN <numerodecanal>,][<cncre>][,<modedu fond>] if ["##########, "page 174");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_117, r##########"10 MODE 0:PAPER 0: INK 8,13
20 FOR p=l TO 15
30 PEN p:PR!NT SPACE*(47>;"PEN";p
48 FOR t=l TO 500:NEXT t,p:G0TO 20 run COMMANDE Selectionne1'<encre> autiliser(de a 15)pourecriresurlecanal indi- : que(pardefaut : #0). Leparametre <modedu fond> peutetresoit transparent(I), soit opaque (0). Au moins undesdeuxderniers parametresdoit figurer. Si l'und'euxestomis,sa valeur antericure reste inchangee. PAPER Motsclesassocies : 4 Chapitre3 Page 56 MotsclesduBASIC"##########, "page 174");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_118, r##########"10 MODE li BORDER 8:PAPER 0:PEN 1
20 INK 0,0: INK 1,26: INK 2, 13,26: DE8
30 FOR x= l TO 360: ORIQIN 320,200
40 DRAW 50*COS(x),50tSIN(x>, I
50 PLOT 100*CaS(x),25*SIN(x):NEXT
60 ORIGIN 0,0:t=TIME+700:WHILE TIME<t
70 PLOT RND*640,RND*400:WEND
80 PLOT RNDI640,RND<400,2
90 GOTO 90 run COMMANDE ; Affiche, enmodegraphique, le point decoordonneesxety. On definit l*<encre> dece point sur une echelle de a 15. Leparametre facullatif <moded'encro determinele moded'interaction entrelacouleur utiliseeet celle de 1'ecran. Voici lesquatre modespossibles Normal :"##########, "page 175");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_119, r##########"40 IF 2NKEY(0)=0 THEN PLOTR ,1
50 IF INKEY ( 1 )=« THEN PLOTR i,0 <Sfl IF INKEY(21*0 THEN PLOTR 0,-1
78 IF INKEY(8)*0 THEN PLOTR -t,a
88 IF INKEY(9)=fl THEN 30: REP t CCOPY]-CLS
90 GOTO 40 run COMMANDE Enmodegraphique,afilcheal'ecranlepointdecoordonneesxetyrela- tives a la positi : on du curseur a ce moment. On definit l'<encre> de ce point sur une echellede a 15. Le parametre facultatif <mode d'encro definit le mode d'interaction entre lacouleur utiliseeet cellede l'ecran. Voici les quatremodespossibles : Normal : XOR (OU exclusif)"##########, "page 176");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_120, r##########"10 FOR =49152 TO 65535 ii
20 POKE ft, IBB
38 NEXT run COMMANDE Inscrit la valeur correspondant au <nombre entier> (compris entre et 255) directem : ent dans la case de la memoire vive (RAM) du Z80dont l'<adresse> est indiquee. Commande a utiliseravecprecaution ! Mots clesassocies : PEEK Chapltre„3 „ Page 5n8o MotsctesduBASIC"##########, "page 176");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_121, r##########"10 MODE Is BORDER BiLOCATE 8,2
22 PRINT"utilisez les Heches droi te/gauche "
30 WINDOW 1,40, 12, 12: CURSOR 1,1
40 FOR n=l TO 19: PRINT CHR* (9) i NEXT
50 IF INKEYU)<>-1 THEN PRINT CHR$<9);
60 IF INKEY(8K>-i THEN PRINT CHR$(8);
70 LOCATE #1,2,24
80 PRINT #l,"curseur texte,";
90 PRINT #1, "position horizontale="j
100 PRINT #1,POS(#0)!BOTO 50 run FONCTION : Calcule la POSition du curseur de textesur1'axe horizontal, apartirdu bord gauchedelafenetre. Le <numerodecanal> doitobligatoirementetreprecise ;il ne prend pas la valeur ?Q pardefaut. POS{*8) calcule la position horizontale du chariot de Fimprimante par rapport a la marge de gauche (de coordonnee I). POS(P) calculelaposition logiqueducanal d'unitededisquettes,e'est-a-direle nom- bre decaracteres transmisdepuis lcdernier«retourchariot». Motsclesassocies : VPOS, WINDOW PRINT PR I NT [If <numerodecanal> ,][ <listede:<articlea imprimer> ]"##########, "page 177");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_122, r##########"10 aS="petite"
20 b*="Ceci est une longue chain* de caracteres"
30 PRINT a*;a*
40 PRINT a*, a*
50 PRINT
60 PRINT b*;b*
70 PRINT b*,b* ... run COMMANDE Transmet la >liste de:<article a imprimer> ou a afficher sur lecanal : indique, (i0 par defaut). MotsclesduBASIC Chapitre3 Page 59"##########, "page 177");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_123, r##########"10 PRINT"ceci est 1' instruction SPC"
20 FOR k=6 TO 15
30 PRINT SPCt5)"a"jSPCU);"b"
40 NEXT
50 PRINT"ceci est 1'instruction TAB"
60 FOR k=6 TQ 15
70 PRINT TAB(5)"an5TAB<x);"b" B0 NEXT run SPC menage le nombred'espaees vides indique par le <nombreentier> avant d'impri- meroud'afficherl'articleindique. aconditionquecederniertienneintegralement surla ligne. Tl est done inutiled'utiliserle point-virgule avee la commande SPC. TAB menage, a partirde la marge de gauche, le nombre d'espaees vides indique avant d'imprimer ou d'afficherl'article designc. a condition que cc dernier tienne surla ligne. Le point-virgule est done inutile apres TAB. Si le curseur a deja depasse la position demandee, un changement de ligneest effectue avant la tabulation. Chapitre3 Page 60 MolsclesduBASIC"##########, "page 178");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_124, r##########"10 FOR x= l TO 10
20 n=100B00*(RNDt5)
30 PRINT"marchdndi5 e";USING "##*#»»#*, *#" }
40 NEXT run PRINTUSINGpermetdedefinirleformatdepressionoud'affichaged'uneexpression transmise par la commande PRINT. On definit pourcela le <modele de format> sous leuuel on desire voir apparaitre 1'expression. On utilise comme <separateur> soit une virgule, soil un point-virgule. Le <modeledeformat> est unechainedecaracterescom- posesdes«indicateursdechamp» suivants Formats numeriques Dans un nombre § Chaque signe ? indique1'emplacement d*un chiffre. Exemple H##M# : Indique 1'emplacementdupoint decimal (equivalent a notrevirgule). #§W#M Exemple : (Reserveunespace). Cesigne.nepouvantfigurerqu'immediatement avant lepoint decimal, indique que leschiffres situes a gauche du point decimal seront disposes pargroupesdetrois(correspondant auxmilliers)separesentreeuxparunevirgule. Exemple : $§#!?##.•#*> Encadrement d'un nombre ££ (Reserve deuxespaces). Indiquequelesigne£ apparaitra immediatement avant le premierchiffre oulepoint decimal,e'est-a-diresur1'undesemplacements reserves auxchiffres. Exemple t£.MW$-M : " (Reserve deux espaces). Indique que tous les espaces vides situes avant le nombre seront combles par les asterisques. Motsci•es jduBDA*SoIiC^ Chapritre3 Page SI"##########, "page 179");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_125, r##########"10 CLSja$="abcde-fqhijklfflnQpqrst"
20 PRINT"chaine alphanum. = ";a*"##########, "page 181");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_126, r##########"40 PRINT USINS " !";a*
50 PRINTiPRINT "Avec \espaces\ =
60 PRINT USING "\ \";a*
70 PRINTiPRINT "Avec S< = ";
80 PRINT USING "&"sa*
98 SOTO 90 run ! Indiquequeseul le premiercaracterede lachainedoit apparaitre. Exemple : \<e$paces> \ ... % * Indique que seulsles x premierscaracteresdela chamedoivent apparaitre, xetant+ egal a la longueurdu format (barrescomprises). Exemple \ \ : & Indiqueque la chainedoit apparaitre «tellequelle». & Exemple ; Le <modelede format> d'unechaine ne peut exceder255caracteres. Tout <modeledeformat> peutetrerepresentparunevariablealphanumerique,comme montre l'exemple suivant le"##########, "page 181");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_127, r##########"10 a**"FF«##*M,.*#"
20 b*="!"
30 PRINT USING a*; 12345. 6789;
40 PRINT USING bS; "centimes" run Pourplusde detailsconcernantles formats, voir la deuxiemepartieduchapitre intitule « A vos heures de loisir...». Motsclesassocics: SPC, TAB, USING, ZONE K HotsciteduBASIC Chapitre3 Page"##########, "page 181");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_128, r##########"10 FOR n=l TO 8
20 READ a*,c
38 PRINT a*;" "; SOUND 1,ciNEXT
48 DATA voici 478,les,42i,B,379,notB5
50 DATA 358,de> ,3l9,li,2B4,ganM 253,. 239 run ( l COMMANDE DATA : Lit les donneescontenues dans une instruction et les assignea desvariables. READ passeautomatiquement d'unedonneea la suivante. Lacommande RESTORE permet de revenir a unecommande DATA anterieure. Pour plus de details, voir la deuxieme partie du chapitre intitule « A vos heures de loisir... ». Motsclesassocies : DATA, RESTORE Chapitre3 Page 64 MotsclesduBASIC"##########, "page 182");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_129, r##########"10 SOUND 65, 1000, 180
20 PRINT"appuyez CR] pour liberffr la note"
30 IF INK£Y(50)*~1 THEN 30
40 RELEASE 1 run COMMANDE Libere lcscanaux sonores bloques par la commandeSOUND. : Le parametre <canaux sonores> prend lesvaleurssuivantes"##########, "page 183");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_130, r##########"10 REM CHASSE AUX ENVAH1SSEURS DANS L' HYPERESPACE INTERGALACT1QUE
20 REM CQPYRI6HT by AMSQFT COMMANDE Insere une REMarquedansleprogramme. BASICnetient pascompte ; du <texte> situesur la ligneadroitede REM, meme si celui-cicomprend un separateur d'instructions " " ou tout autrecode. : On peut remplacer :REM par une apostrophe ' dans tous les cas, SAUF a l'interieur DATA. d'une instruction Motscles associes Aucun : dAoduBASIC Chapitre3 Page 65"##########, "page 183");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_131, r##########"10 AFTER 520, GOSUB 40 1
20 AFTER 100,2 GOSUB 50
30 PRINT"l_e programme tourne":QQTO 30
40 REM ce sous programme ne sera plus appele dans la mesure ou il a ete rendu innoperant en ligne 78
50 PRINT: PRINT "Le chronometre va "; 1
60 PRINT"etre supprime par REMAIN."
70 PRINT"il re5tait";REMAlNU>;"unites de temp* au chrono 1 run FONCTION : Lit le temps restant a decompter par le chronometre indique (de a 3), avant de le desactiver. Pour plus de details concernant les interruptions, consultez la deuxieme partieduchapitre«A vosheuresdeloisir... ». Motsclesassocies : AFTER, Dl, El, EVERY RENUM RENUM [<nouveau numerode ligne>][,[<ancien numerode ligne>][,<increment>]]"##########, "page 184");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_132, r##########"10 CIS
20 REM cette ligne deviendrajl igne 123
30 REM cette ligne deviendra: ligne 124
48 REM cette ligne deviendras ligne 125 RENUM 123,20,1 LIST COMMANDE RENUMerote les Iignesd'un programme. : L'oncien numero de ligne> est un parametre indiquant la ligne du programme a laquetleondesirecommencerlarenumerotation. Enl'absencedeceparametre,toutesles Iignes du programme seront renumerotees. Le <nouveau numero de ligne> indique le nouveau numerodela premierelignerenumerotee(1 pardefaut). L'<increment> indi- RENUM que I'espacement desire entre deux Iignes (10 pardefaut). opere les reajuste- ments neccssairesa I'intericurdesinstructionsd'appeltellesque GOTO et GOSUB. En a r p e p v a a r n a ch i e s , sa i n l t la d i a ss n e s i l n e c s h c a o n m g m e a l n es d n e u s me K r E o Y s , d R e E I M ig , nes C c H o A n I te N nu e s t d C a H n A s I de N sc M h E ai R n G es E d . c L c e ar s a n ct u e m r e e - s rosde Iignesdoivent etrecompris entre 1 et 65535. Motsclesassocies DELETE, LIST : Chapitre3 Page 66 MotsclesduBASIC"##########, "page 184");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_133, r##########"10 READ aliPRINT a»j" "j
20 RESTORE 50
30 FOR t*l TO 508:NEXTsGOTQ 10
40 DATA les data recuperes peuvent etre lus encore
50 DATA et encore run COMMANDE Ramene le pointeur sur l'instruction DATA indiquee. En 1'absencede parametre, le poi : nteurretournea la premiereinstruction DATAdu programme. Pourplusdedetails,consultezladeuxiemepartieduchapitre« A vosheuresdeloisir...». Motsclesassocies : DATA, READ RESUME RESUME [<numero de Iigne>]"##########, "page 185");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_134, r##########"10 ON ERROR GOTO 60
20 FOR x=10 TO BTEP-1: PRINT l/x:NEXT
30 END „.-: . ;
40 PRINTHje viens ici en cas d'erreur"
50 END
60 PRINT"erreur No."sERR!"a la ligne";ERL
70 RESUME 40 run COMMANDE Reprend 1'executiond'unprogrammeapresladetectionetletraitement d'uneerreurpar ; la commandeON ERROR GOTO. Si aucun <numerodeligne> n'est indique, I'executiondu programmereprendala lignecontenantl'erreurdetectee.Suppri- mezce parametredans Texemple ci-dessus,puis faites tourner le programme.
70 RESUME run Mots cles associes : O D N ER E L R E R R O L. R E G R O R T , O, ERROR, RESUME NEXT Mobc.esduBASIC Chapitre3 Pag* 67"##########, "page 185");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_135, r##########"10 ON ERROR SOTO 90
20 PRINT"tapez [ENTER] a chaque fois"
30 INPUT "i";a
40 INPUT " 2 " j a
50 inpot "3" ; as REM erraur da syntaxe
60 INPUT "4"}a
70 INPUT "5"ja
80 END
90 PRINT"erreur Na.";ERR;"a la liqne";ERL
100 RESUME NEXT run COMMANDE Reprend Pexecution dun programmeapresladetectionetletraitement d'une erreurpar : lacornmande ON ERROR GOTO. L'execution du programme reprend a partir de la ligne suivant immediatement la ligne erronee. Motsclesassocies DERR, ERR, ERROR, ON ERROR GOTO, RESUME : RETURN RETURN"##########, "page 186");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_136, r##########"10 GOSUB 50:PRINT "Apres le 60SU6":END
50 FQR n=l TO 20
60 PRINT"sous-program(B«" 70-NEXT:PRINT
80 RETURN run COMMANDE : Indique la fin d'un sous-programme. Apres Pexecutio G n O d' S un U s B ous-pro- gramme, BASICretourneaPinstructionsuivantimmediatementPappel corres- pondant. GOSUB Motsclesassocies : Chapitre3 Page 68 ' Mots clesduBASIC"##########, "page 186");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_137, r##########"10 MODE i!a*="ordinateur CPC 6128"
20 FOR n=l TO 16sL0CATE 41-n,n
30 PRINT RI6HT*(a$,n)
40 NEXT run FONCTION : Extrait un certain nombre de caracteres (entre et 255) a gauche d'une <chalne alphanumerique> . Si la chaineest pluscourtequela <longueur requise>, elle est utiliseeentierement. Mots cles associes : LEFT$. MID$ y i!/ RND RND ((<expression numerique> )]"##########, "page 187");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_138, r##########"10 RANDOMIZE
20 FOR x=l TO -i STEP -1
30 PRINT"para*etre RND="zx
40 FOR n=l TO 6
50 PRINT RND(x)
60 NEXT n,x run FONCTION : Foumit leprochainnombredelasequencepseudoaleatoireencourslor- que r<expression numerique> est positive ou lorsqu'elle ne figure pas dans la com- mande. Lorsque l'<expression numerique> est nulle. RND renvoie ledernier nombregenere. Unevaleurnegativedel'<expressionnumerique> lanceunenouvellesequencealeatoire, RND dont foumit le premierelement. Mots cies associes : RANDOMIZE -'AOO MotsclesduBASIC Chapitre3 Page 69"##########, "page 187");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_139, r##########"10 FOR n=4 TO -4 STEP -1
20 PRINT R0UNDU234.567B,fi),
30 PRINT "arrondi a";n| "deciinales"
40 NEXT run FONCTION Arrondit I'<expression numeriquo au nombredechiffres apres la yir- : gule ou de puissances de dix indique par le parametre <nombre de decimales>. Si ce parametreest negatif,l'expression est arrondiea unentier absolu, suivid'un nombrede zerosegala sa valeurabsolue. Motsclesassocies : ABS, CiNT, FIX, INT RUN RUN <chainealphanumerique> RUN "disc" COMMANDE Chargeetexecute unprogramme BASICou un programme-objetsitue : sur ladisquette.Tout programmedeja present en memoireest automatiquementecrase. Cettecommande permetd'accederdirectementaux programmes BASIC proteges. LOAD Motscles associes : RUN RUN [<numerodeligne>] RUN 280 COMMANDE Executele programme BASIC present en memoire,encommencantau : RUN <numerodeligne> indiqueou,adefaut,audebutdu programme. reinitialisetou- tesles variables. Cette commande peut ne pas donner acces aux programmes proteges charges en memoire. Motscles associes: CONT, END, STOP Chapitre3 Page 70 MotsclesduBASIC"##########, "page 188");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_140, r##########"10 FOR n=2B0 TO -208 STEP -20
20 PRINT" SGN renvoi"!
30 PRINT SGN<n) "pour un* valeur det ";n ;
40 NEXT run FONCTION: Etablit le SiGNe de Pexpression numeriquo. SGN renvoie les valeurs : -1 {si l'expression est negative), (si elleest nulle)et 1 (sielleest positive). ABS Motsclesassocies : SIN SIN (<expression numerique>)"##########, "page 190");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_141, r##########"10 CLStDEGrORIGIN 0,200 -^.\k«
20 FOR n=0 TO 720
30 y=SIN(n)
40 PLOT n*640/720,198tyjNEXT'
50 GOTO 50 run FONCTION : Calcule le SINusde i'<expression numeriquo indiquee. On peut exprimer 1'argument en degres ou en radians en utilisant, respectivement, les DEG RAD. fonctions et Motscles associes: ATN, COS, DEG, RAD,TAN Chapitre3 Page 72 Mots cl6sduBASIC"##########, "page 190");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_142, r##########"10 FOR z= TO 4095
20 SOUND 1,2, 1, 12
30 NEXT run COMMANDE : Permet la programmationd'unson,41'aidedes parametressuivants Parametre 1 : <etatdecanal> L'<etat decanal> admet pour valeurdesentierscomprisentre 1 et 255. La conversion en binairedece parametredonne la signification dechaque bit. selon la table decorres- pondance suivante Bit (1 en decimale) : sortir leson surlecanal A (Bitde poids faible) Bit 1 (2 en decimale) : sortir leson surlecanal B C Bit 2 (4en decimale) : sortir leson sur lecanal Bit 3 (8endecimale) : rendez-vous avec lecanal A Bit 4(16en decimale) : rendez-vous avec le canal B C Bit 5 (32 en decimale) : rendez-vous avec le canal Bit 6(64 en decimale) bloquer un canal sonore : Bit 7 (128 en decimale) vider uncanal sonore (Bit de poidsfort) : L'<etat decanal> 68, parexemple, aura l'effetsuivant Sortiesur 1ccanal C (4), a I'etat bloque (64). Parametre2 : <periodesonore> Ceparametreetablitlahauteurduson,e'est-a-direla«note»produite(parexempleDo, Re, Mi, Fa, Sol). Chaque note se definit par une valeur numerique representant sa <periodesonore> (Voir lechapitre « Pour information... ». Parametre3 <duree> : Ce parametreetablitlalongueur, ou «duree»,du son. 1 unitecorresponda uncentieme deseconde. La <duree> du son prend pardefaut la valeur 20,e'est-a-dire uncinquieme de seconde. Motscl6sduBASIC Chapifre3 Page 73"##########, "page 191");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_143, r##########"18 MODE 1
20 PRINT"met 9 espaces intra vous"!
30 PRINT SPACE$<9) j
40 PR!NT"et inoi " ! run FONCTION Creeunechained'espacesde la longueur indiquee(de a 255) : Motscles associes SPC, STRING$, TAB : SPC (Voir PRINT SPC) SPEED INK SPEED INK <periode !>,<periode 2>"##########, "page 193");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_144, r##########"10 BORDER 7,18
28 FOR =30 TO STEP -1 i i
30 SPEED INK i,i
40 FOR t=l TO 700:NEXT t,i run COMMANDE : Permetd'etablir la perioded'alternance lorsqu'une instruction INK ou BORDER present Tutilisation de deux couleurs intermittentes. Les durees respectives d'utilisation de la premiereetde la secondecouleursontindiquees en cinquantiemesde secondes par les parametrcs <periode ! > ct <periode 2>. Lors du choix des parametres. penscz aux risques d'effets secondaires hypnotiques Motscles associes : BORDER, INK MotsclesduBASIC Chapitre3 Page 75"##########, "page 193");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_145, r##########"13 CLSiFOR K=7 TO i STEP -2
20 PRINT"Entrez votre r>om puis [RETURN]" ,
30 SPEED KEY k,k
40 LINE INPUT a*sNEXT
50 PRINT"quel drale de nam !" run COMMANDE Etablit la vitesse de repetition automatique du clavier. Le parametre : <delaiinitial> fixeletempsdereaction(mesureencinquanticmesdesccondes)entreTen- foncementdela toucheet ledebutdelarepetitionautomatique. L'<intervalleinter-repe- titions> etablit le lapsde temps separant lesrepetitions. La commande SPEED KEY ne concerne que les touches pour lesquelles la repetition automatiqueexiste implicitement ou celles pour lesquellescette fonction aete program- meeau moyen de la commande KEY DEF. Avant dedefinir une repetition automatique a tres faible <delai initial>, il est prudent de programmer une des touches numeriques afin de pouvoir retablir les parametres par defaut de lafonction SPEED KEY (30,2). Voici comment proceder KEY 0, "SPEED KEY 30,2"+CHR*U3) II suffira, pourrevenir a Tetat initial, d'actionner la touche du pave numerique. Motsclesassocies KEY DEF : SPEED WRITE SPEED WRITE <nombre entier> SPEED WRITE I COMMANDE : Indique la vitesse dc transmission des donnecs de l'ordinateur vers un lecteurdecassettes, 1ccasechcant. Cette vitesseestsoitde2000 bauds(bitsparseconde) si ic parametreestegal a I, soit. pardefaut,de 1000 baudssicelui-ciestegala0. Lorsdu chargement d'un fichier enregistre surcassette, l'ordinateurchoisit automatiquement la bonne vitesse de lecture. SPEED WRITE est ledebit assurant la meilleurc fiabilite de transfert. Lacommande SPEED WRITE ne s'applique pasaux unitesde disquettes. Motsclesassocies OPENOUT, SAVE : Chapitre3 Page 76 MotsclesduBASIC"##########, "page 194");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_146, r##########"10 SOUND 65,100,100
20 PRINT SQ(1) -«ft-.«itt*!! • ..--. run 67 FONCTION : Indique I'etat de la file d'attente (Sound Queue) dans un canal sonore donne. Le <numcro de canal> doit ctre une expression numerique entiere prenant les valeurs A"##########, "page 195");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_147, r##########"10 FOR n»l TO 38:PRINT n:NEXT
20 STOP
30 FOR n»31 TO MsPRINT n;NEXT run cont COMMANDE Interrompt un programme, touten laissant a I'utttaateur la poss.bilite d'en reprendreI : 'execution au moyen dela commande CONT. STOP ; permetainsidnv terrompre un programme a un endroit donne afin d'effectuer une miseau point. CONT, END Motsclesassocies : STR$ STR$ <expression numerique> ( IB a=ScFFiR£M 255 hexad«ciMl"##########, "page 196");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_148, r##########"20 b»4tX 1 1 1 1 s REM 15 binaire
38 c*="**t"
40 PRINT c*+STRtU+b)+c* run tit 270*1* FONCTION Fournitsousformedechainealphanumeriquelarepresentationdecimale : de1*<expression numerique> indiquee.: Mots cles associes : BIN$, DECS, HEX$, VAL „ Chapitre3„ „ Page 78 Motsc!6sduBASSC"##########, "page 196");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_149, r##########"10 MODE 1:SYMBOL AFTER 105
20 rangeel=255;REM 11111111 en binaire
30 rangee2=l29:REN 10000001 an binaire
48 rangee3=189:REM 10111101 en binaire
50 rangee4=i53;REI1 10011001 en binaire
60 rangee5=153:REM 10011001 en binaire
70 rangee6=189:REM 10111101 en binaire"##########, "page 197");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_150, r##########"60 rangee7=i29;REM 10000001 en binaire
90 rangee8=255:REM 11111111 en binaire
100 PRINT"La ligne 110 redefinie la lettre i(105). Tapez que lques 'i' et regardez !"
110 SYMBOL 105, rangeel rangee2, rangeeS, rangee4, rangee5, range e6,rangee7,rangee8 , run COMMANDE : Rcdefinit la formed'uncaractereafftcheaI'ecran.Chacundesparame- tres prend une valeur entiere situee entre et 255. duBASIC Chapitre3 Page 79"##########, "page 197");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_151, r##########"10 CLS
20 SYMBOL AFTER 115 .\
30 PRINT"La liqne 40 redefinie la let. tre 5 . ;
40 SYMBOL 115,8,56,64,64,48,8,8,112
50 PRINT"en s" ,
60 PRINT"on revient a l'etat normal en tapant:
70 PRINT"SYMBOL AFTER 240" run COMMANDE : Fixela limiteinferieuredes numerosdecaracteresredefmissables(de a 255). La valeur par delaut du nombre entier est de 240, auquel cas on dispose de 16 caracteres redefinissables (entre 240 et 255). Lorsque le <nombre entier> a pour va- leur 32. tous les caracteres situcs entre 32 et 255 sont redefinissables. La eommande SYMBOLAFTER 256 interdit done toute redefinitiondecaractere. Laeommande SYMBOLAFTER retablitlavaleurpardefautdetouslescaracterespre- cedemment redefinis. MotsciesduBASIC Chapitre3 Page 31"##########, "page 199");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_152, r##########"20 PRINT"Quel va et vient "ja*}" !!"
30 TAG
48 x=LEN(a*)*17;y=50+RND*300:MOVE -x,y
50 FOR f=-x TO 640 STEP RNDI7+3
60 MOVE *,yiPRINT" " a$; FRAME: NEXT j
70 FOR b=640 TO -x STEP -RNDI7+3
80 MOVE b, yjPRINT a*;" "; sFRAMEtNEXT
90 GOTO 40 run COMMANDE : Ecrit le texte specifie a la position du curseuf graphique. Cette com- mande permet d'introduircdutexteetdessymbolesdans un graphiqueet de lesdeplacer pixel par pixel plutot que caractere parcaractcrc. Lc numcro dccanal prend pardefaut la vaieur ft 0. L'extremitegauchedelachatnedecaracteressepositionnesurlecurseurgraphique(Text At Graphics). Les caracteres de controle non visualises tels que lechangement de Iigne ouleretourchariotn'aurontaucuneffeta I'ecransi1'instruction PRINT esttermineepar un point-virgule ; danslecascontraire, ilsapparaitront sous Ieur forme graphique. Si 1'indicateur de canal est # (pardefaut), BASICannule la commande TAG lorsdu retouren mode direct. TAGOFF Motsciesassocies : Chapitre3 Page 82 Motscl6sduBASIC"##########, "page 200");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_153, r##########"18 MODE 2: TA6 J REM texte aux coordonnees graphtques
20 annee=1984:F0R x= l TO 640 STEP 68
30 MOVE x,400:DRAWR 0,-350
40 annee=annee+i:PR!NT annee;sNEXT
50 TAGQFFsREM retour aux coordonnees texte
60 LOCATE 28,25:PRINT MchiHres annuels"
70 GOTO 78 run COMMANDE: Annule la commande TAG concernant le canal indique (ff par defaut). Le textesetrouve doneanouveau dirigesur la positionducurseurde texte. TAG Motscles associes : TAN TAN (<expression numeriquo) PRINT TAN(45) 1.6197751V FONCTION Calcule la TANgente de I'<expression numeriquo, qui doit etre com- : priseentre-200000et +200000. On peut exprimer I' argument en degres ou en radians par 1'intermediaire des fonctions DEG et RAD, respectivement. Mots cles associes : ATN, COS, DEG, RAD, SIN Mo««*a<feiBASIC Chapitre3 Page 83"##########, "page 201");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_154, r##########"20 PRINT'Vous utilisez le stylo (pen) Not "}
30 PRINT TESTU2,394)
40 PRlNT"Changez de mode et de stylo"!
58 PRINT"... puis faites RUN." run FONCTION : Placelecurseurgraphiquea la position deftnieparx ety(encoordonnees absolues) et indique la valeur du parametre <encre> a cet endroit. Motsclesassocies MOVE, MOVER, TESTR, XPOS, YPOS : TESTR TESTR (<decalage x>,<decalagey>) /TAT"##########, "page 202");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_155, r##########"10 MODE 0:FOR x=t TO ISjLOCATE 1,x
20 PEN xsPRINT STRING*<IB, 143) tNEXT ;
30 HOVE 200,400:PEN 1
40 FOR n=l TO 23ilQCATE 12,
50 PRINT'pen";TESTR(0,-161:NEXT run FONCTION : Place lecurseur sur une position decoordonnees x et y par rapporta sa position actuelleet indique la valeurdu parametre <encre> a cet endroit. Mots cles associes : MOVE, MOVER, TEST, XPOS, YPOS THEN (voirIF) ChapitreS Page 84 MottCl6»duBASC"##########, "page 202");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_156, r##########"18 CLSiREK horlog*
20 INPUT "heure";haure
30 INPUT "<?iinut8"5f»inute
40 INPUT "seconde"iSBconde
50 CLS:donnee=INT(TIME/300)
60 WHILE heure<13
70 WHILE iunute<6B
80 WHILE tic<60
90 tic=(INT(TIME/30B)-donnee)+seconde
100 LOCATE 1,1
110 PRINT USING "## ";heure; minute;tic
120 WEND
138 ti c=0s5econde=0:*iinute=minute+l
140 GOTO 50
150 WEND
160 »inute=0iheure=heure+l
170 WEND
180 heure-1
190 GOTO 60 run FONCTION : Indique le tempsecouledepuisla misesous tension de Pordinateurou la derniere commande RESET {Ies temps de transfert entre Pordinateur et Punite de dis- quettenesont pascomptes). A chaquesecondecorrespond une foisla valeur ; TIME/300. Motsclesassocies : AFTER, EVERY, WEND, WHILE TO (Voir FOR) MotsclesduBASIC Chapitre3 Page 85"##########, "page 203");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_157, r##########"10 TROFFiPRINT;PRINT"TROFF"
20 FOR n=l TO 8
38 PRINT'Le programme tournt" 1NEXT
40 IF f»l THEN END
58 TRONiPRINTiPRtNT "TRON" 6(9 f»l!60T0 20 run COMMANDE Permetdesuivre1'executiond'un programmeparl'affichagedechaque : numero de ligne executee. Ce numero est affiche entrecrochets []. Cette fonction s'ob- tientau moyen de la commandeTRON. La commandeTROFF retablit Ie modenormal d'execution. La commande TRON est particulierement precieuse lorsque Ton desire suivre ligne parligne !edcroulement d'un programme afin decorriger uneerreur. Motscles associes Aucun ; UNT UNT(<adresse>) PRINT UNT(fcFF66> -154 COMMANDE: Convertit 1'argument en un nombre entier signe (en representation: complement a 2) compris entre -32768 et 32767. ROUND Motscles associes : CINT, FIX, INT, UPPERS UPPERS (<chaine alphanumeriquo) IB CLS: a*="me5 petites, coilifle voub avez qrandies !""##########, "page 204");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_158, r##########"20 PRINT;PRINT"presse2 une touche (1-9)"
30 a*=INKEY*:IF a*=n" THEN 30
40 n=VALU*}!lF n<l OR n>9 THEN 31
50 FOR x=l TO 12
60 PRINT n;"X";x5"=";n*x
70 NEXTiGOTO 20 run FONCTION Fournit la VALeur numerique du ou des premiers caracteres (y compris : lesignenegatifet !epointdecimal) de la <chaine alphanumerique> indiquee. On obtient la valeur lorsque lepremiercaracterede lachaine n'est pas unchiffre. Si le signe «-» apparait en premier caractere ou si celui-ci est un point decimal suivi d'un caractere non numerique, le message d'erreur «Type mismatch » (erreur de frappe) (13) s'afTichea l'ecran. Motsclesassocies : STR$ VPOS VPOS <numero decanal>) ( ft"##########, "page 205");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_159, r##########"10 MODE liBORDER BsLOCATE 8,2
20 PRINT"util isez les touches Hechees (haut/bas)"
30 WINDOW 39,39, 1,25:CURSDR 1,1
40 LOCATE 1, 13
50 IF 1NKEY(0)O-1 THtN PRINT CHRKil)}
60 IF INKEY(2K>-1 THEN PRINT CHR» 10) { ;
70 LOCATE #1,3,24
80 PRINTf1, "curseur texte ";
90 PRINT#1, "position verticals ="5
100 PRINT*!, VPQS(*B> sGOTO 50 run FONCTION Indique, sur 1'axe Vertical, la POSition du curseur de texte, a partirdu : bordsuperieurdela fcnetredetexte. L'indicateurdecanaldoit obligatoirement figurer il ne prend pas la valeur # pardefaut. WINDOW Mots cles associes : POS, dteduBASIC Chapitre3 Page 67"##########, "page 205");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_160, r##########"10 CL6 :PR NT "chronometre dt 10 socondes": t-TIHE
20 WHILE TII MEU +3000
30 SOUND 1,0,100,15
40 WENDiSQUND 129,40,30,15 run COMMANDE verifiee. Lemot d : e Re W p H et I c LE un i e nd s i ec q t u i e o l n ed d e e bu p t ro de gr la am se m c e ti t o a n n a t e q x u e ' c u u n te e r c t o a n n d d i i t s i q o u n e d 1 o ' e n x n p e r e e e s s - t sion Iogiquo definit lacondition a verifier. Mots cles associes TIME, WEN : Chapitre3 Page 88 Motscjg,sduBAS i c"##########, "page 206");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_161, r##########"10 MODE 0:BQRDER 0j REM cibU tv
28 INK 0,B:INK 1,25: INK 2,23iINK 3,21
30 INK 4, 17 : INK 5,6sINK 6,2: INK 7,26
40 PAPER 0:CLS
50 PAPER tsWINDOM 2,4,i,18sCLS
60 PAPER 2: WINDOW 5,7,1,18; CLS
78 PAPER 3:WIND0W 8, 1 0, 1 , 18;CLS
80 PAPER 4:WINDQW 1 1 , 13, 1, 18:CLS
90 PAPER 5iWIND0W 14, 16, 18iCLS"##########, "page 207");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_162, r##########"100 PAPER 6:WIND0W 17, 19, 18s CLS 1
110 PAPER 7: WINDOW 2, 19, 9, 25: CLS 1
120 GOTO 120 run COMMANDE Enmode texte, indique lesdimensionsd'un canal d'affichage a Tecran : (on parte dans ce cas de fenetre). On veillera a ce que les valeurs des parametres <gauche>, <droite>. <haut> et <bas> correspondent bien aux coordonnees en vigueurdans le MODF.-ecran utilise. Le <numero de canal> prendra par defaut la valeur ff 0. Pour plusde detailsconcemant les fenetres, voirla deuxierrie partie duchapitre intitule «A vos heures de loisir... ». WINDOW SWAP Motscles associes : MotaclfcduBASrC Chapitre3 Page 86"##########, "page 207");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_163, r##########"10 MODE 1: INK i, 24: INK 2,9:INK 3,6
20 WINDOW 21 ,40, 13, 25:PAF'ER 3
30 WINDOW Hi, 1,20, 1,12;PAPER #1,2
40 CLSsPRINT #1," Fenetre No"
58 CLS #1:PRINT #1," Fenetre No 1" '
60 LOCATE 1,6
70 PRINT" Fenetre Rouge (0)";SPC(2)
80 LOCATE #1,1,6
90 PRINT #1," Fenetre Verte U)"
100 FOR t=l TO 1000: NEXT
110 WINDOW SWAP 0,lsGQTO 60 run COMMANDE : Intervcrtit la premiere fenetreet la seconde. Lesdeux <numerosdecanal> doivent obligatoirement figurersansetreprecedes, dans cecas precis, dePindicateurdecanal if. Cettecommande permetdediriger lesmessages BASICsur unautrecanal quecelui par defaut # 0. Pour plusde details concernant les fenetres, voir la deuxieme partieduchapitre intitule « A vos heuresde loisir... ». NDOW Motsclesassocies Wl : WRITE WRITE [ » <numerodecanal>,][<donnees aecrire>]"##########, "page 208");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_164, r##########"10 REM ecrit des donnees sur la disquette
20 INPUT "donnez-moi un nombre";a
30 INPUT "donnez-moi une chaine de caracteres"} a$
40 OPENOUT "NOMFICH"
50 WRITE #9,a,a$
60 CLOSEOUT: PRINT "Les donnees sont sauvees sur disquette' run COMMANDE : Affiche ou ecrit (WRITE) desdonnees sur le canal indique. Deux arti- clesdistinctsdoiventetresepares parunevirguleet leschainesdecaracteressontplacees entre guillemets. Chapitre3 Page 90 MotsclesduBASIC"##########, "page 208");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_165, r##########"10 REM retrouve les dannees sur la disquette
20 DPENIN "NOMFICH"! INPUT *9,a,a$
30 CLQSEIN;PRINT"les 2 donnees sonts"
40 PRINT:PRINT a, a* run Motscles associes INPUT, LINE IN PUT : XOR XOR <argument> <argument> IF "alain" < "bernard" XOR "chien" > "chat" THEN PRINT "vrai" ELSE PRINT "faux" faux IF "bernard" < "alain" XOR "chat" > "chien" THEN PRINT "vrai" ELSE PRINT "faux" faux IF "alain" < "bernard" XOR "chat" > "chien" THEN PRINT "vrai" ELSE PRINT "faux" vrai PRINT AND"##########, "page 209");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_166, r##########"10 MODE liDRAW 320,200
20 PRINT"POSition X du curseur graphiquB=";
30 PRINT XPOS run FONCTION : Indique, surl'axe horizontal (X), la POSition ducurseurgraphique. Motsclesassocies MOVE, MOVER. ORIGIN. YPOS : YPOS YPOS"##########, "pages 209, 210");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_167, r##########"10 MODE ltDRAW 320,200
20 PRINT"PQSition Y du curseur graphique=";
30 PRINT YPOS run FONCTION : Indique, surl'axe vertical (Y), la POSition du curseur graphique. Motsclesassocies : MOVE, MOVER, ORIGIN, XPOS ZONE ZONE <nombreentier>"##########, "page 210");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_168, r##########"10 CLS:F0R z =2 TO 20
20 ZONE z
30 PRINT "X","X ZONE s-jzsNEXT run COMMANDE : Modifielalargeurdela tabulationdesignee par la virguledansla com- mande PRINT. La largeurdes zonesd'affichageou d'impression, (de 13 caracteres par defaut), peut ainsi prendre une valeurentiere quelconque entre 1 et255. Motsclesassocies PRINT : Chapitre3 Page 92 Motsel6eduBASIC"##########, "page 210");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_169, r##########"10 dumpf le*="flagdufflo.srii" i
20 MODE 1: BORDER"##########, "page 233");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_170, r##########"40 FOR 1^0 TO 2
50 READ colour (j): REM tharae les No de couleurs en 'DATA*
60 INK i.colour (i)
70 NEXT BB ON ERROR GOTO 430
90 OPENIN "param.dat" ' teste si le fichier exists
100 CLOSEINsON ERROR GOTO
110 IF errnum=32 AND DERR=146 THEN CLS:G0T0 160 •' le Hchier n'existe pas
120 CURSOR 1: PRI N f "Do yo>.> want to overwrite aid Hie? Y/N "
130 a$=INKEY*:0N INSTRC YN" , UPPER* ( a* t > GOTO 130, 150, 140:GO TO 130
140 PRINT a*:PRINT "Program abandoned": END
150 PRINT a*:CURaOR
160 LiPENOUl "par am.dat" 1/0 WRITE #9, durapt i le*, 1 : REM sauvegarde le nam du fichier e t son mode
180 FOR i=0 TO 2
190 WHITE *9,colour d j • REM sauvegarde les couleurs
208 NEX m T i
210 LLOSttlUT cls 2.50 qp-1:GRAPHICS PEN gp:w=125
240 x=-&5:«=248:y=40a:b=-15B:iiQSUB 400 Chapitre5 Page 6 Leselementsdei'AMSDOSelduCP/M"##########, "page 233");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_171, r##########"260 x=575:a=-240:y=400:b=-150:l3QSUB 400
270 v=0:b=150{BOSUB 400
280 gp-2: GRAPHICS PEN i}p:w=40
290 a=240:x--40:y=400:b=-i$0:BQSUB 400 >00 f-i&iv-0:b-i50: BUSUB 400
310 a=--240:x=640: y=0: b=U.0: GOSIIB 400
320 x=600:y=480:b=-150!GOSUB 400 iU liftI BIN 0, 0,1T>6, 380, 0,400: CI.G 1
340 ORIGIN 0.0,0,640, 150,250:CLS 1
350 ORIGIN 0,0,280,352,0.400:CLG 2
360 ORIGIN 0,0,0,640, 168, 238iCLG 2
370 SAVE dumpHle*,b,&C000,&4080
380 DATA 2,26,6
390 END
400 MOVE x,y:DRAWR a,b:DRANR w,0:DRAWR -a,-b
418 MOVE x+a/2+w/2,y+b/2:FILL gp
420 RETURN
430 errnum=ERR:RE5Ur1E NEXT run Lesindicateurs .DAT et ,SRN n'ontd'autrerolequedenous rappelercequecontient1e fichier. Lefichier PARAM.DAT contientdesparametrescodesen ASCII,donesansen- tete, alors que FLAGDUMP.SRN est un fichier AMSDOS encode binaire, avecen-tete. PARAM DAT Vousremarquerezqueleprogrammeessaied'abordde lire le fichier afin des'assurerqu'ilexisteavant d'y inscriredesdonnees. S'iln'existepas, le BASICsignale uneerreurqui estcaptureepar ie programmeet Pexecutionsederoulesansinterruption. S'il existe, le programme vous propose d'ecraser lefichierexistant. Lesparticularitydel'eeranqueTonvacopier,(sonmode,lapalettedecouleursetle nom dufichiercontenantlesdonnees)sontcopieessurun fichier.Voidquiillustre1'utilisation d'un fichier de donnees pour la sauvegarde WRITE des variables dans un programme (dumpflleS)etdeconstantes(1). pour reutilisation dans un autre programme. leselementsdel'AMSDOSetduCP/M Chapitre5 Page 1"##########, "page 234");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_172, r##########"10 DIM colour (15): REM provision for 16 colours
20 GPENIN "paraffl.dat"
30 INPUT #9,Hlename*,screenfflQde
40 i=0
50 WHILE NOT EOF
68 INPUT #9,colour(i)
70 INK i.coIourU)
80 i=i+l
90 WEND
100 CLQSEIN
110 MODE screenfflode:BORDER
120 LOAD filename* run Sommaire des commandes externes d'AMSDOS IA !A COMMANDE : Dirige les entrees/sorties vers l'unite A. Equivalent a | DRIVE avec A pour parametre. (L'unite integree est 1'uniteA.) IB B COMMANDE Dirigelesentrees/sortiesversl'uniteB.Equivauta DRIVEavecBpour : | parametre. (L'unite integreeestl'unite A.) Chapitre5 Page 8 LeselementsdeI'AMSDOSelduCP/M"##########, "page 235");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_173, r##########"10 QPENOUT "PROFILE.SUB"
20 PRINT #9,"5ETKEYS KEYS.CCP"
30 PRINT #9, "LANGUAGE 3"
40 CLOSEOUT Codes de controle de la console L'univcrs CP/M, c'est aussi un ensemble de codes clavier speciaux pour controler le deroulemcntdesprogrammes.Cescodesremplacentle[ESC]du BASICetlestouchesde deplacement du curscurdu BASIC AMSTRAD. Les codes de controle ci-dessous sont affectes apres lancement de la eommande : Lescodesdecontrole ci-dessous sontaffectesapreslancement de la eommande : SETKEYS KEYS.CCP Le programme transitoire SETKEYS.COM et le fichierdeeommande KEYS.CCP se trouvent surla face I d'unedesdisquettes systeme. Codedecontrole Touche Action [C0NTR0L1A <3 Deplacelecurseurd'uncaractereversla gauche [CONTROL]B [CONTROL]^ Deplace le curseurjusqu'au debut de la ou ligne ou. s'il s'y trouve deja, en fin de [CONTROL]^ ligne. [CONTROL]C [CONTROL][ESC] Abandon [CONTROL]E [CONTROL][RETURN] Retourchariot physique [CONTROL]F O Deplacelecurseurd'uncaractereversla droite [CONTROL]G [CLR] Efface lecaractere situesouslecurseur [CONTROL]H [DEL] Effacement du caractere precedant le curseur [CONTROL]! [TAB] Deplace lecurseurjusqu'a la tabulation suivante ;- [CONTROL^ „-. V.i,-: Validation de la ligne de eommande LeselementsdeIAMSDOSeiduCP/M Chapitre5 Page 19"##########, "page 246");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_174, r##########"600 NUL Pasd'effet. Ignore.
601 SOH 0a255 Affiche le symbole correspondant a la 1 valeur du parametre. Permel ainsi l'affichage dessymbolesde a 31"##########, "page 309");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_175, r##########"615 21 NAK Desactive I'ecran de texte. Rien ne s'affichera jusqu'a ce qu'un caractere ACK (&066)ait eteenvoye.
616 22 SYN a 1 Parametre modulo 2. supprime le mode transparent, 1 etablit le mode transparent."##########, "page 311");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_176, r##########"11 013 OB VT([CTRL]K) 61 075 3D = 111 157 6F"##########, "page 314");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_177, r##########"14 016 OE SO([CTRL)N) 64 100 40 (S 114 162 72 r
15 017 OF SI((CTRL|0) 65 101 41 A 115 163 73 s
16 020 10 DI£([CTRI_1P) 66 102 42 9 116 164 74 t
17 021 11 OC1(ICTRL)Q) 67 103 43 C 117 165 75 u
18 022 12 DC2((CTRLIR) 68 104 44 D 116 166 76 V
19 023 13 OC3((CTRL]S) 69 105 45 E 119 167 77 y
20 024 14 DC4([CTRLJT) 70 106 46 F 120 170 78 X"##########, "page 314");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_178, r##########"25 031 19 EM(ICTRL)Y) 75 113 4B K 125 175 7D I
26 032 1A SUB((CTRl)Z) 76 114 4C L 126 176 7E } 2? 033 1B ESC 77 115 4D H
28 034 1C FS*1 78 116 4E N
29 035 1D GS 79 117 4F
30 036 IE RS 80 120 50 P
31 037 IF US 81 121 51 8
32 040 20 SP 82 122 52 R 3 3 3 4 0 0 4 4 1 2 2 2 1 2 "; S 8 3 4 1 12 2 4 3 5 53 4 S T
35 043 23 # 85 125 55 U
36 044 24 $ 86 126 56 V
37 045 25 X 87 127 57 w
38 046 26 a 88 130 58 X
39 047 27 89 131 59 Y
40 050 28 ( 90 132 5A 1
41 051 29 ) 91 133 5B I
42 052 2A * 92 134 5C \
43 053 2B + 93 135 5D 3
44 054 2C t 94 136 5E t
45 055 2D 95 137 5F _
46 056 2E 96 140 60
47 057 2F ) 97 141 61 a
48 060 30 9 98 142 62 b
49 061 31 1 99 143 63 c 1 Chapitre7 Page 8 Dourinformation..."##########, "page 314");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_179, r##########"32 &H20 &X001ooooo rr --S --S :__
33 34 35 36 &H21 &H22 &H23 &H24 &X00100001 &X00100010 m &X0010 m 0011 &X00100100 i i in in i i _
37 38 39 40 &H25 &H26 &H27 &H28 &X00100101 &X m 00100110 &X00100111 &X00101000 m B:;: 44
41 42 &H2C &H29 &H2A &X00101001 &X00101010 s&MX00101011 &X00101100 H
45 46 47 48 &H2D &H2E &H2F &H30 &X00101101 &X00101110 &X00101111 &X00110000 Chapitre7 Page 9 Pourinformation."##########, "page 315");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_180, r##########"53 54 55 56 &H35 &H36 &H37 m &H38 &X00110101 &X00110110 &X00110111 &X00111000 m «:::
57 58 59 60 &H39 &H3A &H3B &H3C &X00111001 &X0m0111010 &X00111011 &X00111100 m^ www
61 62 63 64 &H3D &H3E &H3F &H40 &X00111101 &X00111110 &X00111111 &X01000000 FfH if as
65 66 67 68 &H41 &H42 &H43 &H44 &X01000001 &X01000010 &X01000011 &X01000100 Chapitre7 Page 10 Pourinformation...
69 70 71 &H45 &H46 &H47 &H48 &X01000101 &X01000110 &X010OO111 &X010O10O0 frrtvr \
73 74 75 76 &H49 &H4A &H4B &H4C &X01001001 &X01001010 &X01001011 &X01001100
77 78 79 80 &H4D &H4E &H4F &H50 &X01001101 &X01001110 &X01001111 &X01010000 Sff
81 82 83 84 &H51 &H52 &H53 &H54 &X01O10O01 &X01010010 &X01010011 &X01010100
85 86 87 88 &H55 &H56 &H57 &H58 &X01010101 &X01010110 &X01010111 &X01011000 Chapitre7 Page 11 Pour information .."##########, "pages 316, 317");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_181, r##########"89 90 91 92 &H59 &H5A 4H5B 4H5C 4X01011001 &X01011010 4X01011011 4X01011100 e p= !
93 94 95 96 4H5D 4H5E &H5F 4H60 4X01011101 &X01011110 &X01011111 4X01100000 Til IT"##########, "page 318");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_182, r##########"97 98 99 100 4H61 4H62 4H63 4H64 &X01100001 4X01100010 &X01100011 &X01100100
101 102 103 104 &H65 &H66 4H67 &H68 4X01100101 4X01100110 4X01100111 4X01101000 ;fflH=;
105 106 107 108 4H69 4H6A &H6B 4H6C 4X01101001 4X01101010 4X01101011 4X01101100 Chapitre7 Page 12 Pourinformation. .
109 110 111 112 &H6D &H6E &H6F &H70 &X01101101 &X01101110 &X01101111 &X01110000 P as fete rrrm i
113 114 115 116 &H71 &H72 &H73 &H74 &X01110001 &X01110010 &X01110011 SX01110100
117 118 119 120 &H75 &H76 &H77 &H78 &X01110101 &X01110110 &X01110111 &X01111000
121 122 123 124 &H79 &H7A &H7B &H7C &X01111001 &X01111010 &X01111011 &X01111100 srsffi I HE
125 126 127 128 &H7D &H7E &H7F &H80 &X01111101 &X01111110 &X01111111 &X10000000 Chapitre7 Page 13 Pourinformation.
129 130 131 132 &H81 *H82 &H83 &H84 &X10000001 &X10000010 &X10000011 &X10000100 _ 9 9 HI E st S-
133 134 135 136 &H85 &H86 &H87 &H88 &X10000101 &X10000110 &X10000111 &X10001000 1 JUUUBBBI
137 138 1 I 39 140 &H89 &H8A &IH8B &H8C &X10001001 & I X10001010 & I I X10001011 &X10001100 I -_Jw_
141 142- 143 144 &H8D &H8E &H8F &H90 &X10001101 &X1000111O &X10001111 &X10010000 ! ff _ l I il
145 146 147 148 &H91 &H92 SH93 &H94 &X1O010001 &X10010010 &X10010011 &X10010100 Chapilre7 Page 14 Pour information."##########, "pages 318, 319, 320");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_183, r##########"149 150 151 152 &H95 &H96 &H97 &H98 &X10010101 &X10010110 &X10O10111 &X1O01100O mm
153 154 155 156 &H99 &H9A &H9B &H9C &X10011001 &X10011010 &X10011011 &X10011100 m sii II;
157 158 159 160 &H9D &H9E &H9F &HA0 &X10011101 &X10011110 &X10011111 &X10100000
161 162 163 164 &HA1 &HA2 &HA3 &HA4 M &X10100001 &X10100010 &X10100011 &X101O0100 r««B IF FSB a
165 166 167 168 &HA5 &.HA6 &HA7 &HA8 &X101001O1 &X10100110 &X10100111 &X10101000 Pourinforma'.ion Chapitre7 Page ) S
169 170 171 172 &HA9 &HAA &HAB &HAC &X10101001 &X1Q10101O &X10101O11 &X10101100 I otbe Se
173 174 175 176 &HAD &HAE &HAF &HB0 &X10101101 &X10101110 &X10101111 &X10110000 assg is?
177 178 179 180 &HB1 &HB2 &HB3 &HB4 &X10110001 &X10110010 &X10110011 &X10110100
181 182 183 184 &HB5 &HB6 &HB7 &HB8 &X10110101 &X10110110 &X10110111 &X10111000 m #
185 186 187 188 &HB9 &HBA &HBB &HBC &X10111001 &X10111010 &X10111011 &X10111100 ChapiJre7 rage 16 Pont information.
189 190 191 192 &HBD &HBE &HBF &HC0 4X10111101 &X10111110 &X10111111 &X11000000 m ^iisS fe
193 194 195 196 &HC1 &HC2 &HC3 &HC4 &X11000001 &X11000010 &X11000011 &X11000100 m
197 198 199 200 &HC5 &HC6 &HC7 &HC8 &mX11000101 &X11000110 &X11000111 &X11001000 m m
201 202 203 204 &HC9 &HCA &HCB &HCC &X11001001 &X11001010 &X11001011 &X11001100 MHUWU
205 206 207 208 &HCD &HCE &HCF &HD0 &X11001101 &X11001110 &X11001111 &X11010000 Chapitre7 Page 1? Pourinformation."##########, "pages 321, 322, 323");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_184, r##########"209 210 211 212 &HD1 &HD2 &HD3 &HD4 &X11010001 &X11010010 4X11010011 &X11010100 »_ I m :: _-. _
213 214 215 216 &HD5 &HD6 &HD7 &HD8 &X11010101 &X11010110 &X11010111 &X11011000
217 218 219 220 &HD9 &HDA &HDB &HDC &X11011001 &X11011010 &X11011011 &X11011100
221 222 223 224 SHOD &HDE &HDF &HE0 &X11011101 &X11011110 &X11011111 &X11100000 FFtm i
225 226 227 228 &HE1 ' &HE2 &HE3 &HE4 &X11100001 &X11100010 &X11100011 &X11100100 Chapitre7 Page 18 Pourinformation...
229 230 231 232 &HE5 &HE6 &HE7 &HE8 &X11100101 &X11100110 &X1110O111 &X11101000 2S
233 234 235 236 &HE9 &HEA &HEB &HEC &X11101001 &X11101010 &X11101011 &X11101100 mm
237 238 239 240 &HED &HEE &HEF &HF0 &X11101101 &X11101110 &X11101111 &X11110000 m HB SSeE
241 242 243 244 I&HF1 &HF2 &HF3 &HF4 &X11110001 &X11110010 &X11110011 &X11110100
245 246 247 248 &HF5 &HF6 &HF7 &HF8 &X11110101 &X11110110 &X11110111 4X11111000 Pourinformation. Chapitre7 Page 19"##########, "pages 324, 325");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_185, r##########"249 250 251 252 &HF9 &HFA &HFB &HFC &X11111001 4X11111010 &X11111011 &X11111100 W m las
253 254 255 &HFD &HFE &HFF &X11111101 &X11111110 &X11111111 i... J. J Chapitre7 Page 20 Pour information."##########, "page 326");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_186, r##########"135 137 N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A IN/A RN/A 136
136 IS 133 N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A 111 133 a N/A 132 133 134 129 N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N/A N.A N/A N/A N/A N/A N/A N/A | 140 N/A N/A N/A 139 N/A N/A N/A 139 TOUCHES VALEURPARDEFAUT PROGRAM- MABLES CARACTEHE VALEURASCII 0(138) &30 1(129) 1 &31 2(130) 2 &32 3(131) 3 &33 4(132) t &34 5(133) 5 &36 6(134) 6 &36 7(135) 7 &37 8(136) 8 &38 9(137) 9 &39 10(138) &2E 11(139) [RETURN) &0D 12(140) RUN"[RETURN! &52&55&4E&22&0D Remarque: Lavaleurpardefautdeatouchesprogram- mable*13a431 (141 a 169)estegalesa0. Dasvaleurs et dei touches leur sorrt affactee*a laid* des com- mandes KEYetKEYOEFreapectivement. Chapitre7 Page 22 Pourinformation.."##########, "page 328");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_187, r##########"174 614 358 +0,.019% Fn
184 997 338 +0,,046% G 195 998 319 +0, .037% G# 207 652 301 +0.005% A 220 .000 284 -0.032% A# 233 .082 268 -0.055% B 246 .942 253 -0.038% NOTE FREQUENCE PERIODE ERREUR RELATIVE 261.626 239 +0.046% Do medium c c# 277.183 225 -0.215% D 293.665 213 +0.081% D# 311.127 201 +0.058% 329.628 190 +0.206% E 349.228 179 +0.019% Octave F F# 369.994 169 +0.046% G 391.995 159 -0.277% 415.305 150 -0.328% Gti A 440.000 142 -0.032% A international A# 466.164 134 -0.055% B 493.883 127 +0.356% Chapitre7 Page 25 Pourinformation..."##########, "page 331");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_188, r##########"10 Array already dimensioned (tableau deja dimensionne) Un des tableaux d'une instruction DIM a deja ete defini.
11 Division by zero (division par zero) L'ordinateur n'aime pasdiviserpar zero, quecesoit reel, entier,etc...
12 Invalid direct command (comiandt directe non valabla) Lacommande n'est pas acceptableen modedirect.
13 Type mismatch (types de variable nt correspondant pas) On adonne unevaleur numeriquepour unechainealphanumeriqueouvice-versaou un nombre non valable a etedecouvert par unecommande READ ou INPUT.
14 String space full (espace reserve aux chaines sature) IIyatellementdechainesqu'il n'y a plusde place, memeapres une remiseen ordre.
15 String too long (chaine trop longue) Une chaine a plus de 255 caracteres, ce qui peut arriver lors d'une concatenation dc chaines.
16 String expression too complex (chaine trop compliquee) Desexpressionsdechaines peuvent produiredes valeurs intermediairesqui, siellessont trop nombreuses, conduisent le BASICa donnerce message. Chapitre7 Page 28 Pour information...
17 Cannot CONTinue (on ne peut pas CONTinuer) LeprogrammenepeutpaspoursuivresonexecutionavecCONT,quisertapresunecom- mande STOP, [ESC][ESC] ou une erreur. Notez que si le programme a ete modifie entre-temps,il est impossiblede le relancer parcettecommande.
18 Unknown user function (fonction inconnue au bataillon) On a oubliededefinirla fonction FN avecla commande DEF FN auparavant.
19 RESUME missing (commande RESUME absente) Onatrouvelafinde programmealorsquecelui-ci procedait a un traitementd'erreur(a ON ERROR GOTO). la suite d'unedeclaration
20 Unexpected RESUME (RESUME inattendu) Ontombesurunecommande RESUME sansetredansunsous-programmedetype ON ERROR GOTO.
21 Direct command found (commande diracte tombant du ciel!) Enchargeant un programme, une lignesans numero s'estpresentee.
22 Operand missing (operands absent) Le BASIC vientde tomber suruneexpression incomplete.
23 Line too long (ligne trop iongue) Le BASIC n'acceptepas les lignesdeplusde 255caracteres.
24 EOF set (rencontre d'une fin de fichier) EOF = End OfFile = fin de fichier; le programme a effectue une tentative de lecture apres une finde fichier.
25 File type error (erreur dans le type de fichier) Le fichier n'estpasdutyperequis. OPENIN peut seulement ouvrirdes fichiersde texte ASCII. LOAD, RUN, etc., ne fonctionnent qu'avecdes fichiersproduits par SAVE. Pourinformation... Chapitre 7 Page 29
26 NEXT missing (NEXT manquant) On tiepeut pas trouver le NEXT quicorresponde a unecommande FOR.
27 File already open (fichier deja ouvert) linecommande OPENIN ou OPENOUT est executee avant que le fichierdeja ouvert n'aitete ferme,
28 Unknown command (commande inconnue) Le BASIC ne trouve pasde references a cettecommandeexterne.
29 WEND missing (WEND manquant) La boucle commencee par WHILE n'est pas terminee par WEND. :".>
30 Unexpected WEND (wend inattendu) Un WEN D est decouvert en dehors d'une boucle WHILE, ou un WEND necorrespond pas au WHILE de la boucle. "
31 File not open (fichier non ouvert) (Voir paragraphs ci-dessous, « Erreurs de disquette».)
32 Broken in (interrofflpu) .'? (Voir paragrapheci-dessous, « Erreurs dedisquette».) Erreurs sur disquette en AMSDOS Plusieurserreurspeuvent seproduirelorsdu traitementdesoperationsd'archivage. Bien que le BASIC les regroupe sous le numero d'ERReur 32, vous pouvez obtenirde plus amples informations en appelant la fonction DERR. Voici la signification des valeurs qu'elle renvoie : i Chapitre? Page 30 Pourinformation.."##########, "pages 334, 335, 336");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_189, r##########"14 142 (1 28+ 14) Etatducanal nonvalable.
15 143 (1 28+ 15) Finde fichier ma terielle. 144(128+16) Mauvaise commande, generalement 16 nom de fichier incorrect.
17 145 (1 28+ 17) Fichierdejaexistant. 1g 1 46 (1 28+ 18) Fichier nonexistant. 147(128+19) Catalogue sature. 19
20 148 (1 28+ 20) Disquette pleine. 2i 149(128+ 21) Changement de la disquette avec fichiers ouverts.
22 150(128+22) Fichieren lecture seulemen t
26 154 (1 28+26) Fin de fichierlogicielle. Si AMSDOS adejarapporteuneerreur,lebit7aprislavaleur 1, decalantcelledeDERR de 128. Lesautresvaleursrapporteespar DERR proviennentducontroleurdeladisquette,lebit 6etanttoujours sur 1. Le bit7indiquesi AMSDOS a rapporte l'erreurou non (voir ci- dessus). Voici la significationdechacun des bits Bit Signification Adresse manquante. Ecriture impossible. Disquette protegee. 1"##########, "page 337");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_190, r##########"10 ON ERROR GOTO 1000
20 OPENOUT "monfich.asc"
30 WRITE S9,"donnee-test"
40 CLOSEQUT
50 END"##########, "page 338");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_191, r##########"1000 aissdoserr=fDERR AND fc7F):REM bit 7 force a
1010 IF ERR<31 THEN END
1020 IF £RR=31 THEN PRINT "etes-vous sur d'avoir tape la liq ne20?":END
1030 IF amsdoserr=20 THEN PRINT "II n'y a plus de place sur la disquette ":END
1048 IF amsdoserr=«i000010 THEN PRINT"placez une disquette ncm protegee dans l'uriite, puis appuyej sur n'importe q u l1 e 0n,5y0r, e rE 1 N;lDr uche "!WHILE INKEY»="U : WEND:RESUME Partie 7 Mots cles du BASIC : (egalement appeles mots reserves) Voici unelistedes motsclesdu BASICAMSTRAD 6128. lissont reserveset nepeuvent etre utilises comme variables. ABS, AFTER, AND, ASC, ATN, AUTO BINS, BORDER " ^kb T AIN CHR$ C1NT CLEAR CLG CLOSEIN, CLOSEOrtUT, ', C F L t S ! , CO'NT, CO'PYCHR*', COS, > CREAL>, CURSOR £8: ""t.S%^'SWEFREflL ' DEFS,R ' DE6 ' DELETE - i&, E,^?E END ENT ENV E0F ERflSE ERL ERR| ix ' ' ' ' ' ' FILL, FIX, FN, FOR, FRAME, FRE Chaptoe? Page 32 Pourinformation..."##########, "page 338");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_192, r##########"15 s ft ft s s s « » 5 s s a K R 5 g R s Wi s S E e £ s"##########, "page 341");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_193, r##########"49 47 45 43 41 39 37 35 33 31 29 27 25 S3 31 19 17 IB 13 11 9 7 5 3 1
50 48 4« 44 42 40 38 36 34 32 30 . 28 26 24 22 20 19 16 14 12 10 PIN 1 SOUND PIN 18 AO PIN 35 INT PIN 2 GND PIN 19 D7 PIN 36 NMI PIN 3 A15 PIN 20 D6 PIN 37 BUSR2 PIN 4 A14 PIN 21 05 PIN 38 BUSAK PIN 5 A13 PIN 22 04 PIN 39 READY PIN 6 A12 PIN 23 03 PIN 40 BUS RESET PIN 7 A11 PIN 24 D2 PIN 41 RESET PIN 8 A10 PIN 25 01 PIN 42 ROMEN PIN 9 A9 PIN 26 DO PIN 43 ROMDIS PIN 10 A8 PIN 27 + 5v PIN 44 RAMRD PIN 11 A7 PIN 28 MREQ PIN 45 RAMDIS PIN 12 A6 PIN 29 Ml PIN 46 CURSOR PIN 13 A5 PIN 30 RFSH PIN 47 L. PEN PIN 14 A4 PIN 31 IORQ PIN 48 EXP PIN 15 A3 PIN 32 RD PIN 49 GND PIN 16 A2 PIN 33 WR PIN 50 PIN 17 A1 PIN 34 HALT Prise de I'unite de disquette 2 VUEARRIERE 3"##########, "page 346");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_194, r##########"17 16 15 14 13 12 11 10 9 6 7 6 5 4 3 2 1
35 34 33 32 31 30 29 2B 27 26 25 24 23 22 21 20 19 P P I I N N ? 1 S DO TROBE P P I I N N 2 1 0 9 G G N N D D PIN 3 D1 PIN 21 GND PIN 4 D2 PIN 22 GND PIN s D3 PIN 23 GND PIN 6 D4 PIN 24 GND PIN 7 D5 PIN 25 GND PIN 8 D6 PIN 26 GND PIN 9 GND PIN 23 GND PIN 11 BUSY PIN 33 GND PIN 14 GND PIN 16 GND Autres broches : I* Partie 10 Imprimante : Interfasage Le6128 peut etreconnecte a une imprimantestandard compatibleCentronics. Lecablederimprimanteestunesimpleconnexionentrelaprise PRINTERsitueea l'ar- rieredel'ordinateuret leconnecteurderimprimanteparallele. Lecircuit impnmede I or- dinateur presentedeuxcontactsdemoinsqueleconnecteurde rimprimante.Cecipermet l'utilisationd'unconnecteurstandard pourcircuitimprime. La partie9decechapitreillustreendetail laconfiguration desbroches. Lecabledoitpermettrelaconnexionde la broche 1 del'ordinateuralabroche 1 derim- primante dela broche 19del'ordinateura labroche 19derimprimante,etc. Toutefois, i les broches 18 et 36derimprimante de doivent pasctreconnecteesa l'ordinateur. Chapitre7 Page 41 Pour information..."##########, "page 347");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_195, r##########"607 BEL Signal sonore.
608 BS Espace arriere. Deplace le curseur d'une colonne vers la gauche. Si le curseursetrouvesurlapremierecolonne.maispassurlapremiereligne, etquela fonction deretourautomatiquealaligneest activee.llseplace sur la dernierecolonne de la lignesuperieure. &OA LF Sautdcligne. Faitdescendrclecurseurd'uneligne.et remonterle texte si necessaire. &OD CR Retour chariot. Place le curseur sur la premiere colonne de la meme ligne. &1B ESC Introduit une sequencedechangementde mode. Tous lesautrescodesdecontrolesont ignores. Les sequences de changement de modeci-dessous sont rcconriues. Tout autre caractere s ca u r i a v c a t n e t re le s c c a o r r a r c e t s e p re on E da S n C t e a s u t x af c f o i d c e h s c d e c n c d o e n p t l r a o c l a e nt &0 1 0 c c a ur & s 1 eu F r . . D c a e n q s ui de pc n r o m m c b t re d ti a x ff l i a c n h g e a r g l e e s s ^application le code de controle &09(TAB) correspond a des espaces et la sequence [ESC][TAB] n'affiche done pas 1ccaractere qui lui correspond. fESCIO Desactive la ligned'etat. Lesmessages systemesontvisualises par une sortieccran ordinaire, la ligned'etat devient dispomble. [Ejjqi Activelaligned'etat. Lesmessagessystemeapparaissentalorssur la ligne inferieure de l'ecran. [ESC12 <n> Change lejeu decaracteres(voirpartie 16decechapitre). nest le parametredela langucchoisic.masqueparlecode&07.Ccrtaincs matrices dc caracteres comprises entre &20 et &7F sont rempla- ceespard'autrescaracterescomprisentre&80ct &FF. Ccttecom- mandcagitdefacontrcssemblable ii la commandedccontroledes imprimantes possedant des jeux de caracteres definissablcs par logicicl. <n> = Etats-Unis <n> = France 1 <n> = 2 Allemagne <n> =3 Royaume-Uni < n > = 4 Danemark <n> = 5 Suede =6 <n> Italie <n> = 7 Espagne Chapitre7 Page49 Poorinformation"##########, "page 355");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_196, r##########"10 ' ANAGRAMMES par ROLAND PERRY
20 ' copyright (c) AMSOFT 1985
40 Rappelez vous de faire RUN"BANK«AN" avant de lancer VO'HUUrHmHU*$*HI*HI*»**ttltll*tMtt»t*UI*ttMI* 5
60 '
70 MODE 2
80 DEFINT a-z 9 1 0 00 rX I = N 0 P : UT 1BA " N D K o Q nn P e E z N,7 un mot de , 7 .letittres„}»«
110 IF LEN(s*)<>7 THEN 108 ."
120 PRINT " patientez. .
130 LOCATE 1 , 5! PRINT "computings
140 FOR cl*l TO 7
150 FOR c2«l TO 7
160 IF c2=cl THEN 370 ,
170 FOR c3=l TO 7
180 IF c3=c2 OR c3=cl THEN 360
190 FOR c4*i TO 7
200 IF c4=c3 OR c4=c2 OR c4=cl THEN , 350
220 "IF c5=c4 OR c5=c3 OR c5=c2 OR c5=cl THEN 348
240 R c6=c5 5r C6=c4 OR c6=c3 OR c6«c2 OR c6=cl THEN 330"##########, "page 367");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_197, r##########"288 LOCATE 12, f 5:PRINT x;o$ >
290 !8ANKWRITE 8rX,o* f
300 IF r%<0 THEN STOP
310 x=x+ t
320 NEXT c7
330 NEXT c6
340 NEXT c5
350 NEXT c4
360 NEXT c3
370 NEXT c2 3B0 NEXT cl .
390 lastrec=r%
408 ' pour les visualiser
410 rX=0:g$=SPACE*(7i
420 PRINT: INPUT "ANAGRAMME a RETROUVER (Utilise? <?> coiae ocker) m$ j
430 «*=LEF , T*(«*,7)
440 FOR x=l TO LEN(fflt)"##########, "page 368");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_198, r##########"470 iBANKFIND,3rX,is$,0,Ustrec
480 IF rX<0 THEN GOTO 420
490 !BANKREAD,3rX,g*
500 PRINT g* f
510 !BANKFlND,3rX,n$,rX+l,lastrec
520 GOTO 480 :hapitre8 Page8 Bank Manager : complementd'informations"##########, "page 368");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_199, r##########"21 = 2 = 2 = 2(2°)
22 = 4 = 2x2 = 2(2')
23 = 8 = 2x2x2 = 2(2 2 )"##########, "page 377");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_200, r##########"10 1010 A
11 1011 B C
12 1100 D
13 1101
14 1110 E mi
15 F
16 10000 10 Un nombre a 8 bits peutetredecompose en deux nombresde4 bits(appeles nibblesen anglais,cequ'on peut traduire parpetite bouchee...). Danscelivre, lesnombreshexadecimauxsontdesignesparlesymbole&, &D6 parexem- ple,et serventauxprogrammeursqui utilisent le langage assembleur,a mi-cheminentre le langage machineet le BASIC. Pourconverterde I'hexadecimat, vousdevezfaireattentiona multiplierle premierchiffire par 16et lui ajouterledeuxieme. Ainsi &D6donne (I3xl6)+6=214 et non pas 13 et 6 ou 136,comme on serait tente de lefaire. C'est le meme procede que celui du systeme decimal, a ceci pres qu'il est plus facilede multiplier par 10que par 16 ! Chapitre9 Psge12 Avosheuresdeloisii.."##########, "page 380");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_201, r##########"10 FOR n=32 to 255
20 PRINT CHR*(n);
30 NEXT ...n'acertainement plusde secretspour vous. Decortiquons-lemaintenant Nousconstatonstoutd'abordqu'aulieud'ordonnerPRINT« abcdefghi]klmn...etc», nousavonsentre PRINTCHR${n). netantunerepresentationpratiquede«variable». Une variable est un element d'information qui « varie» selon les instructions du pro- gramme. (Lechoixde n pour variableest tout a fait arbitraire, nousaurions pu prendre n'importequellecombinaison de lettres, pourvu qu'elle soit differented'un mot cle). Comment reconnaitre une variable? Lechiffre5estfixe,ilestcomprisentre4et6 ; il n'estdonepasunevariable. Lecaractere n est egalement fixe,c'est une Iettrede Palphabet. Avosheuresdeloisir..-. Chapitre9 Page 15"##########, "page 383");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_202, r##########"10 INPUT "Aujourd'hui nous sammes le"ijour
20 INPUT "Buel est le numero du mois";mois
30 IF jour=25 AND mais=12 THEN 50
40 CLSsSOTO IB
50 PRINT"Joyeux Noel ! OR opereegalement parbits,donnant1 danstouslescasou lesdeux bitsdesarguments different de 0, auquel cas le resultat est egal a 0. Avec les memes nombres que dans AND Pexemple : PRINT 1000 OR 10 1002 En binaire: 1010 111101000 1 Resultat: 1111101010 Dans un programme"##########, "page 386");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_203, r##########"10 CLS
20 INPUT "Quel est le nunero du mois"sffloi5
38 If mais"12 OR mais=l OR mois=2 THEN 50
40 SOTO 10
50 PRINT"Nous devrions etre en Hiver !" Avosheuresdeloisir... Chapitre9 Page 19"##########, "page 386");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_204, r##########"10 CLS
20 INPUT "Quel est le numero du mois";(«ois
32 IF NOT Uoi<s*i OR mais=7 OR mois=8) THEN 50
40 GOTO 10
50 PRINT"Nou5 ne pouvons etre en Ete !" On peutaussimelangerautantd'operateurslogiquesqu'oniedesire(danslalimited'une ligne) pourdonner, parexemple :"##########, "page 387");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_205, r##########"10 INPUT "Aujourd'hui nous soames le";jour
20 INPUT "Quel est le numero du mois";moi5
33 IF NOT mois=12 OR diois=l) AND jour=29 THEN 50 (
40 CLS:S0T0 10
50 PRINT"Ce n'est ni Deceitibre ni Janvier nais c'est peut- etre une annee bissextile* Le resultat d'une expression relationnelle est soit -1, soit 0. La representation binaire de -1 donne uneconfiguration binaire dont tous les bit valent 1, et cellede en donne unedont tous lesbitsvalent 0. Le resultatd'uneoperation logiquesurdetelsarguments donnera -1 pour vrai et pour faux. Ajoutonsdeux Iignes pourcompliquer :
60 PRINT N0T((nois=12 OR «nois=l >
70 PRINT (mois=12 OR mois=i! Si vous lancez Ie programme avec 29 pour lejour et 2 pour le mois, vous obtenez la reponsede la ligne 50 et les valeurs produitespar lesexpressionsdes Iignes 60 et 70. Pour finir, XOR (OU eXcIusif) donne un resultat vrai si les deux arguments sont differents. Resumons par un tableau susceptible de donner une idee plusclairede la situation. Ce typede representation est appele tablede verite d'une fonction. ARGUMENTA 1010 ARGUMENT B 0110 AND Resultat (et) 0010 OR Resultat (ou) 1110 XOR Resultat (ou exclusif) 1 100 Chapitre9 Page20 A vosheuresdeloisir.."##########, "page 387");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_206, r##########"10 MODE
20 POR n=0 10 7
30 WINDOW #n,n+l,n+6,n*l,n+6
40 PAPER *n,n+4
50 CIS #n
60 POR c«l TO 200:NOi c
70 NEXT n IIdefinithuitfenetresdecouleursdifferentessechevauchant.Lorsquel'executiondupro- gramme est terminee et que le message « Ready» apparait, appuyez sur ENTER plu- sieursfoiset observez le changementdecouleurdel'ecran queprovoqueledefilement de la fenetre 0. Les blocs de couleur defilent, mais Femplacement des autres fenetres ne change pas. Entrez maintenant CL5 #4 Chapitre9 Page28 Avo*heuresde loisir."##########, "page 395");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_207, r##########"10 HOOE a
20 a=l+RND*l(?:b=l+PNl.>*lV
30 c-l+RNOI24:d=i+RND124
40 e=RND*15
50 WINDOW a,b,c,d
60 PAPER e:CLS
70 6010 20 Puis-je vous interrompre? Vous1'avczpeut-ctreremarque.Tunedesinnovationslogiciellesmajeuresdesordinateurs AMSTRAD consiste a gererlesinterruptions a partirdu BASIC : end'autrestermes, le BASIC AMSTRAD estcapabled'executeruncertain nombredc tachesindependantesa Pintericurd'un memeprogramme.Cettecapacite.connuesousle termedc«multitache», est gerce par les commandos AFTER et EVERY. La possibility de mettre les sons en file d'attente et de les synchroniser par des rendez- vous represente un autreexempledecetteparticularite. Lasynchronisationdusystemeincombea sonhorlogemaitresse. Elkest constituted'un systeme de synchronisation a quartz, integre dans Pordinateur veillant a Porganisation chronologiquedesevenements (le balayage de Paffichage et la frequence du processeur, parexemple). Toutesles fonctions materiellesse rapportantau temps peuvent se retrou- ver dans 1'horloge a quartzdusysteme. L'implementation logicielledecettesynchronisationappartient auxeommandesAFTER et EVERY. Fidelcsa laconvivialitedu BASIC AMSTRAD, celles-cifont exactement ce qu'elles indiquent : AFTER (apres) la duree definie dans la commande, le programme donne lecontrole au sous-programme indiquequi execute la tachedefinie. Avosheuresdeloisir... Chapitre9 Page29"##########, "page 396");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_208, r##########"10 MODE l;n=<14;x=RND»400
20 AFTER x,3 GOSUB 80
30 EVERY 25,2 GOSUB 160
40 EVERY 10. GOSUB 170 1
50 PRINT"Testez vos reflexes"
60 PRlNT"Appuyez sur la barre espace."
70 IF Haq=l THEN END ELSE 70
80 2=REMAIN(2)
90 IF INKEY(47>=-i THEN 110
100 SOUND l,900sPRINT"Tricheur !";G0TD 150
110 SOUND 129,20:PRINT"MAINTENANT":t=TIME
120 IF INKEY<47)=-1 THEN 120
130 PRINT"vous avez mis";
140 PRINT <TIME~t>/300; "seconds"
150 CLEAR INPUTsHagaljRETURN
160 SOUND 1,0,50;PRINT". ";:RETUKN
170 n=n+l:IF n>26 THEN n=14
180 INK l,n:RETURN LescommandesAFTER et EVERY peuvent intervenira toutmoment pour reinitialiser le sous-programme concerne en accord avec le chronometre. Ces deux commandes se partagent leschronometres, unecommandeAFTER annulant toutecommande EVERY anterieure pour un chronometredonneet inversement. Lescommandes Dl et Eldesactiventetactiventlesinterruptionsdechronometretouten permettant 1'execution des commandes les separant. Une interruption de prioriteelevee est ainsi dans I'impossibilite de perturbcr le deroulement d'une interruption de priorite moindre. La fonction REMAIN desactive lecomptaged'undeschronometresetrestitue le nombred'impulstons restantes avant 1cdeclenchement de 1'interruption. 2iapitre9 Page30 Avosheuresdeloisir"##########, "page 397");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_209, r##########"10 READ a,b,c
20 PRINT"les nombres sont";a; "et";b ; "et";c
38 DATA 12, 14,21 run
118 INPUT "fcntres 3 nombres separes par une virqule";a,b,c i.< 20 PRINT "les notnbres sent";a; "et }b; "et"5c ' run i* Des virgules separent lesdifferentselementsd'unecommande DATA, tout eommepour commande INPUT. la Outre des valeurs numeriques, les commandes DATA peuvent contenirdes constantes alphanumeriques"##########, "page 398");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_210, r##########"10 DIM a*(i0)
20 FOR i=0 TO 9
30 READ a*<i)
40 NEIVT
50 FOR 1=0 TO 9 ' 69 PRINT a$(i) ; " ";
70 NEXT"##########, "page 398");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_211, r##########"18 READ a$
20 WHILE a$0"*"
30 PRINT at
40 READ a$
50 WEND
60 DA7A J'ai longtemps, habite,sous de vastes portiques
70 DATA Que, des, saleils marins, teiqnaient, de mille feux
80 DATA t run Lachainede ia Iigne 60comportedesvirgulesentramant Ielectureet l'affichagedecha- que partie. Parcontre, la chainede la ligne 70 estdelimitee pardes guillemets, et sera done affichee comme un tout. READ L'exemple ci-dessus montre que les donnees peuvent occuper plusieurs lignes. traiteleslignesdehauten basdansl'ordredesnumerosdeligne(60,70,80,etc.). Par DATA ailleurs, les instructions peuvent se trouver n'importe ou dans Ie programme avant ou apres Ia commande READ qui extrait les informations. Si un programmecontientplusieurscommandesREAD, la secondereprend laou la pre- miere s'arrete"##########, "page 399");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_212, r##########"10 DATA 123,456,789,321,654,2343
20 FOR i=l TO 5'
30 READ nomb
40 total=total+nomb
50 NEXT
60 READ tota!2
70 IF total=toU12 THEN PRlNT"les donnees sont just«" ELSE PRINT" t1 y a une erreur dans les donnees" run Essayezde modifier Ia ligne 10 en introduisant une erreurdans l'un des cinq premiers nombres, puisiancezleprogramme.Cettemethoded'insertion,a lafind'unecommande DATA,d'unevaleursupplementalegaiea la sommedetouteslesautresvaleursconsti- tue un bon moyen de detection d'eventueiles erreurs, surtout lorsque les lignes DATA sontnombreuses. Cettemethodes'appelle sommedecontrole (checksum). Si un programme faitappel a desdonnees mixtes(chaineset nombres), ilest possiblede lescombinerdans lesinstructions DATA et READ, a condition que leselements soient luscorrectement. Parexemple, si DATA contenaitdessequencesdedeux nombressuivis d'unechaine, seul l'emploi d'unecommande READ suiviededeuxvariablesnumeriques et d'unechaine seraitjustifie : Chapitre9 Page32 A yosh«ur»d©lowr. ."##########, "page 399");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_213, r##########"10 DIM a<5) ,b(5) ,p$(5)
20 FOR i=l TO 5
30 READ a(i),bU) ,p*<i)
40 NEXT
50 DATA i,7,fred,3,9 fr*ncoii,2,2,dany,4,6,olivier,9 l tric ( l l
60 FOR i=l TO 5
70 PRINT p*<i), B B;a<i)*bm- :
80 NEXT Vouspouvez, pourchanger, separer lesdifferents typesdedonnees"##########, "page 400");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_214, r##########"10 DIM a<5) ,b!5) ,p$<5)
20 FOR i=l TO 5
30 READ aft ,b(i )
40 NEXT
50 FOR 1=1 TO 5
60 READ p$<i)
70 NEXT
80 DATA 1,7,3,9,2,2,4,6,9,1
90 DATA fred,francais.dany,Olivier,eric
100 FOR i=l TO 5
110 PRINT pKii , "i ";a<i llhd 1
120 NEXT Silaboucle FOR de la ligne20sechangeen"##########, "page 400");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_215, r##########"15 RESTORE 80
45 RESTORE 90 Lacommande RESTORE place lepointeur-lecteurde DATA a la lignespecifieeetpeut ainsi servirdans unccommandeconditionnelle pour lirecertains blocsdedonneesselon descriteresdonnes. Decettemaniere,dans unjeuaplusieursniveaux possedant plusieurs ecrans, les donnees de chaque ecran pourront etre extraites en fonction d'une variable (leurniveau, par exemple). Voici unprogrammedece type : A vosheuresdeloisir... Chapitre9 Page33"##########, "page 400");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_216, r##########"1000 REM section de dessin sur ecran
1018 IF niveau=t THEN RESTORE 2010
1028 IF niveau=2 THEN RESTORE 2510
1030 IF niveau=3 THEN RESTORE 3010
1040 FOR y=l TO 25
1050 FOR x= l TO 40
1060 READ char
1070 LOCATE x,y;PRINT CHR$(char);
1080 NEXT x,y' •"##########, "page 401");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_217, r##########"2000 REM DATA pour 1'ecran 1
2010 DATA 200,190,244,244,210 ...etc"##########, "page 401");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_218, r##########"2500 REM DATA paur 1'ecran 2
2510 DATA 100,103,245,243,251 ...etc"##########, "page 401");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_219, r##########"10 FOR i=l TO 3
20 RESTORE 100
30 READ note
40 WHILE note <>-t . •
50 SOUND 1, note,35
60 READ note . ..,.»,^v
70 WEND
80 NEXT
90 SOUND 1, 142, 100
100 DATA 95,95,142,127,119,106
110 DATA 95,95,119,95,95,119,95
120 DATA 95,142,119,142,179,119
130 DATA 142,142,106,119,127,-1 run ;hapitre9 Page34 Avosheuresdeloisir."##########, "page 401");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_220, r##########"32 peuventegalement etre utilisees,afind'indiquerqu'un son produit sur un canal a ren- dezvousavecunautrecanal(A, Bet C respectivement).Qu'entendons-nousparrendez- vous?C'est tressimple : lessonsquenousavons fabriquesjusqu'a presentetaientimme- diatementjoues sur lecanal indique. Essayezdonecequi suit SOUND 1,142,2000 SOUND 1,90,200 Chapitre9 Page36 Avosheuresdeloisir..."##########, "page 403");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_221, r##########"10 FOR a=l TO 8
20 BOUND 1,1001a, 2U
30 NEXT
40 PR NT"bonjour" I run Vous constatezque le mot" bonjour "nes'affichequ'au bout d'un certain temps,apres execution des trotspremiers sons. Ceretard tient au fait que 1'execution du programme ne peut continuer tantqu'une place nes'est pas libereedans la file. Le BASIC possede un mecanisme d'interruption assez similaire a celui qu'utilisent les commandesAFTER, EVERY ou ON BREAK GOSUB. fl donneaccesa unsous-pro- gramme d'emission sonore qui n'est appele que lorsqu'une place se libere dans la file demandee. En voici un exemple :"##########, "page 411");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_222, r##########"10 a=
20 ON SQQ) GOSUB 1000
30 PRINT a;
48 SOTO 30"##########, "page 411");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_223, r##########"1000 a=a+10
1010 SOUND l,a,230
1020 IF a<200 THEN ON SQU) GOSUB 1000
1030 RETURN run Ce programme, comme vous pouvez Ie constater, n'effectuejamais de pause. La com- SOUND mande n'estappeleeque si la filed'attenteducanal A{numero 1)contient une place libre, ce que la commande ON SQ{1 ) GOSUB est chargee de verifier a la ligne 20. Cette commande initialise un mecanisme d'interruption qui appelle le sous- p G r O o S gr U a B mme sonore lorsqu'un espace libre apparait dans la file. La commande ON SQ doit etre reinitialisee apres appel du sous-programme, ce que fait la ligne 1020. Dans notre exemple, le sous-programme sonore se reinitialise tant que «a » est inferieur a 200. Chapitre9 Page44 Avosheuresde loisir..."##########, "page 411");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_224, r##########"28 x*S8<2>
30 PRINT BIN*(x> run Le nombre binaireobtenu (1 O0OO100)contient lebit 7 indiquant que lecanal etait en activite au moment ou la fonction SQ a etc utilisee. Les trois dermers chiffrcs (100) c f o o r nc r t e i s o p n on p d e e r n m t et au dc no co m n b n r a e it d r e e c I i ' m e a ta l t 4 d' : u il ne re f s i t l e ea do u n n e p q o u i a n t t r d e o p n l n ac e e d s u li p br r e o s g d r a a n m s me la , f a il l e o . r C s e q t u t e e ON SQ( GOSUB interrogecelui-ci,puisreagitenconsequenceaunmomentimprevi- ) siblc. Lesexemplescitesjusqu'apresentneprevoyaientremissionqued'une voiredeux notes a la fois. Nous allons maintenant considerer la possibilite de manier des ensembles de e n n o t t r e a s n i t n s d o e u p s en f d o a r n m t e es d' l u e n s e u c ne o s mm de a s nd au e tr D es AT (u A n l m u o e r ( c R ea E u AD d ) e m p u a s r i l q a u c e, om p m a a r n e d x e em S pl O e U ) N e D n , les _,,... Chapitre9 Page45"##########, "page 412");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_225, r##########"10 FOR octave*-! TO 2 -.<3**Ml»H*lWib«iwv
20 FOR x=I TO 7:REM notes par octave ' : "•'"V; '
30 READ note
40 SOUND l,note/2Toctave
50 NEXT
60 RESTORE
70 NEXT
80 DATA 426,379,358,319,294,253,239 run Notre dernier exemple de programme sonore montre ce qu'il est possible de realiser a partir de ces elements. Un rythme et une melodic sont joues sur les canaux A et B synchronisesaumoyenderendez-vous. L'exempfe montrecomment introduiredans une DATA intruction desinformationsconcemant lesnotes, lesoctaves, lesdureeset lesren- dezvous :"##########, "page 413");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_226, r##########"10 REM liqne 190 donne la clef de SOL
20 REM ligne 200 donne la clef de FA
30 DIM gammed 12):FOR xX=l TO 12;READ qtmel (xX) :NEXT
40 calXMtREAD cal*!ca2"/.=l:READ ca2$ a ;„
50 cls
60 vitX=12
70 gamme*=" a-b b c+c d-e e f+f g+g"
80 ENV 1,2,5,2,8,-1,10,10,0,15
90 ENV 2,2,7,2.12,-1,10,10,0,15
100 ENT -1,1,1,1,2,-1,1,1,1,1
110 DEF FNm*(5* 5}=MIDt(s$,5,l)
128 cal'4=l GOSUB( 200 :
130 ca2X=l;GOSUB 380
140 IF calX+ca2X>0 THEN 140
150 END
160 DATA &777,&70c,&6a7,!<647 fc5ed,fc598
170 DATA 4547,&4fc,fc4b4,4470, 1 &431,fc3M
180 DATA 4cr4f4fHlglal-b2c2Hg2glal-b6a2crlflolal-blal-b2c2 9*a*9*Ugla2g2f6e2c2e2c2g2e2cl-bl«292He4d8c4f3nc2iJ4-b2fr2- b2a2g2f6e2gr 4c4-blalH-blg2c2-b4a4g4{r6a2a2-b4b2ar2-b2a2g2Ue2g4c4-blaH I~blg2c2-b4a4g8f .
190 DATA r4f4f8f4e4c4fr8f4e2f2e4d2e2d8cBc6e2f4a4g8e4f3flc4dr 8g4cr4e4c6f2d4c4c8fr8-e4dr8g8c4e4c6^2d4c4c8f.
200 REM envoi le son sur le canal A
210 pl*=FNm*(cat*,ealX)
220 IF pltO-r" THEN rU=0;SOTO 240
230 r1X=16:catX=calXH:pl$=FNfit$(cal*,calZ) :hapi!re9 Page46 Avosheuresdeloisir...
240 IF pl*="." THEN ca tX»0i RETURN ELSE LIX'VALtpl*)
250 caU=caU+t cam •260 nl*=FNm*<calf,
278 cal7.=calX+l 2B0 IF nlt='+" OR nl*="-° THEN 350
290 nl*=" "+nl$
300 nd1X=(1+ 1NSTR (gamine*,LOWER* (n $>) )/2 1
310 IF ASC(RIGHT$(niS, 1) ) >96 THEN olX=8 ELSE olX=16
320 SOUND l+rl7.,gaMe7.(ndl7.)/an,vitX*I17., 1,
330 ON SOU) GOSUB 200
340 RETURN
350 ni$=nl$+FNm$(cal*,cal7.)
360 cal7.-cal7.+ l
370 GOTO 300 .
380 REM envoi le son sur le canal B
390 p2$=FNm*(ca2$,ca2X)
400 IF p2*<>"r" THEN r27.=0:GOTO 420
410 r2X=8:ca2X=ca27.+l:p2*=FNm*(ca2*,ca2X) *-; 420 IF p2*="." THEN ca7.=0; RETURN ELSE 127.*VAL(p2*! -;'- 430 ca27.=ca2V.+ l
440 n2*=FNm$(ca2*,ca27.) .-, .
450 ca27.=ca27.+l
460 IF n2S="+" OR n2*="-" THEN 530
470 n2*=" "+n2$
480 nd27.=(l+INSTR (gamine*, LGWER*(n2$) ) ) 12"##########, "pages 413, 414");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_227, r##########"510 ON 3Q(2) SOSUB 380
520 RETURN
530 n2*=n24+FNm*(ca2*,ca2X)
540 ca27.=ca27.tl
550 GOTO 480 run ::,;•:r.f-.»fift. h.;: -.:. ,- /:;«:/? Si nous parlions graphiques ? La partiequisuitestconsacreeaux fonctionsgraphiquesde1'AMSTRAD664,dontnous alions, a partir d'un premierexemple, decouvrir tour a tour les pointsessentiels. Nousalionscommencer par partager Pecran en deux : une fenetrede texte(tigne40)et une fenetre graphique (ligne 30). Puis nous choisirons tin MODE, ainsi que deux cou- leursclignotantes (ligne 20). Avoaheuresdeknsir... Cbapitre9 Page47"##########, "page 414");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_228, r##########"10 REM MASK et TAS dans les fenetres
20 MODE 1 ! INK 2,10,4: INK 3,4,10
30 ORIGIN 440,100,440,640,100,30(3
40 WINDOW 1,26,1,25
50 CLS 2 En executant ce programme, vousallezvoirapparaitre uncarreclignotant au milieude lamoitiedroitede1'ecran. La ligne50attribueacecarre I'encre numerodeux(bleusom- breetrosealternes). La ligne30positionneiepointd'originedescoordonneessurlecoin inferieur gauche du carre tandis que la commande MODE y place le curseur (X= 0, Y=0). On peut maintenant tracer unediagonale a l'aidede la ligne 60 :
60 DRAW 200,200,3 Executezle programme, puisajoutez la lignesuivante
80 MOVE 0,2:FILL 3 La ligne 80 pose le curseur graphique a Pinterieur d'une des moities du carre, qu'elle colore au moyen de i'encre 3. Les limites de cecoioriage sont le bord de la fenetre (qui estaussiceluidenotrecarre)et toutelementgraphiquetraceparlestylographiqueutilise (le 3)ou pour lequel on a indique la memeencrequecelledu remplissage (la numero 3 egalement), Executezle nouveau programme : Verifiez ce qui a ete dit a propos des limites du coioriage, en ecrivant la ligne 70."##########, "page 415");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_229, r##########"78 GRAPHICS PEN 1 run Remarquezbien que si I'encrede la diagonalen'etaitpasla memequecelleducoioriage, c'estlecarretoutentierquiseraitcolorie,cequevouspouvezverifieren remplacant FILL 3par FILL 1 dans la ligne 80. Revenezensuitea I'etatinitial (FILL3). Dessinons maintenant un cadre
100 MOVE 20,20
110 DRAW 180,20 „/* 120 DRAW 180,180 v " 130 DRAW 20, 180
140 DRAW 20,20 run Chapitre9 Page48 Atottutoxesdeloisir..."##########, "page 415");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_230, r##########"90 MASK run Mais pourquoi ce programme ne respecte-t'il pas la sequence choisie lorsqu'il trace un c p o r i e n mi ? e T r o p u o t in s t im d p es le l m i e gn n e t s p q a u r i c l e e q c u o e mp ! o e s c e o n i t n . es O t n tr p a e c u e t d c e o u n x to fo u i r s n : e e r n ce ta p n r t o q b u l e em d e er c ni o e m r m p e oi c n e t ci et
115 MOVE 160,22
125 MOVE 178,180 f 135 MOVE 28,178 run On obtient bien l'effet desire, mais il y a beaucoup plus simple. Si Ton ajoute a la com- mande MASK unsecond parametre " ,0",l'ordinateurnetracepaslepremierpointde 90 chaqueligne. Corrigeons alors la ligne :"##########, "page 416");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_231, r##########"50 CLG 2:GRAPHICS PAPER <D ...et faisons toumer le programme : la couleur du papier se detache maintenant tout autour du cadre. Onpeutegalementchoisirun papiergraphique« transparent »si Tondesirequelesespa- ces d'une ligne pointillee laissent transparaitre le trace sous-jacent. On obtient un trace transparent en ajoutant a la commandeGRAPHICS PEN le parametre " ,1 " (le para- metre" ,0 " nous ramene au traceopaque). Modifiezdonela ligne 70 :
70 GRAPHICS PAPER 1,1 run ...etvoyez le resultat. Surla positionducurseurgraphique,ilestnonseulement possibledetracerdeslignesou despoints,maisaussi d'inscriredu texte.Cettecaracteristiquepermet depositionner les caracteres avec une bien plus grande precision (a unpixelpres, au lieu de huit) etde les agrementer enjouant sur lesdifferents modes d'encre graphique (voir plus haut). Pourecriresur 1'ecran graphique, positionnezd'abord lecurseura 1'endroit ou devra se trouverlecaracteredegauche, puisentrezla commandeTAG (ou TAG#1 , 2 etc... selon lecanal desire) suiviedescommandes PRINT usuelles. Lecurseur graphiquesedeplace automatiquementde8pixelsversladroiteachaquecaracteretrace. Essayezvous-meme :
160 MOVE 64,108
170 TAG t80 PR!NT"SALLY"
190 TAGOFF run mttSmminSu-•: nnitav -. Bien que les messages emis par le BASIC s'affichent sur la fenetre de texte sans tenir compte de l'indicateur TAG/TAGOFF, il est bon des'habituer a annuler la commande TAG desque Ton n'a plus besoin d'elle. Chapitre9 -Page50 A voshears*d»toisir."##########, "page 417");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_232, r##########"150 GRAPHICS PEN t,0 run : ...nousauronsdenouveau lepapieropaque, tandisque
150 GRAPHICS PEN 8,1 run ...ecrira avec l'encre numero (transparente). Executez maintenant le programme apres avoir efface la ligne 150. Vous remarquez alorsquelestylographiquea retrouve l'encrenumero1 + le modetransparentindiques en ligne 70. Lescaracteres tTansparents II est possible, au moyen de certains codes de controle, d'ecrire surTecran de texteen caracteres transparents. Ajoutezau programme les lignes :
200 PRUT #2,CHR$(22);CHR»(1)
210 LOCATE #2,32,U:PR1NT #2, "»$*»*»"
220 LOCATE #2,32, 14: PRINT #2," "
230 PRINT #2,CHR«(22);CHRS(0) run La ligne 200 etablit le mode transparent sur le canal #2. Vous pouvez constater que le soulignement apparait «pardessus» les asterisques : il est done possible de superposer plusieurscaracteres, memedecouleurdifferente. La ligne 230 annule le mode transpa- rent et le canal #2 revientdone au modeopaque. Avosheuresdeloisir... Chapitre9 Page51"##########, "page 418");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_233, r##########"10 REM modes d'encre XOR
20 MODE 1: INK 2, 10: INK 3,4
30 ORIGIN 440,100,440,640,100.300
40 WINDOW 1,26, 1,25 f
50 CLB 2:GRAPHICS PAPER
60 DRAW 200,200,3
70 MOVE 2,0:FILL 3
80 ORIGIN 440,0,440,640.0,400
90 GRAPHICS PEN , isMASK ,0
100 FOR y=60 TO 318 STEP 2
110 SQSUB 220
120 FRAMEtFRAHE
130 GOSU'd 220 ma niim kmaqemruobom si tMsih OS£ lhapitre9 Pa3qeS2 A. vos,heuresd.e,loi.si.r.
140 NEXT
150 TAB
160 FOR y=60 TO 318 STEP 2
170 HOVE 96,y:PRINT CHR*(224);
180 FRAME: FRAME
190 MOVE 96,y:PR!NT CHR*(224)j
200 NEXT
210 END
220 MOVE 90, y,
230 DRAWR 20,8, ,
240 DRAWR 0,20
250 DRAWR -20,0
260 DRAWR 0,-20
270 RETURN run Inanimation graphique IIestpossible, en modifiantlescouleursaffecteesaux encres,d'obtenir uneffetd'anima- tion. L'eflfetdemouvementestainsicreealorsquelecontenudelamemoired'ecran reste inchange. Unexempleduplicationdeccprocedesetrouved'ailleursdansle programme de« Bienvenue»devotredisque-svstememaitre(vouspouvezassisteracettedemonstra- tion en lancant la commande RUN "disc"). Cependant, Feffet de « palette chgno- tante» utilise danscet exempleest insuffisant lorsque les formes successives constituant l'animation doivent se chevaucher. Le programme qui suit inscrit les chiffres 1 a 4 sur I'ecran a 1'aide d'encrescombineesentre ellcspar i'operateurOR. Le programme deter- mine par balayage le graphismedes caracteres affichesdans le coin infeneur gauchede I'ecran, puis le reproduit sous forme aggrandie. Les nombres successifs sont ecnts en combinant les encres 1, 2, 4 et 8 en mode OR. On a eu ici recours a une sequence de caracteresdecontrole (ligne 50). Leslignes"##########, "pages 419, 420");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_234, r##########"160 etsuivantesfonttournerlapaletteenaccordavecuneformulemathema- t I' i e q n u c e re : l s e e s n fa o i m t b e r n es ex r a e m p i r n od a u n i t ts to e u n r- g a r - a t p o h u i r q c u h e a s c o u n n t e a d l ' o e r n s t a r f e fi e c l h l e e s s u p n ou p r ar de u t n e . rm L i e ne c r ho s i i x el d l e e contient la sequence binaire donnee : le nombre 3, parexemple, est dessine avec Tenure numero4. Pourlefaireapparaitre,il faudradonerendrevisiblestouteslesencresdontle numerocontient 4 en binaire, e'est-a-dire : 4(0100),5(0101),6(0110),7(0111),12(1100),13(1101),14{1UO),15{1111). Dans une application pratique, les encres modifiees a chaque stade de l'animation seraientcalculees a I'avance, si bien qu'on pourrait accelerer la partiedeceprogramme
180 200. comprise entre les lignes a Avosheuresdeloisir... Chapitre9 Page53"##########, "page 420");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_235, r##########"10 REM animation par les couleurs
20 ON BREAK GOSUB 220
30 F m O m R i=l TO IS: INK i ,26:NEXT
40 =l:m(2)=2:m(3)=4!m(4)=B
50 MODE 0:PRINT CHR$ (23) CHR$ (3) TAG ; ', :
60 FOR p= l TO 4
70 GRAPHICS PEN m <p > ,
80 LOCATE #1,1, 25: PRINT 1 1 ,CHR$ (48+p)
90 FOR x=0 TO 7
100 FOR y=0 TO 14 STEP 2
110 IF TEST(>;l4,y!=0 THEN 140
120 MOVE (x+6!*32, <y+6)*16:PRINT CHR$(143);
130 MOVE <x+6)t32,ty+71»14iPRINT CHR$(143)j
140 NEXT y,x,p
150 LOCATE #1,1,25:PRINT#1," ";
160 FOR p=l TO 4
170 FOR i=l TO 25:FRAHE:NEXT
180 FOR i=0 TO 15
190 IF (i AND <n(p))=0 THEN INK i,0 ELSE INK i,26
200 NEXT i,p
210 GOTO 160
220 INK 1,26 run Plans colores L'exempleprecedent nousamontrecommentanimeral'aidedeschangementsdecouieur desgraphiques tracesaveclesencres I, 2et8. On peut,avec les memesencreseten utili- sant lescouleursdifferemment,creeruneffetcompletementdifferentappele« planscolo- res». En voici un exemple :"##########, "page 421");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_236, r##########"10 REM Montagnes
20 DEFINT a-z
30 INK 0,1: INK 1,26 .
40 INK 2,6:1NK 3,6
50 FOR i=4 TO 7: INK i , 9: NEXT
60 FOR i=B TO 15: INK j,20;NEXT
70 MODE 0:DEG:OR1GIN 0, 150:CLG: MOVE 0,150
80 FOR x=16 TO 640 STEP 16 Chapitre9 Page54 Avosfaeuresdeloisir...
90 DRAW x,CQS<x)U50+RNDI100,4
100 NEXT
110 MOVE 0, 0: F ILL 4
120 cx=175:G0SU8 -320
130 cx=525iG0SUB 320
140 SYMBOL 252,0,8 fcC tlF,l(3B,li7F,*FF l )
150 SYMBOL 253,0,6,?<£,«(F2,2,*<F2,StFE
160 SYMBOL 254,0,&60,&70 &7F,*7F,&7F,(<7F " (
170 SYMBOL 255,0, 0,0, kF8, «<EC, &FE, &FF
180 pr*=CHR*(254)+CHR$(255)
190 pl*=CHR*(252)+CHR$(253)
200 TAG:t!=TII1E
210 FOR x=~32 TO 640 STEP 4
220 x2-(<60B-x)*2>MOD 640: hi =RND* 10: hr=50*SIN (x"##########, "pages 421, 422");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_237, r##########"250 IF (T£ST(x2-2, 115+hl-t2> AND 8>=8 THEN 380
260 IF TIME-t!<30 THEN 260
270 FRAMEst!=TIME
280 GRAPHICS PEN 7,1: MOVE x, 100+hr, ,2; PRINT pr$;
290 GRAPHICS PEN 13,1: MOVE x2, 15+hl 2:PR1 NT pi*;"##########, "page 422");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_238, r##########"300 NEXT
310 GOTO 210
320 MOVE ex 100 ,
330 FOR x=0 TO 360 STEP 10
340 DRAW cx+SIN(x)*50+10IRND,100+COS(x)*25+10*RND,l
350 NEXT
360 DRAW cx,100jMOVE cx, 90;F ILL I
370 RETURN
380 ENT -1,1,1,1
390 SOUND 1,25,400, 15,, l, 15
400 FOR y=100+hr TO -132 STEP -2
410 GRAPHICS PEN 7, IsHOVE x,y,,2;PRINT pr*j ji
420 GRAPHICS PEN 8, 1 : MOVE x , y-2, ,3:PRINT pr*}
430 NEXT
440 GOTO 70 run La encore, Ie fonctionnement de ce programme repose sur la forme binaire du numero d'encre. Tous les numeros d'encre comprenant le bit «8» (2 J ) (de 15 a 8) sont sur bleu sombre. Les numeros d'encre contenant le bit «4» (2 2 ) sont sur vert. Lesencres2et 3, contenant chacune le bit «2» (21 ) sont sur rouges, et l'encre 1 (2°), pour finir, est sur blancbrillant, l'encre restant bleue. Avtwheuresdeloisir.. Chapitre9 PageS5"##########, "page 422");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_239, r##########"10 'SALUT L'ARTISTE PAR DAVID RADISIC
20 ' copyright (c) AMSOFT 1985
30 '
40 'N'oubliez pas de *aire RUN "bankman" avant de ,lancer le programme »mmmm»m»j_m_ ti! (»i»tJtim»u«
50 '»i>
60 '
70 ON ERROR GOTO 2740"##########, "page 423");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_240, r##########"100 DIM command*(22)
110 norx*(0) ="NORMAl"snorx*(l)=',XOR H:norx«<2)«"transp"!nor x 12 S 0 (3 R ) E = S " T X QR R EsR " EAD cmnds* ( 1 ) ,ctnnds* (2) :c«ind*=CHR* ( 1 6) +CHR* (&7F )+cfflnds*(l)+cmnds*(2)
130 READ cmno:F0R i = l TO cuincuREAD command* ( i ): NEXT
140 READ st*i IF st*<>"*t" THEN cmnd* (cmnd ) =st$;cmnd=cinnd+l!6 OTO 140
150 WINDOW «B. l,4B,l,3sPAPER *0,0:PEN *8,1:CLS »0
160 WINDOW #1,1,48, 4, 4sPAPER #l,3;PEN #t,l:CLS #1
170 ORIGIN 0,0,0,640,0,334
180 x=320:y=200:MOVE x,y
190 BORDER pn(4);F0R i=0 TO 3s INK i,pnU);NEXT
200 MASK. 255,0;PAPER 0:PEN UPAPER #l,3sPEN #1 , 1 sGRAPH ICS PE H pn,norx ' >••!
210 IF f lag<>5 THEN 280 ..'u Chapiue9 Page56 Avo»twur«sdaloiair.
220 IF pn<2 THEN pnt*=CHR* (248) px= <pn+l »13 ELSE IF pn<4 TH EN pnt*£CHft*(24l)ipx=tpn-l)*13 ELSE pnt*=CHR* (243) px=37
230 LOCATE px,2;PR!NT pntf;
240 LOCATE 1,1;PRIMT USING " pen : fttt pen i s ##";pn<8 )ipn<l)j
250 LOCATE 29,2:PR1NT USING "border ##";pn(4) :
260 LOCATE 1,3:PRINT USING" pen 2 ## pen 3 ##";pn<2> : ; pn (3) i
270 LOCATE px,2:PRINT " ";
280 LOCATE #1,1,1:PRINT •1,USIN6"X :#### Y ;#### "jxjy;: PRINT #1 "MODE TRACE i " ;norx* (narx+ (undraw*2) > ; " ";
290 IF *lag=0 THEN GOSUB 2260
300 '
310 GOSUB 970
320 '
330 IF flag>8 THEN 390
340 IF it="" THEN 390
350 cmnd=INSTR(ciimd*,i*) s IF cmnd= THEN 390
360 IF cmnd=l THEN CLGi x=320: y=200 GOTO 390
370 IF cmnd=2 THEN RUN 70
380 ON cmnd-2 GOSUB 1240,1410,1520,1640,1840,1860,1950,2820, 2090,2120,2170,2200,2668,2660,2660,2660,2390,2330,2200
390 IF tx=0 AND ty=0 THEN 200
400 IF *Ug >8 THEN 440
410 GOSUB 630
420 GOSUB 680:FRAME: GOSUB 688
430 GOTO 200 .,,,..'
448 MOVE tesipx, tempy,pn, 1
450 ON Hag GOSUB 470,490,550,640 HiS
460 GOTO 200
470 PLOT x,y:GOSUB 630.-PLOT x,y
480 RETURN
490 DRAW tempx+x tempyiDRAW tempx+x tenpy+y , ,
500 DRAW tempx,teflipy+y)DRAW tenipx tenipy ,
510 GOSUB 630
520 DRAW tempx+x,te«py:DRAW tempx+x tempy+y ,
530 DRAW tempx tempy+y: DRAW tempx, tempy
540 RETURN ,
550 MOVE tefflpx,tempy: DRAWR x,y
560 IF triside=0 THEN 580
570 DRAW tempxx, teinpyy: DRAW tempx,te«py «U20 : Avosheuresdeloisir... Chapitre9 Page57"##########, "pages 423, 424");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_241, r##########"580 GOSUB 630
590 MOVE tenpx, te*py DRAW te«px+x tempy+y : ,
608 IF triside=0 THEN RETURN
610 DRAW tempxx,tempyy:DRAW teapx tempy ,
620 RETURN
630 x=x+tx:y=y+ty:RETURN
640 MOVE teapx,te<npy:DRAW x^y
650 SOSUB 630
660 MOVE tempx, tempy DRAW x,y :
670 RETURN
680 ' curseur trace & Hfact"##########, "page 425");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_242, r##########"670 IF f lag=5 THEN RETURN
700 MASK 255,1
710 IF flag >1 THEN xx=te*px+x :yy=te«py+y ELSE xx=x:yy*y
720 IF Hag*4 THEN xx=x:yy=y
730 IF f lag=l THEN xx=x:yy=y
740 IF undraw*l THEN 820
750 GOSUB 790
760 MASK 255,0
770 IF H-" "THEN GOSUB 2150;i*=""
780 RETURN
790 MOVE xx-4,yy,pn, DRAW xx+4,yy 1
800 MOVE xx,yy~4:DRAW xx,yy+4
810 MOVE xx,yy,,xorn:RETURN
820 nx-l:GOSUB 1220
833 FRAME:GOSUB 1220
840 IF i*=" " THEN nx=norx : GRAPHICS PEN pn,l:GOSUB 1220
850 i*=""
860 IF fliq <>6 THEN 760
870 IF aoved=0 AND j*<>"" AND j«<CHR«<240) OR j*>CHR$ (247! ( ) THEN ch=ASC(j*) imoved=l
880 IF feaved=0 THEN RETURN
890 LOCATE 5,2
900 FOR i=ch-5 TO ch+5
910 PEN ABS(iOch) +l
920 ch$=CHR$(l)+CHR$(ABSa+256)M0D 256)
930 IF ch"i THEN PRINT " "ch*" ";ELSE PRINT ch$}
940 NEXT
950 PEN PRINT " = "ch" "; 1
960 GOTO 760
970 ty=0:tx=0:GOSUB 680: FRAME:GOSUB 680
980 IF INKEY(0K>-1 OR INKEY<72><>-1 THEN ty=16
990 IF IMKEY(2X>-1 OR INKEY(73K>-1 THEN ty=-16
1000 IF INKEY(8X>-1 OR INKEY(74)<>-1 THEN tx=-16 Chapitre9 Page58 Av«sheuresdeloisir."##########, "page 425");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_243, r##########"1040 j$=INKEY$:i*=UPPER*(j«) „„
1050 IF <i*=" " OR i*=CHR»<13) > AND Haq>0 THEN 1098
1060 IF Uag=5 THEN 1120
1070 IF Hag =6 THEN 1170
1090 ON fiaq GOSUB 1240,1418,1640,1860,1950,2820
1100 i*=""
1110 RETURN"##########, "page 426");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_244, r##########"1150 GRAPHICS PEN pnsPEN II,pn
1160 tx=0:ty=0:BORDER pn <gn) I RETURN
1170 IF tx<0 THEN ch=ABS(ch+255> MOD 256"##########, "page 426");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_245, r##########"1160 IF ty<0 THEN ch=A8S (ch+246) MOD 256
1190 IF tx>0 THEN ch=(ch+l) MOD 256
1200 IF tv>0 THEN ch=(ch+10) MOD 256
1210 tx=0:ty=0;RETURN „-,,«'
1220 TAGsMOVE xx-8, yy+6,pn, nx'.PRINT CHRt tch> j :TAGQFF
1230 RETURN
1240 ' C
1250 IF Hao=l THEN 1290
1260 ro=t:GQSU8 2240
1270 tempx=x: terapy=y: +1 aqa l
1280 RETURN
1290 IF tefnpx=x AND teapy-y THEN 139ni0t tix=MAX<x!tempx)-MIN<tempx,x):tiy*NAXCy,tMpy>-MIN<te«p 1310 y,yi
1320 ti=SQR<(tixT2)+<tiyt2))
1330 ORIGIN tempx.tempy
1340 PLOT 0,0,pn,0:MOVE 0,-ti
1350 FOR s=0 TO PII2+0.01 STEP PI/tti/2»
1360 DRAW SlN(z+Pl>*ti,COS(z+PI)tti,pn,norx
1370 NEXT 2
1380 ORIGIN 0,0
1390 x=terapx:y= ! te«pY:teflipx=0:tei*py=8MUg=B
1400 RETURN Avos hemes de loiar... Chapitre9 Page59"##########, "page 426");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_246, r##########"1440 tempx+x: te(tpy=y:+"lag=2 .?
1450 x=0:y=0
1460 RETURN j -.*:
1470 IF norx= l THEN 1500 -.-Si"##########, "page 427");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_247, r##########"1500 x=tempx: y=tewpy lag=0 : f
1510 RETURN
1520 ' F ycil
1530 ro=3:G0SUB 2240 :*.*}
1540 60SUB 1620; IF i**" " THEN 1680 r-tl
1550 edgecol=VAUl«) .->.-. tt
1560 ro=4:Q0SUB 2240 ;.^1{
1570 GQSUB 1620: IF i*=" " THEN 1600 ':;/
1580 filler=VAL(i$! , f
1590 MOVE x,y,edgecol:FILL filler
1600 flag=0:i*="" ; i
1610 RETURN -i
1620 i*=INKEY$!lF li$<"0"OR i$>"3") AND i*<>" " THEN 1620 '.1
1630 RETURN }VI"##########, "page 427");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_248, r##########"1690 RETURN n:j
1700 IF triside<>0 THEN 1770 ; S1
1710 ro=6;G0SUB 2240 3U
1720 MOVE 0,0:pn,l:GOSUB 590 :.C1 :
1738 tempxx=tempx+x: te«pyy=tempy+yjx=x/2: y=20 . n,y
1740 triside=l
1750 GQSUB 550:GOSUB 590 . ; I
1760 RETURN -tl
1770 IF narx= l THEN 1800 .".r.'
1780 MOVE te(spxx,tempyy, ,norx:DRAW teipx,tenpy ,rd
1790 DRAW tempx+x,terapy+y: DRAW tetnpxx tempyy '11
1800 tempxx=0!teipyy=0 , ?{
1810 x=tefltpx y-tempy: triside=0 '''? :
1820 teinpx=0: te«py=0; f 1 ag*0 : Chapitre9 Page60 Avosheuresdeloisir..
1830 RETURN
1848 ' 3
1850 norx«liundraw"uridran XOR ltRETURN
1860 ' L
1870 IF flaqM THEN 1910
1880 ro=7:GOSUB 2240"##########, "pages 427, 428");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_249, r##########"1900 RETURN
1910 IF norx=l THEN 1930
1920 MOVE tempx, terapy, ,narx;DRAW x,y
1930 x=tempx;y=tedipy: Hag=0
1940 RETURN
1950 ' I
1960 IF flag=5 THEN f Ug=0sCLSi INK 3, tupcol i INK pn, col : GOTO 1990
1970 CLS:Haq=5:B0RDER pn(pn)
1980 RETURN
1990 FOR i=0 TO 3:INK i , pn (i ): NEXT i BORDER pn(4)"##########, "page 428");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_250, r##########"2010 CLSjRETURN 2B20 ' A
2030 IF flatpi THEN 2870"##########, "page 428");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_251, r##########"2050 undraw=l:Hag=6i narx= l : •oved»l
2060 RETURN
2070 flsg=0
2080 RETURN
2090 ' N
2100 norx=0
2110 RETURN
2120 ' E
2130 GRAPHICS PEN pn, 0: TAG: MOVE xx-8, yy+6, , 0: PRINT " "pTAGQ FF
2140 RETURN
2150 '<ESPACE>
2160 PLOT x,y,pn,norxsRETURN
2178 ' X
2180 norx=l
2190 RETURN
2280 ' M -.* ¥."##########, "page 428");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_252, r##########"2218 menu=menu MOD 2+1
2220 GQSUB 2260:RETURN •* '
2230 i*=UPPER*(INKEY*):IF i$="" OR INSTR (ser*, i t) =0 THEN 223 ELSE RETURN Avosheuresdeloisir... Chapitre9 Page61
2240 CLS:undraw=0:PRINT c«nd$ (ra> ;; LOCATE 1,3iPRINT "(espace > ";:IF ro=3 OR ro=4 THEN PRINT "Pour Sortir"
2250 RETURN
2260 CLSifUg=-l
2270 FOR i =l TO LEN(c«tnds* (*enu)
2280 ps=i+ABS(menu-2!*LEN(cdtnds*(l) )
2290 PEN 1 s PRINT "< "MI D* (cmnds* (menu) , i , 1> >"MID« (comnand* (p s),2,4>" ";
2300 NEXT
2310 PRINT "<CLR> <DEL> <EBPA>"
2320 RETURN
2330 ' S
2340 GOSUB 2460: IF Hlenaae$="n TKEN 2370
2350 GOSUB 2558
2360 SAVE filename*, b,J<C000,M000
2370 GOSUB 2268
2380 RETURN
2390 ' R
2400 GOSUB 2460! IF f i lenant*""" THEN 2440
2410 GOSUB 2730
2420 LOAD filename$,«<C80B
2430 GOSUB 2570
2440 GOSUB 2260
2450 RETURN
2460 CLS: LOCATE 10,3: PRINT "<RETURN> pour annuler!";
2470 LOCATE 1,1:PRINT " NQM du DOSSIER " : 5
2480 INPUT "".filename*! IF filenaae*="" THEN RETURN
2490 n=INSTR(Hlena«e*,"."):IF n= THEN 2520
2500 IF n=l THEN 2460
2510 Hlename*=LEFT*(HI enamel, n-1) 2520 f ilenaffle*=LEFT$(-filename$,8!+".scn"
2530 CLS fcj* i"
2540 RETURN
2550 FOR 1=0 TO 4:POKE feC000+i ,pn<i :NEXT )
2560 RETURN
2570 FOR i = TO 4:pn(i)=PEEK <&C000+i) HOD 27:NEXT
2580 cn=0:FOR i=0 TO 2: IF pnU )=pn (i+1) THEN cn=cn+l
2590 NEXT: IF cn=3 THEN 2630
2600 FOR i=0 TO 3: INK i ,pn <i > : NEXT
2610 BORDER pn (4) pn= GRAPH ICS PEN pn"##########, "pages 428, 429");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_253, r##########"2620 RETURN
2630 pn(0>=0:pn(l)=26:pn(2)=15:pn(3)=6spn<4)M
2640 GOTO 2608
2650 ' 1. 2, 3, % 4 Chapitre9 Page62 Avosheuresdeloisir..
2660 CLS:PRINT "Desirez vous <S>tocker": PRINT TAB (16) "<R>t trouver";PRINT TA6U3)"ou <E>changer l'ecran?"
2670 ser$="SRE"+CHR«(13):GQSUB 2230; IF i*=CHR»(13) THEN 2260
2680 bnk2=(cmnd-13) bnk1=1 :
2690 IF i*="S" THEN CLS:60SUB 2550: SCREENCOPY, bnk.2,bnk 1
2700 IF i*="R" THEN SQSU8 2730: i SCREENCOPY, bnk 1 ,bnk2 : GQSUB 2 570
2710 IF i$="E" THEN CLSsGQSUB 2730sGOSUB 25501 SCREENSWAP, bn k2,bnkl:B0SUB 2570
2720 GQSUB 2260:RETURN
2730 FOR i=0 TO 3s INK i , GiNEXT:BORDER 0.-RETURN
2740 CLS:GOSUB 2600:R£SUKE 2260
2750 DATA "CBFT3LIANEXM" , " 1234RSM""##########, "pages 429, 430");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_254, r##########"10 'BUSTOUT by ALEXANDER MARTIN
20 'copyright (c) AMSOFT 1984
38 '
40 MODE 1:BQRDER Js INK 0,1:1NK 1,26: INK 2,24; INK 3,6
58 SPEED KEY 15,2
60 ENV 1,1,18,0, 11,0,10
70 ENT 1,10,2,2 .
80 ENV 3,1,0,16,5,-3,2
90 ENV 2,5,3,3,1,-21,22,9,-3,2
100 ENT -2, 10,2,2,5,-7, 1,2, 11,3,2,.-4,B
110 '
120 '
130 MOVE 30,32:DRAWR 0,400, MOVE 610,32:DRAWR 0,400,1"##########, "page 469");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_255, r##########"140 PEN 38LOCATE 3, UPRINT STR ING*<36, 143!
150 PEN 2:L0CATE 3, 2: PRINT STRING* (36, 1 43)
160 PEN isFQR r=5 TO 6;L0CATE 3,r:PRINT STRING*(36, 1431 :NEXT r
170 bx=9
180 bals=5;scare=0
198 PEN IsGQSUB 680:CLEAR INPUT
200 IF 1NKEY*<>CHR*<32) AND JOY(0)<16 THEN 208
210 LOCATE 4,23:PR1NT SPACE* (35) LOCATE 1,24:PRINT SPACE»(48 >i
220 GOSUB 690SGOSUB 660:GDTO 280
230 '
240 '
250 LOCATE bx,24sPR!NT " " STRING* (4, 1 31 ) j " "iRETURN
260 '
270 '"##########, "page 469");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_256, r##########"260 xa=l ya-1 IF INT(RND*2)=1 THEN xa=-xa : :
290 PEN IsGQSUB 250
300 ORIGIN 0,400 •>«« '-'
310 x=bx+4:y=il:xt=x:yl=y
320 ' ''''• -
330 '
340 xl=x+xaiy l=y+ya Etpourquelquesprogrammesdeplus... Annexe3 Page 1"##########, "page 469");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_257, r##########"370 IF yl=24 AND xl>bx+l AND xl<bx+6 THEN ya*-yat y l=y1-2:SOU ND 130,44,8,7,l,l:a=<(x>bx<-5>OR(x<bx+2)):IF a=-l THEN xa=xat a: x l =x l+xa:y 1=y1+
380 IF y 1 =25 THEN LOCATE x,y.PRINT " " I GOTO 580
390 GOSUB 250
400 =TESTU16*xl)~l,-<16*yl)-l) t
410 IF t<>0 THEN ya=-ya:xz=xl:yz=ylsyl=yl+yasGOSUB 590: IF t*"##########, "page 470");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_258, r##########"420 IF t=3 THEN score=score+20:GOSU8 660
430 IF t=l THEN 5Core=score+5;GOSUB 660
440 IF yl=l THEN ya*l
450 LOCATE x,y:PRINT H "sLOCATE xl,yl:PRINT CHR$ (233) !X*xtiy =yl
460 IF y=l OR x=3 OR x=3B THEN SOUND 129,78,8,7,1,1
470 GOTO 340
480 '
490 '
500 bals=bals-l:SOUND 132, 19,46, 12,2, 2: IF bals=0 THEN 628
510 GQSUB 660: GOTO 280
520 '
530 '
540 IF <INKEY(B)=8 OR INKEY(74)*0) AND bx>2 THEN bx=bx~2:RET URN
550 IF (INKEY(l)=a OR INKEY(75>=0) AND bx<32 THEN bx=bx+2:RE TURN
560 RETURN
570 '
580 '
590 LOCATE xz,yz: PRINT ' "-.RETURN
600 '
610 *
620 IF score>hiscore THEN hi5core»score
630 GOSUB 660:score=0:vies=5;GQTQ 130
640 »
650 '
660 SOUND 130,1,21,13,3,0,311 LOCATE l,25i PRINT TAB(4)"HISCO RE "jhiscorej
670 LOCATE 1B,25: PRINT "SCORE*" ; score:LOCATE 30,25:PRINT "BA LLES ";bals;RETURN
680 LOCATE 4,23: PRINT "Appuyez sur <ESPACE> pour co«aencer"l RETURN
690 LOCATE 25: PRINT SPACE* (40) ;: RETURN i , Annexe3 Page-.2 Elpourquelqueaprogrammesdeplus..."##########, "page 470");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_259, r##########"10 ' BOMBARDIER
20 ' copyright (c)AMSOFT 19B4
38 '
40 MODE 1 : CLSs INK 0,0:BORDER 0: INK 1 , 18: IKK 2,6: INK 3,4sINK 5,!5:INK 6,2: INK 7,24: INK 8,8: INK 9,26l INK 10, 13: INK 11,28:1 NK 12, 12: INK 13, 16: INK 14, 14: INK 15,21
50 SYM S B Y O M L BOL 242 A , F & T 0 E , R 8<3 2 2 4 , 0 & : 7 B A Y , M & B F D E L ,!< 2 F 4 A 1 , , f J c < F 4 2 0, ,S &6 E 0 0 , ,S 8<70, lc7F, J.7F , &3F, *7,*0 : ! !
60 score=B:hiscore=0:avion$=CHR*(241)+CHR$<242>:x=2:y=2:chut e=0:a=2:b=2
70 BOSUB 480
80 CLS
90 PEN 2:L0CATE 1,15: INPUT "Niveau : 8 <as) a 5 (debutant):" ,niv
100 IF niv<8 OR niv>5 THEN 60T0 90
110 niv=niv+10
120 LOCATE 1 , 15: PRINT CHRt tl8! ; :L0CATE 1, IS: INPUT "Vitesse :"##########, "page 471");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_260, r##########"130 IF vit>108 OR vit<0 GOTO 120
140 '
150 ' Intseubles
160 '
170 MODE 0:FOR base=5 TO 15:F0R haut=21 TO INT (RNDU 18+niv) > STEP -ULOCATE base,haut PEN base~2iPRINT CHR* 143) +CHR*(8) : ( +CHR*(11)+CH R*(244) fi NEXT:NEXT
188 PLOT 0,20,4:DRAW 640,28,4
190 LOCATE 1,25:PEN 2: PRINTHcore" score: :LQCATE 13,25:PRINT ; "HI";hiscore;
200 '
218 ' JEU PRINCIPAL
220 '
230 LOCATE x-1 y:PRINT" "; ,
240 PEN 1 i LOCATE x,y:PRINT avionl;:PEN 2
250 IF y=21 AND x=15 THEN GOTO 298 ELSE GOTO 340
260 '
270 ' atterissage reussi
288 '
290 FOR c=0 TO 1088:NEXT
300 score=score+100-(niv$2):niv=niv-l:x=t2! y*2: a*2:br2:chutee Etpourquelquesprogrammesdeplus... Annexe3 Page 3
310 IF niv<10 THEN ni v=10:vi t=vi t-20
320 IF vit<0 THEN vit=
338 SOTO 150
340 FOR c=0 TO vit:NEXT
350 x=x+l
360 IF x=18 THEN LOCATE x-l,ysPRINT CHR* < 18) ; : x=2:y=y+l LOCft TE x,y:PEN PRINT avion*;:PEN 2 I
370 a*=INKEY$: IF a*=" " AND chute=0 THEN chute»l:b"y*2:«"K
380 IF y=21 THEN chute=0
390 IF chute=l THEN LOCATE a,b:PRINT CHR*(252);: LOCATE a,b-l PRINT" ";:b=b+l:IF b>21 THEN LOCATE a,b:PRINT" "j {LOCATE a, : 5-1 PRINT" " : ;:a=0:b*0:chute=0: SOUND 3,4000,10,12,0,0,10 ga=U~0.5M32:gb-400~(bm):bomb=TEST(ga,<}b)
410 IF bamb>0 THEN GOTO 670
420 gx=((x+1.5)*32):gy=408-(y*16) icrash=TEST <gx gy) ,
430 IF crash>0 THEN GOTO 570
440 GOTO 230 450
460 mode d'emploi 470
480 LOCATE 1,2;PEN IsPRINT" Vous pilotez un avion au-dessus d'une vills abandonnee que vous devez 'raser' pour atterrir et faire le plein de -fuel.Votre avion se deplace de gauche a droit e. ":PRINT
490 PRINTsPRINT" Une fois le bord droit atteint-,1 'avion revi ent a gauche UNE LIGNE PLUS BAS. Vous disposez d'une quan tite il 1 iini ee de bombes et vous pouvez les larguer surles immeubles en appuyant sur la BARRE ESPACE.";:PRINT
500 PRINTsPRINT" A chaque fois que vous atterrissez,soitla v itesse de votre avion soit la hauteur des immeubles au , gmente. "; ;PR INT: PRINT; PRINT" VOUS NE POUVEZ LARGUER DE BOMBE TANT QUE Ml" LA PRECEDANTE N'A PAS EXPLOSEE ... ,
510 PEN 2:L0CATE 24;PR INTi PRINT"APPUYEZ SUR UNE TOUCHE PDU"##########, "pages 471, 472");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_261, r##########"520 a$=INK£Y$: IF a*="" THEN GOTO 520
530 RETURN
540 '
550 ' collision
560 '
570 LOCATE x-1 ys PRINT CHR* (32) +CHR* (32>+CHR» (32! +CHR* (253) + CHR$(8)+CHR*(23,8)+CHR*(8);
580 FOR t=l TO 10 ;S0UND 7, 4000, 5, 15, 0,0, 5: PEN tsPRINT CHR*( 243)+CHR$(8)+CHR*(238)+CHR$(8)+CHR$(32)+CHR*(8! ;: NEXT i PEN 2
590 CLS:LOCATE 1 , 5: PRINT"SCORE: "score: Axmexe3 Page 4 Etpourquelquesprogrammesdeplus.
600 IF score>hi5core THEN hiscore=scoresLOCATE i,8:PRINT"NEI LLEUR SCORE !"j"##########, "pages 472, 473");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_262, r##########"640 '
650 ' Immeubles Detruits 660 '
670 LOCATE a,h-l:PRINT" "+CHR*(BI ; ; PEN 4:F0R tr«l TO INTCRND <Ut3>+l:score=score+5:S0UND 3,4000, 10, 12,0,0, IBsLOCATE a,b! FOR t*0 TO 4 PRINT CHR*(253S+CHR*(B)+CHR*(32)+CHR*(8>$iNEXT:b=b+l i 6S0 IF b=24 THEN b*b-l
690 NEXT
700 LOCATE 6,25:PRINT score; :chute=0:a=x :b=y:60T0 230 Etpourquelquesprogrammesdeplus... Annexe3 Page 5"##########, "page 473");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_263, r##########"10 'PING PONG by DAVID RADISIC
20 ' copyright (c> AMSQFT 1985 30
40 DEFINT a-z
50 comp=l
60 ENV 1,=11,20, =9,5000 7a MODE IjINK 0,10:BORDER 1 s 1NK 1 , 26 INK 2, 18s INK 3,0
80 GOSUB 710
90 GOSUB 150
100 GOSUB 330
110 GOSUB 420
120 LOCATE 13,1: PRINT USING "###* "jscorelj
130 LOCATE 35, SPRINT USING' #### ";score2j 1
140 GOTO 100
150 PEN 2
160 x(l)«3:y(l)»S -
170 x<2)=37:y(2)=22
180 cote*=CHR*(233!:cote2*=STRING*(2,207)
190 LOCATE 1,3:PRINT STRINGS (39, cote*) PRINT STR INGI (39, cot e$)
200 FOR i=l TO 19
210 PRINT cate2*;TAB(38>;cote2»
220 NEXT
230 PRINT STRING* (39,cote*) sPR INT STRING* (39,cote*)
240 WINDOW #1,3,37,5,23
250 CLS #1
260 SYMBOL 240,0,60,126,126,126,126,60,0 k
270 raquette**" "+CHR$(8I +CHR*(10)+" ! !"##########, "page 474");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_264, r##########"290 balle*=CHR*(24B)
300 PEN 3
310 LOCATE 2,l!PRINT"jaueur 1 0";:LOCATE 24, 1 PRINT"j oue ur 2 :
320 RETURN
330 n=IH'." [RND*2)!CLS #l:scored=0
340 PEN 3
350 FOR i=l TO 2:L0CATE x(i),y(i):PRINT raquette*; ;NEXT
360 ON n GOTO 390
370 xb«21idK»l
380 GOTO 400
390 xb=19:dx=l
400 yb=12:dy=INT (RND»3)-1 9+ Annexe3 Page 6 Btpourquelque*programmesdeplus..
410 RETURN
420 BOSUB 680
430 oxb=xb;oyb=yb
440 SOSUB 500
453 IF note>0 THEN BOUND 129, note, 50, 15,
460 LOCATE axb,oyb: PRINT" ";
470 LOCATE xb,yb:PRINT balle*
480 IF scored=0 THEN 420
490 RETURN
500 LOCATE xb+dx,yb+dyiCh*=COPYCHR*(*0)
510 note=0
520 IF ch$=" " THEN xb=xb+dx : yb=yb+dy: RETURN
530 IF ch$="i" THEN dx=2-dx-2:dy=INT<RND*3>-1 ; note=200:RETUR N
540 IF ch*=LEFT*(cote2*,l> THEN 570
550 IF ch*=cate$ THEN dy=2-dy-2:note=250
560 RETURN
570 IF dx>0 THEN score l=score 1 +1 ELSE scare2=scare2+l
580 scored=i;note=2000
590 RETURN pm
600 = (INKEY(69)>=0) + (INKEY<72)>=0)+ABSUINKEY(71)>=0) + (I NKEY<73>>=0) )*2
610 IF comp=l THEN p (2)=ABS (y(2><yb ) *2+<y(2) >yb ) GOTO 630
620 p(2)=(INKEY(4)>=0)+(INKEY(48)>=0)+ABS((lNKEY(5)>=0)+(INK EY(49>>=0!)*2
630 PEN 3
640 FOR i=l TO 2"##########, "pages 474, 475");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_265, r##########"670 LOCATE x (i ),yli ):PRINT raquette*;
680 NEXT
698 PEN 1
700 RETURN
710 PEN 2:PRINTiPRINT TAB 15) "Ping^-Pong" PRINT TAB(15)" ( :
720 PEN 3i PR INT: PRI NT TAB(14)"Pour utiliser les raquettes :"
730 PRINT:PRINT;PEN 1
740 PRINT" Joueur 1 Joueur 2 direction"!PRINT
750 PRINT" A 6 HAUT"
760 PRINT" Z 3 BAS":PRINT
770 PEN 3: PRINTjPRINT TAB(l3)"Ou les Joysticks"
780 PRINTsPRINTjPRINTjPRINT
790 PEN 2
800 PRINT TAB(6)" Choixs <1> ou <2> Joueurs"
810 i$=INKEY$iIF $<>"!" AND i*<>"2" THEN 810 i
820 IF i*="t" THEN comp=l ELBE comp=0
830 MODE IsRETURN Etpourquelquesprogrammesdeplus... Annexe3 Page?"##########, "page 475");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_266, r##########"10 "ESCRIME ELECTRIBUE par ALEXANDER MARTIN
20 'copyright <c> AMSOFT 1985
30 '
40 DEFINT a-z
50 MODE
60 GQSUB 980
70 BOSIJB 1370
80 GOSUB 270
90 GOSUB 1520
100 GOSUB 1370
110 GOSUB 1270
120 ' '
130 '
140 REM debut
150 IF fini THEN GOTO 100
160 GOSUB 240
170 FRAME: IF jldir THEN GOSUB 570 ELSE FRAME:FRAME
180 FRAME: IF j 2d ir THEN GOSUB 620 ELSE FRAME:FRAME
190 IF jlsa=-l THEN GOSUB 670
200 IF j2sa=-t THEN GOSUB 720
210 GOTO 140
220 '
230 '
240 IF j THEN 380 ELSE 480
250 '
260 '
270 CLSiPEN 6
280 PRINT: PRINT" Choix des commandes"
290 PRINT;PRINT;PRINT:PRINT" <J>ou<C> puis ENTER"
300 LOCATE 6, 10: PRINT"JOYSTICK" PRINT!PRINT" ou CLAVIER" :
318 LOCATE 15, 10: IF j THEN PRINT"*"ELSE PRINT" "
320 LOCATE 15,12: IF j THEN PRINT" "ELSE PRINT"*"
330 IF NOT <INKEY<45)> THEN j=-l
340 IF NOT (INKEY(62)( THEN j=0
350 IF NOT (INKEYUB)) THEN RETURN ELSE 310
360 '
370 '
380 jl=JOY(0) :j2=J0Y(l)
390 jldir=(jl AND l)l-l+(jl AND 2)10.5
400 j2dir=(j2 AND D*-l+(j2 AND 2)10.5
410 IF jl AND 16 THEN j lsa=j isa-1 : IF jlsa*~l THEN AFTER 15 G QSUB 770 ** Annexe3 Page 8 Etpourquelquesprogrammesdeplus."##########, "page 476");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_267, r##########"428 IF j2 AND 16 THEN j2sa=j2sa-l : IF j2sa»-l THEN AFTER 15 9 0SU8 770
430 IF jlsa THEN jldir=0
440 IF j2sa THEN j2dir=0
458 RETURN
460 '
470 '
480 j2dir=UINKEY(4)=0)*l)+((INKEY(5)=0H-ll
490 jldir={UNKeY(691=0m) + <(INKEY<7t)=0)t-l)
500 IF INKEY(63)=0 THEN j lsa=j lsa-1 i IF jlsa=-l THEN AFTER 15 GOSUB 770
510 IF INKEY (10)=0 THEN j 2sa=j2sa-l : IF j2sa=-l THEN AFTER 1"##########, "page 477");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_268, r##########"520 IF j lsa THEN jldir=0
530 IF j2sa THEN j2dir=0
540 RETURN
550 '
560 '
570 pt= lwp+jldir: IF pt>25 OR pt<6 THEN RETURN ELSE jlwp=pt j
580 tdir=0 j
590 PEN 1: LOCATE 3,jlwp;CLS #3tPRINT CHR* (209) ;: RETURN
600 ;
610 '
620 pt=j2wp+j2dir: IF pt>25 OR pt<6 THEN RETURN ELSE j2wp=pt
630 j2dir=0
640 PEN 2:L0CATE 18,j2wp: CLB #5:PRINT CHR* (21 1 );: RETURN
650 '
660 '
670 PAPER #4, 4; WINDOW #4, 4, 17,j lwp, j 1 wp: CLS #4: FRAME: FRAME 68-0 PAPER #4,0:CLS #4
690 GOTO 570
780 '"##########, "page 477");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_269, r##########"710 '
720 PAPER #6, 5:W NDDW «6, 4, 17, j 2wp, j 2np; CLS #6:FRAME: FRAME
738 PAPER #6,0:CLS #6
740 GOTO 620
750 '
768 '
778 jwpe=(j lwp=j2wp) : IF jlsa AND NOT (j2sa) AND jwpe THEN jl sc=jlsc+USQUND 132, 120, 10, 0, 1 , 0: PR INT # 1 , a* (j t sc ) ; : IF jlsc="##########, "page 477");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_270, r##########"780 IF j2sa AND NOT (jlsa) AND jwpe THEN j2sc=j 2sc+l ; SOUND 1 32,100,10,0,l,0:PRiNT #2, a*(j2sc) IF j2sc=9 THEN 860 ; :
790 IF'jlsa THEN SOUND 132,40,70,0,1,1
800 I? j2sa THEN SOUND 132,56,70,0,1,1
810 jlsa=0
820 j2sa=0 Etpourquelquesprogrammesdeplus... Annexe3 Page 9
830 RETURN -" ; --":*. V M -.=:=;. :<-<* --r-'t- -
840 *
850 '
860 PEN 6
870 LOCATE 6, 10: PRINT"GAME OVER"
880 IF jlsc=9 THEN INK 1 , 2, 20! INK 2,0 ELSE INK 2,6, 17s INK 1,
890 SOUND 129, 1000,0, 12,3:S0UND 130,900,0,12,3
900 WHILE INKEY*<>""iN£ND
910 t f=TIME: WHILE t +2000>TIME: WEND
920 WHILE INK£Y$="":WEND
930 CLS
940 fini=-l
950 RETURN
960 '
970 '
980 a$<0>="lltl0110n01111"
990 a$U)="00100100i001001" ':'
1000 a*(2)="111081111100111" a f
1010 a*(3)="11100Ult0011U"
1020 a*(4}="10010010111100l"
1030 a$(5)=»nn00iii00inr
1040 a$(6»="11110011110llll"
1058 a*(7)="ill00100l0100i0»
1060 a*(8)="111101111101111"
1070 a*(9)="lill01111001001"
1080 FDR n=0 TO 9"##########, "pages 477, 478");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_271, r##########"1070 taille*LEN(a$(n))
1108 FOR n2=l TO taille
1110 IF MID*(a4(n) ) n2,l!=,ll B THEN MID* (a* (n) ,n2, 1) *CHR* ( 143) ELSE KID$(a$(n),n2,l!*CHR*(32)
1120 NEXT n2,n
1130 '
1140 »
1150 b*="ESCRIHE ELECTRIQUE"
1160 c*=CHR*(32>+CHR*U64)+" Alexander Martin"
1170 ENV 1,=9,2000:ENT -1,6,3,1
1180 ENV 2,127,0,0,127,0,0,127,0,0,127,0,0,127,0,0
1190 ENV 3, =9, 9000
1200 '
1210 '
1220 BORDER
1230 INK 0,12:PEN #4, tsPEN #6,2tPEN tl,lsPEN #2,2;PAPER #1,3 iPAPER #2,3:PEN #0,6
1240 RETURN 'FIN DE DEFINITION DES CONSTANTES
1250 '
1260 '
1270 INK 8,12!lNK 1,2: INK 2,6:INK 3,13:INK 4,20sINK 5,1: INK Annexe3 Page 10 Etpourquelquesprogrammesdeplus."##########, "page 478");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_272, r##########"1288 WINDOW #3,3,3,6,25: WINDOW 15,18,18,6,25
1290 WINDOW #1,3,5, l,5iWINDQW #2, 16, 18, 1 , 5:WINDOW #7,1,28,1, 5:PAPER #7,3
1380 CLS:CLS #7: PRINT #1 , a* (0) ; PRINT #2,a!(B> j :j lsc=fl:j2sc= 0:j lwp=Ssj2wp=24sjldir=t:j2dir=l"##########, "page 479");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_273, r##########"1310 GOSUB 570;GOSUB 620
1320 SOUND 1,1000,0, 12, 2:S0UND 2,900,8,12,2
1330 jlsa=0:j2sa=0:Uni=8
1348 RETURN 'FIN DU RENOUVELEMENT DE L'AFFICHAGE DU SCORE
1350 *
1360 '
1378 CIS
1380 PEN 7
1390 FOR n=l TO LEN<b$)
1400 LOCATE Itn.lB
1410 FOR n2=LEN(b*> TO n STEP -1
1420 PRINT MlD*(b$,n2,U
1430 LOCATE l+n,10
1440 SOUND 135, 20»n2,5, 12,2,1
1450 NEXT n2.n
1460 SOUND 135,108,0, 13,3,1,20
1470 PEN 6:PRINT:PRINT:PRINTsPRINT c*
1480 FOR n=t TO 5000:N£XT
1490 RETURN
1500 '
1510 '
1520 IF j THEN RETURN
1530 CLS
1540 LOCATE 1,5
1550 PRINT" controle du jeu"
1560 PRINT
1570 PRINT"Joueur 1 Joueur 2"
1580 PRINT
1590 PRINT" A HAUT 6"
1600 PRINT" BAS 3" Z
1610 PRINT" X TIR 7"
1620 t'=TIME:WHILE t ! +1000>TIME: WEND
1630 RETURN Etpourquelquesprogrammesdeplus. . Annexe3 Page 1"##########, "page 479");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_274, r##########"10 ' database 20
30 ' Dfl 12/12/B4
40 ' Initialisation
50 CLEAR
60 SYMBOL AFTER
70 DEF FNdeek (x)=PEEK(x)+256*PEEK<!<+l> ' Deek
80 SPEED WRITE 1
90 recnum = 50 ' Nombre d' enregistremertts
100 ht =12 ' Nombre d'en-tetes
110 beep* = CHR*<7)
120 ' Definition du cods double hauteur
130 RESTORE 3520
140 double*=STRING*<81,32) double*="" :
158 FOR j=0 TO 80
160 READ double*=double*+CHR*(j) j :
170 NEXT IB0 double = FNdeek(3double*+l)
190 definition de 6 champs/enreg."##########, "page 480");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_275, r##########"240 'Le libelle des champs est ds le ler enreg.
250 RESTORE 3620 lit les en-tetes des champs
260 READ Heldlt(B) ,Held2*(0; Held3$(0!
270 READ Held4*(0l , Held5*(0; f i el d6S (0)"##########, "page 480");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_276, r##########"260 Ecran '
290 MODE liBOROER 13
300 INK fl, 13: INK 1,0s INK 2,3r INK 3,26
310 PAPER 0:PEN 1
320 SOSUB 2990 11 lustration
330 GOSUB 370 ' Ecran d' introduction
340 GOSUB 560 ' Menu
350 END
360 ' SOUS-PROGRAMME: Ecran d'intro
370 title*="Database Amstrad"
380 GOSUB 2820
390 sp*=STRING$(40,32)
400 LOCATE 1,6
410 PRINT CHR$!24);sp«; .nnexe3 Page 12 Elpourquelquesprogrammesdep!i
420 PRINT " Ce programme vans permet cP enre-gistrer " ; 5p*
430 PRINT " jusqu'a 50 noms, avec au plus 6 champs ";sp$j
440 PRINT " par enregistrement Le programme a ete ":sp*; .
450 PRINT " fait pour etre utilise comme un ";sp$;
460 PRINT " repertoire, mais il peut etre modifie ";sp»;
470 PRINT " pour servir de liste de disques, "isp«;
480 PRINT " de lisle de programmes ou de '5 sP* 5
490 PRINT " dictionnaire geoqraphique par example. ";sp«;
500 PRINT CHR*(24)
510 MQVE 4,316 DRAW 634,314,0 :
520 DRAW 634,50 DRAW 4,50 :DRAW 4,316 :
530 6DSUB 2750 ' Appuyez sur ESPACE
540 RETURN
550 ' SOUS-PROGRAMME: Menu
560 title*="Database Amstrad": GOSUB 2820 ' en-tete
570 MOVE 10,38 DRAW 10,330,1 DRAW 630,330 : :
580 DRAW 630,38 DRAW 10,38 ;
590 MQVE 14,42 : DRAW 14,326,1 J PRAW 626,326
600 DRAW 626,42 DRAW 14,42 :
610 WINDOW 3,39,6,25
620 PRINT:PRINT "Ajouter un enreg. a database ft"
630 PRINTsPRINT "Enlever un enreg. a database. ".E"
640 PRINTtPRINT "Retrouver un enreg. particulier .R" . . .
650 PRINTsPRINT "Chancier le nam des champs C"
660 PRINTiPRINT "Imprimer les enreg. de database....!"
670 PRINTsPRINT "Charger un fichier L"
680 PRINTiPRINT "Trier les enreg. par ordre alphab..T"
690 PRINTsPRINT "Sauvegarder un Uchier S"
700 WINDOW 1,40,1,25
710 WHILE G*>"" G*=INK£Y* WEND : :
720 WHILE G*="" G*=INKEY$ WEND : s
730 option = INSTR("AETCftLSr',UPPER*(G$M
740 ON option GOSUB 810,970,1170,2400,1550,1850,2060,2260
750 ' OPTIONS : A E T C R L S I
760 IF option THEN 560 ' commands a e-f-fectuer
770 PRINT beep*; SOTO 710 :
780 '
790 ' SOUS-PROGRAMME: Ajoute un enreg.
800 '
840 title$="Ajout d'un enrg" ! 60SUB 2828 ' en-tete"##########, "pages 480, 481");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_277, r##########"820 IF nextblank > recnum THEN LOCATE 14,8 t PRINT "Database plein" GOTO 2750 :
830 PRINT"C'est 'enreg. numero" snextblank 1
840 PRINT
850 PRINT fieldl$(0) ; TAEMht) ;; INPUT " : ", f ieldl* (nextbl ank) Etpourquelqwsprogrammesdeplus.. Annexe3 Page 1
860 PRINT field2*(0);TAB(hti;:INPUT ": ",field2*(nexthlank)
870 PRINT field3*(0))TA6(ht>; INPUT " ",field3f(nextblank)
880 PRINT field4*<0>;TA8(ht) INPUT " " field4$(nextblank) f
890 PRINT field5« (0) ; TAB (ht) INPUT " ",field5*(nextblank)
900 PRINT field6*<0) jTAB(ht) INPUT " ",f ield6$(nextblank!
910 nextblank=nextblank+l ;
920 SQSUB 2750
930 RETURN
940 '
950 ' SOUS-PROGRAMME: Efface un enreg.
960 t
970 title*="Effacer un enrq." : GOSUB 2820 ' en-tete
980 INPUT "Effacer quel enrq." dl ;
990 PRINT
1000 IF dl >= nextblank THEN PRINT "Get enreg. n'existe pas" GOTO 1120 :
1010 d»dl : st*0 ; GOSUB 2640 ' affiche 1'enreg.
1020 INPUT "0 pour confirmer 1 ' ef acagej " , q$
1030 PRINT
1040 IF UPPERS (q$)<>"0" THEN PRINT TABU2)i"Pas d'effacaqe" GOTO 1120 :
1050 FOR i*d+ TO nextblank l
1060 fieldl*(i-l)=fieldl*U):field2*U-l)afield2$ (t)
1070 field3*(i-i)=field3*(i)!field4*(i-l ! =field4$ (i)"##########, "pages 481, 482");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_278, r##########"1060 field5*<i-l>=fieid5$U>ifield6$(i-l)=field6*(i)
1090 NEXT
1100 nextblank-nextbl ank-1
1110 PRINT TAB ( 18) i "Effacage"
1120 GOSUB 2750 ' espace
1130 RETURN
1140 t
1150 ' S0US-PR06RAME: tri"##########, "page 482");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_279, r##########"1170 title*="Tri enreg." ; SOSUB 2820 ' en-tete
1180 PRINT PRINT TAB(81;"Tri des enrq. par ";fieldl$<Bt s
1190 ' Shell/Metzner No. 1 Classement des fieldl* (1 a numXl
1200 ' debute avec nuffl"/.=na(ribre d'enreg. ! utilise 1'integral
1210 timethen = TIME ' chronometre le classement
1220 numX=nextblank-l
1230 slX=nuiftX H
1240 WHILE si 7.
1250 six=sr/. / 2
1260 2X>nuaX-fll
1270 f 1 ag '/. = 1
1280 WHILE HagX=l
1290 flag7.=
1300 FOR s37.=l TO s2X Annexe3 Page 1 Et pourquelquesprogrammesdeplus..."##########, "page 482");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_280, r##########"1338 NEXT s37.
1340 WEND
1350 WEND
1360 tiroenow = TIME - timethen
1370 timenow = INT(tiflienow/30) / 10 ' en seconde a la dizai ne pres.
1380 LOCATE 6,12 : PRINT "Classement en" ; timenow; "secondes
1390 60SU8 2750 ' espace
1400 RETURN ' au menu principal
1410 '
1420 ' SOUS-PROGRAMME: Permutation
1430 '"##########, "page 483");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_281, r##########"1460 5l*»=field3*(s3X):field3*(s3X)=Ueld3*(s4X):Held3*(54XJ - si*
1470 sl$=field4*(s3X):Held4*(s3X)=field4*(s4X)!*ield4*(s4X> - si*"##########, "page 483");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_282, r##########"1450 sl4=Held5*(537,):Held5*(s3X)=Held5*(s4X):field5*(s47.) = si*
1490 sl*=-field6*(s37.) :*ield6* (s3X)=field6*(s4X) :field6*(s4/.J = si*
1500 *laqX=l
1510 RETURN ' au sous-prg de Classement
1520 '
1530 ' SGUS-PROSRAMME: Recherche
1540 '
1550 title*="Trouver un enrg." s SOSUB 2820 ' en-tete
1560 LOCATE 3,6
1578 PRINT"pour taut lire appuyez sur ENTER"
1580 LOCATE 1,8
1590 INPUT "Lettres a rechercher :",qu*
1600 qu*=UPFER*<qu*l
1610 FOR se=l TO nextblank-1
1620 q* = qu*
1630 IF INSTR( UPPER*(fieldl* (se) q* ) THEN GOSUB 1750
1640 IF INSTR( UPPER*(Field2*<se> ) q$ ) THEN GOSUB 1750
1650 IF INSTRt UPPER*(*ield3S(se) q* ) THEN GOSUB 1750
1660 IF INSTR( UPPER* (f t eld4* (set q* ) THEN G0SU8 1750
1670 IF I.NSTR( UPPERStfield5*(se> > q* ) THEN 80SUB 1750
1680 IF INSTRi UPPER*<*ield6* !se> q$ ) THEN GOSUB 1750
1690 NEXT Etpourquelquesprogrammesdeplus. . Annexe3 Page IS
1700 CLS : GOSUB 2750 » espacE
1710 RETURN ' au menu principal
1720 r
1730 ' SOUS-PROGRAMME: Affiche l'enreg. trouve
1740 9
1750 CLS
1760 PRINT PRINT :
1770 d=se : st=0
1780 GOSUB 2640 ' Affiche l'enreg.
1790 GQSUB 2750 * espace
1800 q$=CHR${0! ' pour eviter flTecrire plusieurs fois le mem e it<?m
1810 RETURN
1820 I
1830 ' SOUS-PROGRAMME : Chargement fichier
1840 I
1850 title$="Charger un fichier": GOSUB 2820 ' en-tete I860 PRINT1'Mettez la disquette contenant le "
1870 PRINT1'fichier dans le lecteur puis"
1880 PRINT''entrez le nodi du fichier" 1.890 PRINT;iINPUT "Nom: ",fS
1900 PRINT;iOPENIN f*
1910 INPUT #9,nextblank
1920 FOR i==0 TO nexttslank-1
1930 INPUT #9,fieldl$<i)
1940 INPUT #9,fieid2*U)
1950 INPUT «9,fiflld3*(i)
1960 INPUT #9,field4*(i)
1970 INPUT #9,+ield5*(i>
1980 INPUT #9,f ie!d6*(i )
1990 NEXT i
2000 CLOSEIN '
2010 GOSUB 2750 ' espace
2020 RETURN 1
2030 J
2040 ' SOUSi-PRQGRAMME: Sauvegarder 1 fichier
2050 J
2060 title*i="Sauvegarde des enreq.": GOSUB 2820 ' en-tete
2070 PRINT"Placez une disquette dans le lecteur"
2080 PRINT"Entrez le now du fichier :"
2090 PRINT! INPUT "NoitiJ ",f$
2100 PRINT:OPENOUT f$ '
2110 PRINT #9,nextblank
2120 FOR =9 TO nextblank-i i
2130 PRINT »9,fieidU(i)
2140 PRINT #9,+ield2*(i) Page 16 Qpows-quthjawrpcogrammesdeplus..
2150 PRINT *9 ield3$(i) f
2160 PRINT #9,tfield4*(i ) " .
2170 PRINT #9,Heid5»(il
2180 PRINT #9,*ield6*(i>
2190 NEXT . •
2200 CLOSEOUl
2210 GOSUB 2750 ' espaee
2220 RETURN
2238 '
2240 ' SOUS-PROGRAMME: Impression
2250 '
2260 tjtle*="lmprimer les enreg." ! GOSUB 2820 ' en-tete
2278 INPUT "Ave: vous uoe imprimante connectee <0/N) ", q$
2280 IK UPPERS (qllO'O" THEN CIS 5t=0 ELSE St=8 :
2290 PRINT #st, "DATABASE AMSTRAD"
2300 PRINT #5t PRINT #st ;
2310 FOR d=0 TO nextblank-1
2320 GOSUB 2640 ' aHiche enreg.
2330 NEXT
2340 PRINT #st PRINT #st :
2350 GOSUB 2750 ; espaee
2360 RETURN
2370 '
2380 ' SOUS-PROGRAMME: Nom champs
2390 '
2400 tiUe$ ="Nom des champs" : 8QSU6 2828 ' en-tete
2410 PRINT : PRINI "en-tete 1: "; f ieldi$ (0!
2420 INPUT "Change en: !' ,iii
2430 IF +i $.>"" THLN f eld1*10) =*iS
2440 PRINT : PRINT "en-tete 2: " ; +i el <i?t 'G ;
2450 INPUT "Change en : " ii
2460 II- *\$> ('HEN field2$(0)H i*
2470 PRINT : PRINT "en-tete 3: " i + ieldSt < 0!"##########, "pages 483, 484, 485");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_283, r##########"2460 INPUT "Change em", (it
2490 IF fi*>"" THEN (ieid31 (0 -i 1
2500 PRINI : PRINT "en-lete 4: " iield4*(01
2510 INPUT "Change en:",M$
2520 IF fi*>"" THEN Hel d4* (0) -U*
2530 PRINT : PRINT "en-tete 5: " ; Held5$ t0) .2540 INPUT "Change en:", fit H
2550 IF ti$>"" THEN eld5* (0) =fi " :-
2560 PRINT : PRINT "en-tete 6: " tield6* (0)
2570 INPUT "Change en:",fi* H
2580 IF fi*>"" THEN eld6*(0) =fi
2590 GOSUB 2750 ' espaee E' pourquelquesprogrammesdeplus... Annexe3 Page 17
2600 RETURN . •
2610 >
2620 ' SOUS-PROGRAMME! AHiche L'enreg. en cours 2630
2640 PRINT •st.-L'enreg.* fd! "est: PRINT Kst
2650 PRINT *st,fieldl*(0i jTAB(ht) eldl*(d)
2660 PRINT *5t,<ield2*<0] jTAB(ht) field2*(d)
2670 PRINT #st,field3$(0; ;TA8(ht) eld3$(d)
2680 PRINT #st,Held4$<8! ;TAB(ht> Held4$(d)
2690 PRINT #st,field5$(0! jTAB(ht) eld5*(dl
2700 PRINT #st,Held6«(0) :TAB<ht) field6*(d)
2710 PRINT #st RETURN : 2720
2730 SOUS-PROGRAMME: Espace 2740
2750 LOCATE 10,25
2760 PRINT "ESPACE pour cant inuer'
2770 WHILE INKEY$<>" " WEND ;
2780 RETURN 2790 2B00 SOUS-PROGRAMME: AHichag* du titre 2810
2820 tl=LEN(title*)+2
2830 tt=INT((40-tl)/2)
2840 CLS
2850 PAPER 1 : PEN 3
2860 LOCATE tt,l PRINT STRINBKtl, 32)
2870 LOCATE tt , 2 : : PRINT" ";title*;' n
2880 LOCATE tt,3 PRINT STRINE*(tl, 32) :
2890 PAPER PEN : 1
2900 PRINT ' pour eHacer une ligne a la -fin
2910 tx=tt*l6-l2 : tx t= < tt+tl ) »16-2*
2920 MOVE tx,396 DRAW txl, 396,0
2930 DRAW txl,354! DRAW tx,354 :
2940 DRAW tx, 396
2950 RETURN 2960
2970 SOUS-PROGRAMME: Image ecran 2980
2990 CLG 0:ZX=16
3008 RESTORE 3460
3010 YX=250:FOR XX=4( TO 10 STEP -ZX*2
3020 YX=YV.~ZX
3030 READ N$
3040 GQSUB 3310 Annexe3 Page 18 Etpourquelcruesprogrammesdeplus.
3050 NEXT
3060 ORIGIN 0,0,8,639,0,3??
3070 FOR XX=630 TO 616 STEP -IsMQVE XX,33B:DRAW XX, XX-296, 2: NEXT
3080 FOR XX=0 TO 16 STEP 2:MQVE XX, 40: DRAW XX, 140+XX/l, 5,2: 1 NEXT
3090 ORIGIN 0.0,0,234,30,140 CLG 2 !
3100 ORIGIN 0,0,0,639,0,399
3110 YX=30;FOR X7, =234 TO 630 STEP 2
3120 MOVE XX, YX; DRAW XX, YX+ 110,2
3130 YX*YX+1 NEXT :
3140 MOVE 16,152:DRfiW 0,140,3
3150 DRAW 0,30:DRAW 234,30
3160 DRAW 630,226:DRAW 630,340
3170 DRAW 616, 340; MOVE 630,338
3180 DRAW 2-34, 40: DRAW 0,140 i
3190 MOVE 234, 140: DRAW 234,30
3200 LOCATE 2,2 : XS="Database" : G0SU8 3400 ' Double
3210 LOCATE 2,5 : X*=" Amstrad" ; BOSUB 3400 ' Double
3220 LOCATE 27,20 : X*="Appuyez sur" : GOSUB 3400 ' Double
3230 LOCATE 27,23 : X*="une touche" ; GOSUB 3400 ' Double
3240 WHILE INKEY*="" WEND :
3250 RETURN
3260 '
3270 ' SOUS-PROSRAMME: Dessine une carte »n couleur 3
3280 : ligne couleur cantient 1
3290 ' re-ference N$,N (4 chiHres)
3300 '
3310 ORIGIN 0,0,XX,XX+20B,YX,YX+140
3320 CLG liMOVE XX,YX:DRAWR 8, 140, 1 ) DRAWR 214,0:DftAWR 0,-148 :DRA«R -214,0
3330 MOVE XX+6,YX+4:DRAWR 202, 0, 0: DRAWR 0, 132: DRAWR -202,0:D RAWR 0,-132:MQVER 8,112:DRAWR 202,0
3340 MOVER -194, 16
3350 TAG:PRINT N*;:TAGOFF
3360 RETURN
3370 '
3380 ' SOUS-PRQGAMME: AHiche X* en double hauteur
3390 '
3400 FOR i*l TO LEN(xJ)
3410 POKE double+i,ASC<MID$(X*,i,m
3428 CALL double NEXT RETURN : :
3430 '
3440 ' Norns pour 'image ecran 1
3450 ' -
3460 DATA "Wi 11 arcs: 7471 ", "Stevens:6216" , "Soames :5807", "Smi t h :2201" 4M Etpourquelquesprogrammesdeplus... Annexe3 Page 19"##########, "pages 485, 486, 487");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_284, r##########"3470 DATA "Nesbit 2207" "Marks 2022", "Upton 1091", "Gree : , s j :8087" n
3480 DATA "Frinton: 1011 ", "Evans 2877", "Barker :8123","Alto : n :998l"
3490 DATA " DATABASE"
3500 '
3510 ' Code machine pour le texte double hauteur
3520 DATA k3E,&6E,&CD,&A5 t&B,*16,icFE,I(3E,&19 l
3530 DATA leCD, &5A, &BB,&7A,(.CD, fcSA,fcBB, &0E, &04,*7E
3540 DATA &CD,&5A,fcBB,*CD,fc5A,&B8,*23, MD, !<20, &F5
3550 DATA M4, fc20,&E7,&3E, &FE, &CD, !<SA,&BB,fc3E,&0A
3560 DATA *CD,&5A,4BB, &3E, &08, fcCD,4SA,*tBB,St3E fcFF
3570 DATA &CD &5A, &BB &3E fcBB,«eC0 &5A, &BB, &C9, f &5A , , , ,
3580 DATA 4BB,fc3E, &0A, &CD, &5A, 4BB, &3E, *<08, fcCD, &5A
3590 DATA fcBB, &3E, fcFF,&CD, &5A, &BB,i3E,&0B, &CD,&5A
3600 DATA i6B,&C9
3610 ' Norn de champs par cfe+aut
3620 DATA Nom, Rue, Vi 1 1 e, Pays, Code Postal , Tel .No ftB *• » i Etpourquelquesprogrammesdeplus. Annexe3 Page 20 ."##########, "page 488");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_285, r##########"10 'ARSENE LUPIN by DAVID RADISIC
20 'copyright (c) AM30FT 1985
30 '
40 MODE BilNK 0,0jBORDER 8: INK 1,26: INK 2, 15: INK 3,25
58 INK 4,14: INK 5,24: INK. 6,B:INK 7,0:INK 8,0:PAPER #1,7
70 DIM objx(5,20),objy(5,20),ge(nx(5,20),Qe(ny{5,^)
80 GQSUS 380
90 GOSUB 720
100 pause=200:GOSU8 340
110 IF gems=0 THEN 6GSUB 970
120 PEN' 4
130 FOR 1=10 TO 12
140 LOCATE 15,i:PRINT"SUnN";
150 NEXT
160 PAPER 8sCL8 #2:PAPER 8
170 G0SU8 1170
180 GQSU8 1230
190 GOSUB 1370
200 GOSUB 1510
210 IF rm=0 THEN GOSUB 1980
220 IF mort=0 THEN 160
230 pause=t00:GOSUB 340
240 PAPER 0:CLS:PEN 1
250 LOCATE 5, 3: PRINT"VQULEZ-VQUS" ;
260 LOCATE 7,5;PR1NT"REJ0UER""##########, "page 489");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_286, r##########"290 IF i*="N" THEN MODE 2:PEN 1 STOP
300 RUN
310 IF chien=l THEN RETURN
320 chien=l:cruenx=minx (r«) chieny=miny (riti)
330 RETURN
340 FOR loop=l TO pause
350 FRAME
360 NEXT
370 RETURN ^ a
380 rm=l;xp=6:yp=4:homme*-CHR$(224)ichien»8lvol»0
390 SYMBOL 240,8,8,8,8,8,3,8,8
400 SYMBOL 241,0,0,0,0,255,0,0,0
410 SYMBOL 242,0,0,0,0,15,8,8,8
420 SYMBOL 243,0,0,0,0,248,8,8,8
430 SYMBOL 244,8,8,8,8,248,0,0,0 Etpourquelquesprogrammesdeplus.. Annexe3 Page 21
440 5YMB0L 245,8,8,8,8,15,0,0,0
450 SYMBOL 246,8,12,13,14,12,12,8,8
460 SYMBOL 247,8,12,12,14,13,12,8,8
470 SYMBOL 248,8,24,88,56,24,24,3,8"##########, "pages 489, 490");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_287, r##########"460 SYMBOL 249,8,24,24,56,88,24,8,8
490 SYMBOL 250,0,0,255,253,255,255,255,0 .
500 SYMBOL 251,28,20,20,20,20,20,20,28
510 SYMBOL 252,0,0,255,255,255,255,255,0
520 SYMBOL 253,28,28,26,28,28,28,28,28
530 SYMBOL 255,195,165,60,126,90,60,36,24
540 ENT 1,12,-4,1
550 ENT -2, =1000,60, =3000,40
560 ENV 1,10,1,5,2,-4,1,2,-1,20
570 Tenet*(l}=STRING*(2,250>!Tenet*<2)=CHR*(25l)+CHR*(8)+CHR *(251>+CHR*<8}+CHR*(i0)+CHR*(251!
580 porte*(l)=STRING*(2,252)!porte*(2)=CHR*(253>+CHR*(8)+CHR *(10)+CHR*(253)+CHR*(8)+CHR*(10)+CHR*(253)
590 inter! U,0)=CHR* (246); inter* (1,1)=CHR$(247)
600 inter*(2,0)=CHR*(248):inter*(2,l)=CHR*(249!
610 qeffl*=CHR*(144) obj*=CHR*(233) :chien*=CHR*<255) ;
620 coup*=CHR*(246)+CHR*(248)+CHR*(247)+CHR*(249)+CHR*<252>+ CHR*(253)+CHR*(250)+CHR*(251)+qeffl*+obj*+chien*
630 RESTORE 3010
640 FOR i=l TO 5
650 READ minx (i ,fliiny(i ,maxx (i ,maxy (i
660 READ dirU, ) 1) ,dir(i ! ,2) ,dir (i ! ,3) ,dir (i,4)
670 NEXT
680 WINDOW #1 ,minx (rn)-l ,aaxx <ra)+l,ifny(n)-l,Mxy<i )+l
690 WINDOW #2,1,14,1,25
700 CLS #1 PAPER #8,8
710 RETURN
720 ORIGIN 50,50
730 INK 6,24,12
740 RESTORE 3060
750 GOSU8 1280 i
760 LOCATE 2,20 — m f 7 7 7 8 0 0 P P E E N N 5 l : : P P R R I I N N T T " "e—ch B a;ppatoires"; . \x '
790 PEN 5:PRENT" "|
800 LOCATE 8, 2: PR I NT ENTREE"
810 pause=380:GOSUB 340
820 CLSiLOCATE 1,3: INK 6,0
830 PEN 1 ! PR I NT homme*}" A. LUPIN (VOUS) ": PRINT
840 PEN 2:PRINT LEFTS (porte* ( 1 ) , 1) 5 LEFT* (parte* (2) , 1) B Portes":PRINT
850 PEN 3: PRINT inter* ( 1 0) 5 inter* (2, 0) ; "i nterrupteur eteint tl
860 PEN 3iPRINT inter* (1, 1! 5 irtter*<2, 1); "interrupteur allume ":PRINT
870 PEN 4:PRINT LEFT* (lenet* ( 1 ) , 1 ) ;LEFT* (* enet* (2! , 1) \ " Fenetres":PRINT Annexe3 Page 22 Etpourquelquesprogrammesdeplus...
880 PEN SiPRINT gem$?" Bijoux"t PRINT
898 PAPER PEN BiPRINT objtj" Obstacles"! PEN liPAPE"##########, "pages 490, 491");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_288, r##########"900 PEN PRINT chien*:" Lb chien" 1
910 PEN 5:PRINT:PRINT
920 PRINT"Utilisez le Joystick":PRINT" Ou les touches"sPRI NT" flechees"
930 feint=REMAIN(l)
940 AFTER delais*4,l 60SUB 340
950 RETURN
960 '
970 'GENERATEUR DE RUB IS/OBSTACLES
980 '
990 FOR piece=l TO 5 1000 gemr=lNT(RND*8)+2:objr=INT(RNDI10H5
1010 minx=minx (piece! : miny=miny(piece) taaxx»fiaxx (piece) max y =maxy (pi ece)
1020 FOR i=l TO gemr
1030 x =INT(RND*(*naxx-fflinx+l>)+minx
1040 y=INT (RND* (maxy-miny+l) )+miny
1050 gefflx (piece, i > =x gemy (piece, i)=y
1060 ge<ns=gems+l
1070 NEXT i
1080 FOR i = i TO obj
1090 x=INT(RNDI (maxx-minx+1) )+fflinx
1100 y=INT(RND*(maxy-miny+l) )*«niny
1110 objx (piece, i >=x:abjy (piece, i)=y
1120 NEXT i
1130 gems(piece)=ge«r:obj (piece)=objr
1140 NEXT piece
1150 CLB
1160 RETURN
1170 ON rm GOTO 1180,1190,1200,1210,1220
1180 RESTORE 2670:RETURN
1190 RESTORE 2740sRETURN
1200 RESTORE 2810: RETURN
1210 RESTORE 2B80:RETURN
1220 RESTORE 2960:RETURN
1230 PAPER 0;READ rm*;PAPER 8 -
1240 WINDOW # 1 , minx (riti)-l f maxx (r*)+l,«iny (rm) -1 ,maxy (riI+i!C LS #1
1250 PEN liLOCATE 1 , 1 j PRINT SPACE* < 19)
1260 LOCATE PRINT"Piece »"jr«*|
1270 IF lumie 1 r , e 1 ( : rm) THEN INK 7,10:INK 8,10 ELSE INK 7,liINK
1280 READ a$:IF aS="END" THEN RETURN
1290 IF a$="D" THEN 2180
1300 IF a*="W" THEN 2260
1310 IF a$="L" THEN GRAPHICS PEN Is GOTO 2340
1320 IF a*="S" THEN 2420
1330 IF a*="F" THEN GRAPHICS PEN 6i60T0 2340 Etpourquelquesprogrammesdeplus .. Annexe3 Page 23
1340 PRINT '**» ERREUR ***' 1
1358 STOP 1360
1370 ' AFF1CHAGE Bl JOUX/OBJETS
1388 J
1398 PEN 6
1400 FOR i*l TO obj(rm)
1410 LOCATE objx (rm, i ) , objy (rut, i
1420 PRINT obj*5
1430 NEXT
1440 PEN 5
1450 FOR i=J TO gems (rm>
1460 LOCATE gemx (rm, i ) ,ge*y(m, i
1470 PRINT gem*;
1480 NEXT
1490 PEN IsLQCATE xp,yp:PRINT ho»i*«$:
1500 RETURN
1518 x+=B:yf=0:PEN !
1528 IF INKEY(0)<>~1 OR INKEY(72K>-1 THEN yf=-l 1
1530 IF INKEY(2K>-1 QR INKEY<73)<>-1 THEN y*= l
1540 IF INKEY(B)<)-1 OR INKEY(74K>-t THEN xf=-l i
1550 IF INKEYUIO-1 OR INKEY(75)<>-1 THEN x+=l
1560 IF x*= AND y-f =0 THEN 1630
1570 LOCATE xp+xf ,yp+y+ :ht$=COPYCHR$(#0!
1580 IF ASC(ht$)>239 AND ASC(ht*)<246 THEN 1510
1590 IF htt<>" " THEN 1660
1600 LOCATE xp,yp:PRINT" "j
1610 PAPER 0;LOCATE 15,5;PRINT" B j« PAPER 8
1620 xp=xp+xf:yp ayp+yf
1630 LOCATE xp,yp:PRINT hommet:
1640 IF chien>0 THEN chien=chien MOD 2+l:IF chiart*2 THEN 255
1650 GOTO 1510
1660 coup-INSTR(coup*,ht*):char=ASC(MID$(coup«,coup,l))
1670 ON coup GOTO 1690,1690,1690,1690,1750,1750,1850,1900,19 70,2090,2650
1688 BOTO 1600
1690 IF coup)2 AND coup<5 THEN ch*r=char-i
1700 IF coup<3 THEN char*char+t
1710 PEN 3:L0CATE xp+xf yp+y+ s PR INT CHRKchar);
1720 lunuere(rm) =lumiere( , rfl) *XOR 1
1730 IF lumiere(rm) THEN INK 7, 103 INK 8,10 ELSE INK 7,BsINK 8,0
1740 GOTO 1510
1758 IF xf<>0 AND yf<>0 THEN 1638
1760 IF xf<0 THEN dir=4 ELSE IF xf>0 THEN dir=3
1770 IF yf<0 THEN dir=l ELSE IF yf>0 THEN dir=2
1780 IF dir(rm,dir)=-l THEN 1630 ELSE rra=dir (rm,dir)
1790 IF chien>0 THEN GOSUB 310
1800 IF dir=1 THEN xp=6: yp=maxy(rm) Page 24 Etpourquelquesprogrammesdeplus..."##########, "pages 491, 492");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_289, r##########"1840 RETURN
1850 IF xp>5 AND xp<8 THEN 18B0 I860 IF xp<6 THEN dir=4 ELSE dir=3
1870 GOTO 1780
1880 IF yp>13 THEN dir=2 ELSE dir=l
1890 GOTO 1780
1900 PAPER 0; CIS: PEN 1
1910 LOCATE 3,3:PRINT"vou5 vous en tlrez f ; „"##########, "page 493");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_290, r##########"1940 PRINT USING " ##" vol
1950 PEN 5-.L0CATE 8,9:PRINT J "Bijoux"; I960 mort=i:RETURN
1970 LOCATE xp,yp:PRINT " "; xp=xp+x*:yp=yp+y<
1980 i=0
1990 i=i+l"##########, "page 493");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_291, r##########"2020 IF i=gems(rm> THEN 2050
2030 gemx (rm, i ) =gemx (rm, gems (rm) )
2040 gemy(rfl),i)=o,etny(r(n,geins(r(n))
2050 qems(rm)=Qem5(rm)-l! vol=vol+l ",*„,
2060 MOVE 400,150+(vol*2),l,l:DRAW 560, 150+ vol*2) , 1 . , «
2070 SOUND 129,248,10,12,0,1 20B0 GOTO I960
2090 bruit=INT (RNDt 15)
2100 SOUND 1,3000, 10,bruit,0,0, 10
2110 PAPER 0:LOCATE 15,5;PR1NT"CRACK !" j : PAPER 8
2120 IF bruit<10 OR delais=50 THEN 1630
2130 delais=delais-50
2140 feint=REHftIN(i)
2150 AFTER delais*4,l G0SU8 318
2160 GOTO 1630
2170 '
2180 ' dessin des portes
2190 '
2200 READ no,dr»
2210 IF dr»="V" THEN dr=2 ELSE dr*l
2220 PEN 2 • Etpourquelquesprogrammesdeplus.. Annexe3 Page 25
2230 pici=porte$(dr):GOSUB 2508
2240 GOTO 1280 I
2250 ' -3
2260 ' dessin des fenetres i
2270 ' A
2280 READ no.wi* 1
2290 IF wi$="V" THEN wi=2 ELSE wi =
2300 PEN 4 l
2310 pic*=-fenet»(wi);GOSUB 2S00
2320 BOTO 1280
2330 '
2340 ' tire les traits
2350 '
2360 READ xl,yl,x2,y2
2370 MOVE x!,yl,,0
2380 DRAW x l,y2, ,0: DRAW x2,y2,,B
2390 DRAW x2,y£, ,8: DRAW xl,yt,,8
2400 SOTO I28B
2410 '
2420 ' dessine les interrupteurs
2430 '
2440 READ no,swt
2450 IF sw*="L" THEN sw*l ELSE sw=2
2460 PEN 3
2470 pic*=intert(sn,0) [GOSUB 2500
2480 BOTO 1280
2490 '
2500 ' affichc le caracter«
2510 '
2520 READ x,y:L0CATE x,y:PRINT picf;
2530 no=no~l;IF no>0 THEN 2520
2540 RETURN
2550 PEN 1:L0CATE chi enx,chieny: PRINT" ";
2560 hom*e*=CHR*(225)
2570 IF (chienx=xp AND chieny=yp) OR (chi *nx»xp+xf AND chien y=yp+yf) THEN 2650
2580 IF chienx<xp THEN chienx=chienx+
2590 IF chienx>xp THEN chienx=chienx~l
2600 IF chieny<yp THEN chi eny=chieny+l
2610 IF chieny>yp THEN chi eny=chi eny-1
2620 LOCATE chienx chi eny: PR INT chien*;
2630 SOUND i,0,RNDt,40, 10,1,2,31
2640 GOTO 1510
2650 PRINT-'SNAP' "; famexe3 Page 26 Etpourquelquesprogrammesdeplus.."##########, "pages 493, 494");
basic_test!(test_amstrad_cpc6128_manuel_de_lutilisateur_1985_amsoft_fr_text_listing_292, r##########"2660 *ort=l:RETURN
2670 DATA ENTREE
2680 DATA 1,64,308,226,4
2690 DATA 0, 2,H,6,3,6,22
2700 DATA D,2,V,4,12,9,ll
2710 DATA S,1,L,4,U
2720 DATA S,1,R,9,14
2730 DATA END
2740 DATA SALON
2750 DATA 1,2,308,258,4
2760 DATA 0,1,V, 10,12
2770 DATA W, t,H,6,3
2780 DATA 14, I, V,2, 12 U
2790 DATA S,2,R, 10, , 10, 15 2B00 DATA END
2810 DATA SALLE A MANGER
2820 DATA L, 2, 308, 258,
2830 DATA W,1,V,18,12
2840 DATA W,1,H,6,3
2850 DATA D, 1 ,V, 2, 12
2860 DATA S, 2,L,2, 11 ,2, 15
2870 DATA END
2880 DATA CUISINE 2B90 DATA L, 2,276,384,4
2908 DATA D, 2,H,6, 5, 6,22
2910 DATA W,1,H, 10,22
2920 DATA W,1,V,14,13
2930 DATA D,1,V,2,13
2940 DATA S,1,L,2,16
2950 DATA END
2960 DATA LINGERIE
2970 DATA L, 2, 276, 256,
2980 DATA D,1,V,10,12
2990 DATA S,l,R,10,lt
3000 DATA END
3010 DATA 5,4,8,21,0,4,3,2
3020 DATA 3,4,9,21,-1,-1,1,-1
3030 DATA 3,4,9,21,-1,-1,-1,1
3040 DATA 3,6,13,21,1,0,-1,5
3050 DATA 3,6,9,21,-1,-1,4,-1
3060 DATA L, 64,308,480, 100
3070 DATA F, 250, 98, 294, 102
3080 DATA F, 250,306, 294,310
3090 DATA F, 390, 94, 430, 106 Annexe3 Page 27 Etpourquelquesprogrammesdeplus-.-
3100 DATA F, 390,302, 430, 314
3110 DATA F, 474, 240,488,270
3120 DATA F, 474,124,488,154
3130 DATA F, 58, 240, 72, 270
3140 DATA L, 226, 308, 322, 180
3150 DATA L, 160,180,480,100
3160 DATA 1,64,180,160,100
3170 DATA END Si cesjeux vous ont plu, vous pouvez vousjoindre au Club des utilisateurs AMSTRAD. Entre autres privileges et avantages, vous recevrez gratuite- AMSTRAD ment le mensuel quipublicprogrammes dejeu, utilitaires,fonc- tions speciales, bulletins departicipation a des concours et des informations de derniere minute. i 1 1 •3* Annexe3 Page 28 Etpourqueiquesprogrammesdeplus..."##########, "pages 495, 496");
