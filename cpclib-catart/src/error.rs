/// Errors that can occur when processing CatArt commands
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatArtError {
    /// A BASIC command that is not compatible with CatArt (line_number, command_name)
    IncompatibleBasicCommand(u16, String),
    /// Ran out of tokens while parsing a command
    NotEnoughTokens(String),
    /// A parameter value was invalid (line_number, details)
    InvalidParameter(u16, String)
}
