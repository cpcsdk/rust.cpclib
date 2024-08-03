fn main() {
    static_vcruntime::metabuild();

    built::write_built_file().expect("Failed to acquire build-time information");
}
