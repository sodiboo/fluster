use crate::sys;

simple_enum! {
    pub enum EngineResult(sys::FlutterEngineResult) {
        Success,
        InvalidLibraryVersion,
        InvalidArguments,
        InternalInconsistency,
    }
}

impl sys::FlutterEngineResult {
    pub fn to_result(self) -> crate::Result<()> {
        let result: EngineResult = self.try_into().expect("invalid FlutterEngineResult; flutter added a new variant but i thought that enum was exhaustive");

        result.into()
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Error {
    InvalidLibraryVersion,
    InvalidArguments,
    InternalInconsistency,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidLibraryVersion => write!(f, "There has been a serious breakage in the Flutter embedder API. The version of the Flutter Engine that this library was compiled against is fundamentally incompatible with the version of the Flutter Engine that is present on the current system."),
            Error::InvalidArguments => write!(f, "Invalid arguments were passed to a function. You should check the documentation for the function you are calling to see what you might have done wrong."),
            Error::InternalInconsistency => write!(f, "Internal inconsistency; this is likely a bug in the Flutter Engine"),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for std::io::Error {
    fn from(error: Error) -> std::io::Error {
        let kind = match error {
            Error::InvalidArguments => std::io::ErrorKind::InvalidInput,
            Error::InvalidLibraryVersion => std::io::ErrorKind::Unsupported,
            Error::InternalInconsistency => std::io::ErrorKind::Other,
        };
        std::io::Error::new(kind, error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<EngineResult> for crate::Result<()> {
    fn from(result: EngineResult) -> Self {
        match result {
            EngineResult::Success => Ok(()),
            EngineResult::InvalidLibraryVersion => Err(Error::InvalidLibraryVersion),
            EngineResult::InvalidArguments => Err(Error::InvalidArguments),
            EngineResult::InternalInconsistency => Err(Error::InternalInconsistency),
        }
    }
}

pub(crate) unsafe fn return_out_param<T>(out: *mut T, value: Option<impl Into<T>>) -> bool {
    if let Some(value) = value {
        unsafe { std::ptr::write(out, value.into()) };
        true
    } else {
        false
    }
}
