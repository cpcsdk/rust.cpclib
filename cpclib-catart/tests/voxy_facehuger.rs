use cpclib_catart::basic_command::{BasicCommand, PrintArgument};

const  BASIC_CODE : &str = r###"10 MODE 2
20 PAPER 0
30 INK 0,0
40 INK 1,26
50 LOCATE 13,6
60 PRINT "   ___  _____  _________  _______________________  _  __"
70 LOCATE 13,7
80 PRINT "  / _ // __/ |/ / __/ _ \/  _/ ___/_  __/  _/ __ \/ |/ /"
90 LOCATE 13,8
100 PRINT " / _  / _//    / _// // // // /__  / / _/ // /_/ /    /"
110 LOCATE 13,9
120 PRINT "/____/___/_/|_/___/____/___/\___/ /_/ /___/\____/_/|_/"
130 LOCATE 33,13
140 PRINT "proudly presents"
150 LOCATE 32,17
160 PRINT "HORNY  BYTE  LOVERS"
170 WINDOW 37,48,19,21
180 LOCATE 37,20
190 PRINT "RUN";
200 PRINT CHR$(34);
210 PRINT "HBL";
220 PRINT CHR$(34);
230 WINDOW 37,48,20,21"###;

#[test]
pub fn test_voxy_hbl_facehug() {
    let tokens = cpclib_basic::BasicProgram::parse(BASIC_CODE).expect("Tokenization failed");
    let commands = cpclib_catart::convert::basic_to_commands(&tokens).expect("Conversion failed");
    let expected =        [
            BasicCommand::mode(2),
            BasicCommand::paper(0),
            BasicCommand::ink(0, 0, None),
            BasicCommand::ink(1, 26, None),
            BasicCommand::locate(13, 6),
            BasicCommand::print_string_crlf(br#"   ___  _____  _________  _______________________  _  __"#),
            BasicCommand::locate(13, 7),
            BasicCommand::print_string_crlf(br#"  / _ // __/ |/ / __/ _ \/  _/ ___/_  __/  _/ __ \/ |/ /"#),
            BasicCommand::locate(13, 8),
            BasicCommand::print_string_crlf(br#" / _  / _//    / _// // // // /__  / / _/ // /_/ /    /"#),
            BasicCommand::locate(13, 9),
            BasicCommand::print_string_crlf(br#"/____/___/_/|_/___/____/___/\___/ /_/ /___/\____/_/|_/"#),
            BasicCommand::locate(33, 13),
            BasicCommand::print_string_crlf(br#"proudly presents"#),
            BasicCommand::locate(32, 17),
            BasicCommand::print_string_crlf(br#"HORNY  BYTE  LOVERS"#),
            BasicCommand::window(37, 48, 19, 21),
            BasicCommand::locate(37, 20),
            BasicCommand::print_string(br#"RUN"#), // PRINT "RUN"; - has semicolon, no \r\n
            BasicCommand::print_string(PrintArgument::ChrDollar(34)), // PRINT CHR$(34); - has semicolon, no \r\n
            BasicCommand::print_string(br#"HBL"#), // PRINT "HBL"; - has semicolon, no \r\n
            BasicCommand::print_string(PrintArgument::ChrDollar(34)), // PRINT CHR$(34); - has semicolon, no \r\n
            BasicCommand::window(37, 48, 20, 21),
        ];
    
    for (idx, (obtained, expected)) in commands.iter().zip(expected.iter()).enumerate() {
        println!("Checking command {}... ", idx + 1);
        assert_eq!(obtained, expected, "Command mismatch: \nObtained: {:?}\nExpected: {:?}", obtained, expected);
    }

    let genrated_listing = commands.to_string();
    for (obtained, generated) in genrated_listing.lines().zip(BASIC_CODE.lines()) {
        assert_eq!(obtained, generated, "Listing line mismatch:\nObtained: {:?}\nExpected: {:?}", obtained, generated);
    }


    let char_cmds = dbg!(commands.to_char_commands().expect("Char command conversion failed"));
}