use thiserror::Error;
use mupen64plus_sys::*;

pub use mupen64plus_sys as sys;

pub mod core;
pub mod plugin;
pub mod vidext;

pub use crate::core::Core;
pub use plugin::Plugin;
pub use vidext::Video;

#[derive(Error, Debug)]
pub enum MupenError {
    #[error("a function was called before its associated module was initialized")]
    NotInit,
    #[error("initialization function called twice")]
    AlreadyInit,
    #[error("API versions between components are incompatible")]
    Incompatible,
    #[error("invalid function parameters, such as a NULL pointer")]
    InputAssert,
    #[error("an input function parameter is logically invalid")]
    InputInvalid,
    #[error("the input parameter(s) specified a particular item which was not found")]
    InputNotFound,
    #[error("memory allocation failed")]
    NoMemory,
    #[error("error opening, creating, reading, or writing to a file")]
    Files,
    #[error("internal mupen64plus error (bug)")]
    Internal,
    #[error("current program state does not allow operations")]
    InvalidState,
    #[error("a plugin function returned a fatal error")]
    PluginFail,
    #[error("a system function call, such as an SDL or file operation, failed")]
    SystemFail,
    #[error("function call is not supported (ie, core not built with debugger)")]
    Unsupported,
    #[error("a given input type parameter cannot be used for desired operation")]
    WrongType,
}

impl From<m64p_error> for MupenError {
    fn from(err: m64p_error) -> Self {
        #[allow(non_upper_case_globals)]
        match err {
            m64p_error_M64ERR_SUCCESS => panic!("refusing to convert m64p_error=SUCCESS to Error"),
            m64p_error_M64ERR_ALREADY_INIT => Self::AlreadyInit,
            m64p_error_M64ERR_NOT_INIT => Self::NotInit,
            m64p_error_M64ERR_INCOMPATIBLE => Self::Incompatible,
            m64p_error_M64ERR_INPUT_ASSERT => Self::InputAssert,
            m64p_error_M64ERR_INPUT_INVALID => Self::InputInvalid,
            m64p_error_M64ERR_INPUT_NOT_FOUND => Self::InputNotFound,
            m64p_error_M64ERR_NO_MEMORY => Self::NoMemory,
            m64p_error_M64ERR_FILES => Self::Files,
            m64p_error_M64ERR_INTERNAL => Self::Internal,
            m64p_error_M64ERR_INVALID_STATE => Self::InvalidState,
            m64p_error_M64ERR_PLUGIN_FAIL => Self::PluginFail,
            m64p_error_M64ERR_SYSTEM_FAIL => Self::SystemFail,
            m64p_error_M64ERR_UNSUPPORTED => Self::Unsupported,
            m64p_error_M64ERR_WRONG_TYPE => Self::WrongType,
            _ => panic!("unknown m64p_error={}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_system() {
        Core::load_from_system().unwrap();
    }
}
