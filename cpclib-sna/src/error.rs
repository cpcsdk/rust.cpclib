#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum SnapshotError {
    FileError,
    NotEnougSpaceAvailable,
    InvalidValue,
    FlagDoesNotExists,
    InvalidIndex
}
