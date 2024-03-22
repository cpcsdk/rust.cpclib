use std::env;
use std::path::Path;

fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");
}
