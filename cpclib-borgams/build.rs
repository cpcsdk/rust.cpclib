use built;

pub fn main() -> std::io::Result<()> {
    built::write_built_file().unwrap();
    Ok(())
}
