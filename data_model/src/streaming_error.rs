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