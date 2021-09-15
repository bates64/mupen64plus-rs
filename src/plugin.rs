use mupen64plus_sys::*;
use semver::Version;
use bitflags::bitflags;
use std::ffi::CStr;
use std::path::Path;
use libloading::Library;
use crate::Error;

pub const MINIMUM_CORE_VERSION: Version = mupen_to_version(0x016300);
pub const CORE_API_VERSION: Version = mupen_to_version(0x020001);

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("failed to load library: {0}")]
    LibLoading(#[from] libloading::Error),
    #[error("bad plugin type: {0:?}")]
    BadPluginType(PluginType),
    #[error("plugin version ({0}) is unsupported")]
    IncompatibleVersion(Version),
    #[error("m64p_error: {0}")]
    M64Err(#[from] Error),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PluginVersion<'a> {
    pub plugin_type: PluginType,
    pub plugin_version: Version,
    pub api_version: Version,
    pub plugin_name: std::borrow::Cow<'a, str>,
    pub capabilities: Capability,
}

bitflags! {
    #[derive(Default)]
    pub struct Capability: m64p_core_caps {
        const DYNAREC = m64p_core_caps_M64CAPS_DYNAREC;
        const DEBUGGER = m64p_core_caps_M64CAPS_DEBUGGER;
        const CORE_COMPARE = m64p_core_caps_M64CAPS_CORE_COMPARE;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PluginType {
    Rsp,
    Gfx,
    Audio,
    Input,
    Core,
    Other(m64p_plugin_type),
}

impl From<m64p_plugin_type> for PluginType {
    fn from(plugin_type: m64p_plugin_type) -> Self {
        #[allow(non_upper_case_globals)]
        match plugin_type {
            m64p_plugin_type_M64PLUGIN_RSP => PluginType::Rsp,
            m64p_plugin_type_M64PLUGIN_GFX => PluginType::Gfx,
            m64p_plugin_type_M64PLUGIN_AUDIO => PluginType::Audio,
            m64p_plugin_type_M64PLUGIN_INPUT => PluginType::Input,
            m64p_plugin_type_M64PLUGIN_CORE => PluginType::Core,
            _ => PluginType::Other(plugin_type),
        }
    }
}

impl From<PluginType> for m64p_plugin_type {
    fn from(plugin_type: PluginType) -> Self {
        #[allow(non_upper_case_globals)]
        match plugin_type {
            PluginType::Rsp => m64p_plugin_type_M64PLUGIN_RSP,
            PluginType::Gfx => m64p_plugin_type_M64PLUGIN_GFX,
            PluginType::Audio => m64p_plugin_type_M64PLUGIN_AUDIO,
            PluginType::Input => m64p_plugin_type_M64PLUGIN_INPUT,
            PluginType::Core => m64p_plugin_type_M64PLUGIN_CORE,
            PluginType::Other(plugin_type) => plugin_type,
        }
    }
}

/// Converts a mupen64plus version number to a semver::Version.
pub const fn mupen_to_version(x: i32) -> Version {
    Version::new(
        ((x >> 16) & 0xffff) as u64,
        ((x >> 8) & 0xff) as u64,
        (x & 0xff) as u64,
    )
}

/// Converts a semver::Version to a mupen64plus version number.
pub const fn version_to_mupen(v: &Version) -> i32 {
    ((v.major as i32) << 16) | ((v.minor as i32) << 8) | (v.patch as i32)
}

impl PluginVersion<'_> {
    pub fn is_compatible(&self) -> bool {
        // This is how mupen64plus-ui-console checks for version compatibility, but it may be wise
        // to use a semver expression instead.
        !(self.plugin_version < MINIMUM_CORE_VERSION || self.api_version.major != CORE_API_VERSION.major)
    }

    pub(crate) fn from_ffi<'a>(plugin_get_version: ptr_PluginGetVersion) -> Result<PluginVersion<'a>, Error> {
        let mut plugin_type = 0;
        let mut plugin_version = 0;
        let mut api_version = 0;
        let mut plugin_name = std::ptr::null();
        let mut capabilities = 0;
        unsafe {
            let ret = plugin_get_version.unwrap()(&mut plugin_type, &mut plugin_version, &mut api_version, &mut plugin_name, &mut capabilities);
            if ret != m64p_error_M64ERR_SUCCESS {
                return Err(ret.into());
            }
        }
        let plugin_name = unsafe { CStr::from_ptr(plugin_name) };

        Ok(PluginVersion {
            plugin_type: plugin_type.into(),
            plugin_version: mupen_to_version(plugin_version),
            api_version: mupen_to_version(api_version),
            plugin_name: plugin_name.to_string_lossy(),
            capabilities: Capability::from_bits_truncate(capabilities as m64p_core_caps),
        })
    }
}

pub struct Plugin {
    pub(crate) lib: m64p_dynlib_handle,
    plugin_get_version: ptr_PluginGetVersion,
    pub(crate) plugin_startup: ptr_PluginStartup,
    pub(crate) plugin_shutdown: ptr_PluginShutdown,
}

impl Plugin {
    pub fn load_from_path<P>(dylib_path: P) -> Result<Self, LoadError>
    where
        P: AsRef<Path>
    {
        Self::load_from_library(unsafe {
            Library::new(dylib_path.as_ref().as_os_str())?
        })
    }

    pub fn load_from_library<L>(lib: L) -> Result<Self, LoadError>
    where
        L: Into<Library>
    {
        let lib = lib.into();

        let plugin = Self {
            plugin_get_version: unsafe { lib.get(b"PluginGetVersion\0") }.ok().and_then(|p| *p),
            plugin_startup: unsafe { lib.get(b"PluginStartup\0") }.ok().and_then(|p| *p),
            plugin_shutdown: unsafe { lib.get(b"PluginShutdown\0") }.ok().and_then(|p| *p),
            lib: {
                #[cfg(unix)]
                use libloading::os::unix::Library;
                #[cfg(windows)]
                use libloading::os::windows::Library;

                let lib: Library = lib.into();
                lib.into_raw()
            },
        };

        if let Ok(version) = plugin.get_version() {
            if version.plugin_type == PluginType::Core {
                return Err(LoadError::BadPluginType(version.plugin_type));
            }

            if !version.is_compatible() {
                // Not a hard error because the frontend doesn't talk to the plugins
                log::warn!("possibly incompatible plugin loaded (API version={})", version.api_version);
            }
        }

        Ok(plugin)
    }

    pub fn get_version(&self) -> Result<PluginVersion<'_>, Error> {
        PluginVersion::from_ffi(self.plugin_get_version)
    }
}

impl Drop for Plugin {
    fn drop(&mut self) {
        #[cfg(unix)]
        use libloading::os::unix::Library;
        #[cfg(windows)]
        use libloading::os::windows::Library;

        // Close the library
        let lib = unsafe { Library::from_raw(self.lib) };
        let _ = lib.close();
    }
}

unsafe impl Send for Plugin {}
unsafe impl Sync for Plugin {}
