
fn main() {
    build_deps::rerun_if_changed_paths( "tests/asm/*" ).unwrap();
}