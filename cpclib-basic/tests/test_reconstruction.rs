use cpclib_basic::string_parser::parse_basic_line;
use cpclib_common::winnow::Parser;
use fs_err as fs;

#[test]
fn show_skipped_lines() {
    let file = "tests/amstrad-cpc-projects-master/graphics/bounce.bas";
    let content = fs::read_to_string(file).unwrap();

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;

        if line.trim().is_empty() {
            continue;
        }

        let line_with_newline = format!("{}\n", line);

        if let Ok(parsed_line) = parse_basic_line.parse(&line_with_newline) {
            let reconstructed = parsed_line.to_string();

            if reconstructed.contains("<const:") || reconstructed.contains("<value:") {
                println!("Line {}: {}", line_num, line);
                println!("  Reconstructed: {}", reconstructed);
                println!();
            }
        }
    }
}
