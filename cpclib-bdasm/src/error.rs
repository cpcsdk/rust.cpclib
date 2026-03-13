use cpclib_asm::Token;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BdAsmError {
    #[error("Expression evaluation failed: {0}")]
    ExprEvaluation(String),

    #[error("Invalid address value: expected integer")]
    InvalidAddress,

    #[error(
        "Unable to determine assembling address for instruction: {instruction:?} ({bytes} bytes)"
    )]
    UnknownAssemblerAddress { instruction: Token, bytes: usize },

    #[error("Failed to assemble listing: {0}")]
    AssemblyFailed(String),

    #[error("Invalid data bloc format: {0}")]
    InvalidDataBloc(String),

    #[error("Value type not supported for address resolution: {0}")]
    UnsupportedValueType(String),

    #[error("Invalid address range: {0}")]
    InvalidAddressRange(String),

    #[error("Control file error: {0}")]
    ControlFile(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseInt(#[from] std::num::ParseIntError)
}

pub type Result<T> = std::result::Result<T, BdAsmError>;
