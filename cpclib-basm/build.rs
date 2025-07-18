fn main() {
    //   static_vcruntime::metabuild();
    build_deps::rerun_if_changed_paths("tests/asm/*.asm").unwrap();
    build_deps::rerun_if_changed_paths("tests/asm/*").unwrap();
    build_deps::rerun_if_changed_paths("tests/asm/").unwrap();
    build_deps::rerun_if_changed_paths("tests/asm").unwrap();

    built::write_built_file().expect("Failed to acquire build-time information");
}
