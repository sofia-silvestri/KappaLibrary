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


#[derive(Debug, PartialEq)]
pub enum StreamErrCode {
    Ok,
    GenericError,
    AlreadyDefined,
    InvalidStateTransition,
    InvalidParameter,
    InvalidInput,
    InvalidOutput,
    InvalidStatics,
    InvalidProcessorBlock,
    InvalidOperation,
    SendDataError,
    ReceiveDataError,
    UnsetStatics,
    OutOfRange,
    WrongType,
    PathError,
    FileNotFound,
    CreateError,
    ReadError,
    WriteError,
    TaskError,
}
impl std::fmt::Display for StreamErrCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
pub struct StreamingError {
    pub code: StreamErrCode,
    pub message: String,
}
impl StreamingError {
    pub fn new(code: StreamErrCode, message: &str) -> Self {
        StreamingError {
            code,
            message: message.to_string(),
        }
    }
}
impl std::fmt::Display for StreamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error {:?}: {}", self.code, self.message)
    }
}