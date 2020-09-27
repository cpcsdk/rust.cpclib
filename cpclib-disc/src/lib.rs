#![feature(register_attr)]
#![register_attr(get)]

/// Concerns all stuff related to Amsdos disc format
pub mod amsdos;
/// Utility function to build a DSK thanks to a format description
pub mod builder;
/// Parser of the format description
pub mod cfg;
/// EDSK File format
pub mod edsk;
