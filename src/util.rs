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
