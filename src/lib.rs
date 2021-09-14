use thiserror::Error;
use mupen64plus_sys::*;

pub mod core;
pub mod plugin;

pub use crate::core::Core;
pub use plugin::Plugin;

#[derive(Error, Debug)]
pub enum Error {
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
    #[error("a plugin fucntion returned a fatal error")]
    PluginFail,
    #[error("a system function call, such as an SDL or file operation, failed")]
    SystemFail,
    #[error("function call is not supported (ie, core not built with debugger)")]
    Unsupported,
    #[error("a given input type parameter cannot be used for desired operation")]
    WrongType,

    #[error("no PluginStartup() function found")]
    NoPluginStartup,
    #[error("no ROM open")]
    NoRomOpen,
}

impl From<m64p_error> for Error {
    fn from(err: m64p_error) -> Self {
        #[allow(non_upper_case_globals)]
        match err {
            m64p_error_M64ERR_SUCCESS => panic!("refusing to convert m64p_error=SUCCESS to Error"),
            m64p_error_M64ERR_ALREADY_INIT => Error::AlreadyInit,
            m64p_error_M64ERR_NOT_INIT => Error::NotInit,
            m64p_error_M64ERR_INCOMPATIBLE => Error::Incompatible,
            m64p_error_M64ERR_INPUT_ASSERT => Error::InputAssert,
            m64p_error_M64ERR_INPUT_INVALID => Error::InputInvalid,
            m64p_error_M64ERR_INPUT_NOT_FOUND => Error::InputNotFound,
            m64p_error_M64ERR_NO_MEMORY => Error::NoMemory,
            m64p_error_M64ERR_FILES => Error::Files,
            m64p_error_M64ERR_INTERNAL => Error::Internal,
            m64p_error_M64ERR_INVALID_STATE => Error::InvalidState,
            m64p_error_M64ERR_PLUGIN_FAIL => Error::PluginFail,
            m64p_error_M64ERR_SYSTEM_FAIL => Error::SystemFail,
            m64p_error_M64ERR_UNSUPPORTED => Error::Unsupported,
            m64p_error_M64ERR_WRONG_TYPE => Error::WrongType,
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
