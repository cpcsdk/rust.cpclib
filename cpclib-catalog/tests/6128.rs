mod test_helpers;
use cpclib_catart::interpret::Interpreter;
use test_helpers::compare_memory_with_visual_diff;

#[test]
fn test_6128_warmup() {
    let mut interpreter = Interpreter::new_6128();
    let actual_screen_memory = interpreter.memory_screen().memory();
    let palette = interpreter.palette();

    let expected_screen_memory =
        std::fs::read("tests/6128.SCR").expect("Failed to read expected screen memory");

    if let Err(msg) = compare_memory_with_visual_diff(
        palette,
        (&expected_screen_memory[..])
            .try_into()
            .expect("Expected screen memory must be exactly 16384 bytes"),
        actual_screen_memory,
        "6128_screen_diff",
        "tests"
    ) {
        eprintln!("{}", msg);
        panic!("Screen memory mismatch - see PNG file for visual comparison");
    }

    assert_eq!(interpreter.memory_screen().r12r13(), 0);
}
