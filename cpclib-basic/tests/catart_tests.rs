/// The aim of this file is to test if we can parse/ptokenize
/// minmal instructions needed for catart

#[test]
pub fn test_voxy_hbl_facehug() {
    let code = r###"10 MODE 2
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
170 :'--- Tidy cursor ---
180 WINDOW 37,48,19,21
190 LOCATE 37,20
200 PRINT "RUN";
210 PRINT CHR$(34);
220 PRINT "HBL";
230 PRINT CHR$(34);
250 WINDOW 37,48,20,21"###;

    let tokens = cpclib_basic::BasicProgram::parse(code).expect("Tokenization failed");
    let reconstruct = tokens.to_string();

    for (i, (orig, recon)) in code.lines().zip(reconstruct.lines()).enumerate() {
        if orig != recon {
            eprintln!("Line {}: ", i + 1);
            eprintln!("  Original:      {:?}", orig);
            eprintln!("  Reconstructed: {:?}", recon);
        }
    }

    assert_eq!(
        code.trim_ascii_end(),
        reconstruct.as_str().trim_ascii_end(),
        "Reconstructed code does not match original"
    );
}
