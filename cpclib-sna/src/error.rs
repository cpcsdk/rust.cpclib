use std::path::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum SnapshotError {
    FileError,
    NotEnougSpaceAvailable,
    InvalidValue,
    FlagDoesNotExists,
    InvalidIndex,
    AnyError(String)
}
