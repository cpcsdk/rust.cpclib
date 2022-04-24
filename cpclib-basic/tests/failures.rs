use cpclib_basic::BasicProgram;

#[test]
pub fn failures() {
    let codes = [
        "10 PRINT \"HELLO" // Unclosed string
    ];

    for code in codes {
        let basic = BasicProgram::parse(code);
        if let Err(_err) = basic {
            // everything is ok
        }
        else {
            panic!("{} has been parsed", code);
        }
    }
}
