#[derive(Debug, PartialEq)]
pub enum StreamingState {
    Null,
    Initial,
    Running,
    Stopped,
}
impl std::fmt::Display for StreamingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}


#[derive(Debug)]
pub enum StreamingError {
    Ok,
    InvalidStateTransition,
    InvalidParameter,
    InvalidInput,
    InvalidOutput,
    InvalidStatics,
    InvalidProcessorBlock,
    InvalidOperation,
    UnsetStatics,
    OutOfRange,
    WrongType,
    PathError,
    CreateError,
    ReadError,
    WriteError,
}

impl std::fmt::Display for StreamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}