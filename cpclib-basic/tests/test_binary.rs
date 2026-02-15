use cpclib_basic::BasicProgram;

#[test]
fn test_blocus() {
    // WARNING THere are some trainling spaces at the end of some lines. Do not remove them !!
    let source = r#"10 MODE 1
20 BORDER 4
21 INK 0,4   
30 INK 1,26
31 INK 2,17
32 INK 3,25
40 PAPER 0 
50 PEN 1
59 LOCATE 2,4  
60 PRINT"There are only 7 real demos on CPC..."
69 LOCATE 2,4
70 PRINT
71 PEN 2
79 LOCATE 13,6
80 PRINT"1. RTS (1998)
89 LOCATE 7,7
90 PRINT"2. Ecole Buissoniere (2000)" 
99 LOCATE 9,8
100 PRINT"3. A Step Beyond (2003)"
109 LOCATE 8,9
110 PRINT"4. Midline Process (2004)"
119 LOCATE 9,10
120 PRINT"5. From Scratch (2009)"
129 LOCATE 11,11
130 PRINT"6. Pheelone (2009)"
139 LOCATE 8,12
140 PRINT"7. Batman Forever (2011)"
150 PEN 1
152 LOCATE 3,14
154 PRINT"However, there are tons of intros..."  
160 PEN 3
162 LOCATE 2,16
164 PRINT"CPC scene desperately needs newcomers!"    
170 LOCATE 8,17
172 PRINT"We are waiting for you!" 
180 PEN 1
182 LOCATE 1,22
184 PRINT"Run"+CHR$(34)+"-bloc.us"   
185 PEN 3
186 LOCATE 18,25
188 PRINT"...enjoy the 8th demo!"   
190 PEN 3
192 LOCATE 1,21 
"#;

    let content = std::fs::read("tests/TEST-CAT.BAS").unwrap();
    let mut content = &content[128..];
    let binary_tokens = BasicProgram::decode(content).expect("Error in the basic file");
    
    let src_tokens = BasicProgram::parse(source).expect("Error in the basic file");
    
    let bin_str = binary_tokens.to_string();
    let src_str = src_tokens.to_string();
    
    if bin_str != src_str {
        // Find first difference
        for (i, (b, s)) in bin_str.chars().zip(src_str.chars()).enumerate() {
            if b != s {
                eprintln!("First difference at position {}: binary={:?}, source={:?}", i, b, s);
                let start = if i > 20 { i - 20 } else { 0 };
                let end = (i + 50).min(bin_str.len());
                eprintln!("Binary context: {:?}", &bin_str[start..end]);
                eprintln!("Source context: {:?}", &src_str[start..end]);
                break;
            }
        }
    }

    assert_eq!(bin_str, src_str);
}