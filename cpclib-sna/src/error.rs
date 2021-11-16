#[derive(Debug, Copy, Clone)]
#[allow(missing_docs)]
pub enum SnapshotError {
    FileError,
    NotEnougSpaceAvailable,
    InvalidValue,
    FlagDoesNotExists,
    InvalidIndex
}
