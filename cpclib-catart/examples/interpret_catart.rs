use cpclib_catart::basic_command::{BasicCommand, BasicCommandList};
use cpclib_catart::char_command::CharCommand;
use cpclib_catart::entry::{Catalog, SerialCatalogBuilder};
use cpclib_catart::interpret::Interpreter;

fn main() {
    // List of (catart name, BASIC code as &str)
    let catarts = vec![
        (
            "FORCE1.BAS",
            r#"
10 MODE 1
20 BORDER 0
30 INK 0,0
40 INK 1,24
50 INK 2,15
60 INK 3,20
70 LOCATE 10,10
80 PEN 3
90 PRINT "Ceci est une demonstration"
100 PEN 2
110 LOCATE 10,12
120 PRINT "du mode PEN force"
130 PEN 1
140 LOCATE 10,14
150 PRINT "de la version 1.1 de"
160 PEN 3
170 LOCATE 10,16
180 PRINT "CATaclysme !!!"
        "#
        ),
        (
            "MODE0.BAS",
            r#"
10 BORDER 0
20 INK 0,0
30 MODE 0
40 INK 1,1
50 PAPER 1
60 INK 2,2
70 PEN 1
80 INK 3,5
90 INK 4,11
100 INK 5,14
110 INK 6,25
120 LOCATE 1,10
130 PRINT CHR$(18)
140 PRINT CHR$(20)
150 LOCATE 1,11
160 PAPER 2
170 PRINT CHR$(18)
180 PRINT CHR$(20)
190 LOCATE 1,12
200 PAPER 3
210 PRINT CHR$(18)
220 PRINT CHR$(20)
230 LOCATE 1,13
240 PAPER 4
250 PRINT CHR$(18)
260 PRINT CHR$(20)
261 LOCATE 1,14
270 PAPER 5
280 PRINT CHR$(18)
290 PRINT CHR$(20)
291 LOCATE 1,15
300 PAPER 4
310 PRINT CHR$(18)
320 PRINT CHR$(20)
330 LOCATE 1,16
340 PAPER 3
350 PRINT CHR$(18)
360 PRINT CHR$(20)
370 LOCATE 1,17
380 PAPER 2
390 PRINT CHR$(18)
400 PRINT CHR$(20)
410 LOCATE 1,18
420 PAPER 1
430 PRINT CHR$(18)
431 PAPER 0
440 PRINT CHR$(20)
450 PAPER 5
460 LOCATE 6,14
470 PRINT "G.P.A"
480 LOCATE 12,14
490 PEN 6
500 PRINT "2004"
510 PAPER 1
520 PAPER 0
530 LOCATE 4,22
540 PRINT "Type : RUN'GPA"
550 LOCATE 1,24
560 PEN 0
570 PAPER 0
        "#
        ),
        (
            "MODE1.BAS",
            r#"
100 MODE 1
110 BORDER 0
120 INK 0,0
130 INK 1,0
140 PAPER 1
150 INK 3,26
160 PEN 2
170 :' G
180 WINDOW 4,13,8,9
190 CLS
200 WINDOW 4,6,10,17
210 CLS
220 WINDOW 6,13,16,17
230 CLS
240 WINDOW 11,13,12,15
250 CLS
260 WINDOW 9,13,12,13
270 CLS
280 :' P
290 WINDOW 16,18,8,17
300 CLS
310 WINDOW 18,25,8,9
320 CLS
330 WINDOW 23,25,10,14
340 CLS
350 WINDOW 18,23,13,14
360 CLS
370 :' A
380 WINDOW 28,30,8,17
390 CLS
400 WINDOW 30,37,8,9
410 CLS
420 WINDOW 35,37,10,17
430 CLS
440 WINDOW 30,36,12,13
450 CLS
460 WINDOW 1,40,1,25
470 INK 1,2
480 INK 2,24
490 PAPER 0
500 LOCATE 6,20
510 PRINT " The impossible made possible ";
520 LOCATE 6,20
530 PRINT CHR$(22);CHR$(1);
540 PRINT " ____________________________ ";
550 PRINT CHR$(22);CHR$(0);
560 PEN 3
570 LOCATE 9,22
580 PRINT CHR$(1);CHR$(9);
590 PEN 2
600 PRINT CHR$(24);
610 PRINT " Le Basic est ton ami ";
620 PRINT CHR$(24);
630 PEN 3
640 PRINT CHR$(1);CHR$(8);
650 PEN 1
        "#
        ),
        (
            "MODE2.BAS",
            r#"
10 MODE 2
11 INK 0,0
12 INK 1,26
13 BORDER 0
20 LOCATE 20,10
21 PRINT "The GPA Presents"
30 LOCATE 30,13
40 PRINT "another intriging tool"
50 LOCATE 40,16
60 PRINT "CATaclysme"
70 LOCATE 20,23
80 PRINT "Type : RUN CATACLYS to start the toolbox."
        "#
        ),
        (
            "TEST.BAS",
            r#"
10 MODE 1
20 BORDER 4
30 INK 0,4
40 INK 1,17
50 INK 2,22
60 PAPER 0
70 PEN 1
80 ' WORD 1: "horny" (rows 4-6)
90 ' h
100 PAPER 1
110 WINDOW 3,3,4,6
120 CLS
130 WINDOW 3,4,5,5
140 CLS
150 WINDOW 5,5,5,6
160 CLS
170 ' o (hollow)
180 WINDOW 7,9,4,6
190 CLS
200 PAPER 0
210 WINDOW 8,8,5,5
220 CLS
230 PAPER 1
240 ' r
250 WINDOW 11,11,4,6
260 CLS
270 WINDOW 12,13,4,4
280 CLS
290 ' n
300 WINDOW 15,17,4,6
310 CLS
320 PAPER 0
330 WINDOW 16,16,5,6
340 CLS
350 ' y
360 PAPER 1
370 WINDOW 19,21,4,6
380 CLS
390 WINDOW 21,21,7,7
400 CLS
410 PAPER 0
420 WINDOW 20,20,4,5
430 CLS
440 '=======================
450 ' WORD 2: "byte" (rows 8-10)
460 ' b (hollow center)
470 PAPER 2
480 WINDOW 6,8,8,10
490 CLS
500 PAPER 0
510 WINDOW 7,8,8,9
520 CLS
530 PAPER 2
540 WINDOW 8,8,9,9
550 CLS
560 ' y
570 WINDOW 10,12,8,10
580 CLS
590 WINDOW 12,12,11,11
600 CLS
610 PAPER 0
620 WINDOW 11,11,8,9
630 CLS
640 PAPER 2
650 ' t
660 WINDOW 14,16,9,9
670 CLS
680 WINDOW 15,15,8,10
690 CLS
700 ' e
710 WINDOW 18,20,8,10
720 CLS
730 PAPER 0
740 WINDOW 19,20,9,9
750 CLS
760 PAPER 3
770 '=======================
780 ' WORD 3: "lovers" (rows 12-14)
790 ' l
800 WINDOW 9,9,12,14
810 CLS
820 WINDOW 9,11,14,14
830 CLS
840 ' o (hollow)
850 WINDOW 13,15,12,14
860 CLS
870 PAPER 0
880 WINDOW 14,14,13,13
890 CLS
900 PAPER 3
910 ' v
920 WINDOW 17,17,12,13
930 CLS
940 WINDOW 19,19,12,13
950 CLS
960 WINDOW 18,18,14,14
970 CLS
980 ' e
990 WINDOW 21,23,12,14
1000 CLS
1010 PAPER 0
1020 WINDOW 22,23,13,13
1030 CLS
1040 PAPER 3
1050 ' r
1060 WINDOW 25,25,12,14
1070 CLS
1080 WINDOW 26,27,12,12
1090 CLS
1100 ' s
1110 WINDOW 29,31,12,14
1120 CLS
1121 WINDOW 31,31,15,15
1122 CLS
1130 PAPER 0
1140 WINDOW 30,31,13,13
1150 CLS
1200 '--- Tidy cursor ---
1201 WINDOW 1,40,1,25
1210 LOCATE 28,20
1220 PRINT "RUN";
1230 PRINT CHR$(34);
1240 PRINT "HBL"
1250 REM PEN 0
1260 WINDOW 28,35,19,20
        "#
        ),
    ];

    // --- Original single-demo code ---
    // let mut interpreter = Interpreter::new_6128();
    //
    // println!("--- Initial State ---");
    // println!("{}", interpreter);
    //
    // Create some custom commands
    // Clear screen, Locate 10,10, Print "Hello Rust!"
    // let cmds = BasicCommandList::from(vec![
    // BasicCommand::Cls,
    // BasicCommand::Locate(10, 10),
    // BasicCommand::print_string_crlf(b"Hello from Rust!"),
    // BasicCommand::Locate(10, 12),
    // BasicCommand::print_string(b"Char by char: "),
    // BasicCommand::print_string(b"A"),
    // BasicCommand::print_string(b"B"),
    // BasicCommand::print_string_crlf(b"C"),
    // ]);
    //
    // Convert to CharCommands
    // let char_cmds = cmds.to_char_commands().expect("Conversion failed");
    //
    // Execute them
    // interpreter.interpret(&char_cmds, true);
    //
    // println!("\n--- After Commands ---");
    // println!("{}", interpreter);
    for (name, code) in &catarts {
        let code = code.trim_start_matches('\n').trim_end();
        // Reset interpreter for each catart
        let mut interpreter = Interpreter::new_6128();
        println!("\n--- Running the BASIC program {} ---", name);
        // Parse BASIC and convert to commands
        let program = cpclib_basic::BasicProgram::parse(code).expect("Tokenization failed");
        let commands =
            cpclib_catart::convert::basic_to_commands(&program).expect("Conversion failed");
        let cmds = BasicCommandList::from(commands);
        let char_cmds = cmds.to_char_commands().expect("Char conversion failed");
        interpreter.interpret(&char_cmds, true);
        println!("{}", interpreter);

        // Display using catalog
        println!("\n--- Running the CATART {} ---", name);
        let builder = SerialCatalogBuilder::new();
        let catalog = builder.build(&char_cmds, cpclib_catart::entry::ScreenMode::Mode1);
        println!("\n--- Catalog for {} ---", name);
        println!("Number of entries: {}", catalog.entries().count());
        // Optionally, verify by reconstructing commands
        let reconstructed_cmds = catalog.commands_by_mode_and_order(
            cpclib_catart::entry::ScreenMode::Mode1,
            cpclib_catart::entry::CatalogType::Cat
        );
        println!(
            "Reconstructed commands length: {}",
            reconstructed_cmds.len()
        );

        let mut interpreter = Interpreter::new_6128();
        interpreter.interpret(&reconstructed_cmds, true);
        println!("{}", interpreter);
    }
}
