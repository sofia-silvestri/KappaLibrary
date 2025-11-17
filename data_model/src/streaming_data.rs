#[derive(Debug, PartialEq)]

#[repr(C)]
pub struct TaskStatistics {
    pub timestamp: f64,
    pub mean: f64,
    pub max: f64,
    pub min: f64,
    pub std_dev: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
}

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
    CreateError,
    ReadError,
    WriteError,
    TaskError,
}

impl std::fmt::Display for StreamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}