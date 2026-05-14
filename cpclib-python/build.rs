fn main() {
    // Required on macOS for Python extension modules built via cargo.
    // This injects "-undefined dynamic_lookup" for the final cdylib link.
    pyo3_build_config::add_extension_module_link_args();
}
