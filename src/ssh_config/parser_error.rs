#[derive(Debug)]
pub struct UnknownEntryError {
    pub line: String,
    pub entry: String,
}

#[derive(Debug)]
pub enum InvalidIncludeErrorDetails {
    Pattern(glob::PatternError),
    Glob(glob::GlobError),
    Io(std::io::Error),
    HostsInsideHostBlock,
}

#[derive(Debug)]
pub struct InvalidIncludeError {
    pub line: String,
    pub details: InvalidIncludeErrorDetails,
}

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    UnparseableLine(String),
    UnknownEntry(UnknownEntryError),
    InvalidInclude(InvalidIncludeError),
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::Io(e)
    }
}

impl From<UnknownEntryError> for ParseError {
    fn from(e: UnknownEntryError) -> Self {
        ParseError::UnknownEntry(e)
    }
}

impl From<InvalidIncludeError> for ParseError {
    fn from(e: InvalidIncludeError) -> Self {
        ParseError::InvalidInclude(e)
    }
}
