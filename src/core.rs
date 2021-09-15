use std::path::Path;
use std::ffi::CStr;
use std::sync::Arc;
use libloading::Library;
use mupen64plus_sys::*;
use crate::Error;
use crate::plugin::*;

pub mod debug;

/// The emulator core, also known as `libmupen64plus`.
#[allow(dead_code)]
pub struct Core {
    lib: m64p_dynlib_handle,

    // API
    plugin_get_version: ptr_PluginGetVersion,
    core_error_message: ptr_CoreErrorMessage,
    core_startup: ptr_CoreStartup,
    core_shutdown: ptr_CoreShutdown,
    core_attach_plugin: ptr_CoreAttachPlugin,
    core_detach_plugin: ptr_CoreDetachPlugin,
    core_do_command: ptr_CoreDoCommand,
    core_override_vid_ext: ptr_CoreOverrideVidExt,
    core_add_cheat: ptr_CoreAddCheat,
    core_cheat_enabled: ptr_CoreCheatEnabled,
    config_list_sections: ptr_ConfigListSections,
    config_open_section: ptr_ConfigOpenSection,
    config_delete_section: ptr_ConfigDeleteSection,
    config_list_parameters: ptr_ConfigListParameters,
    config_save_file: ptr_ConfigSaveFile,
    config_set_parameter: ptr_ConfigSetParameter,
    config_get_parameter: ptr_ConfigGetParameter,
    config_get_parameter_type: ptr_ConfigGetParameterType,
    config_get_parameter_help: ptr_ConfigGetParameterHelp,
    config_set_default_int: ptr_ConfigSetDefaultInt,
    config_set_default_float: ptr_ConfigSetDefaultFloat,
    config_set_default_bool: ptr_ConfigSetDefaultBool,
    config_set_default_string: ptr_ConfigSetDefaultString,
    config_get_param_int: ptr_ConfigGetParamInt,
    config_get_param_float: ptr_ConfigGetParamFloat,
    config_get_param_bool: ptr_ConfigGetParamBool,
    config_get_param_string: ptr_ConfigGetParamString,
    config_external_open: ptr_ConfigExternalOpen,
    config_external_close: ptr_ConfigExternalClose,
    config_external_get_parameter: ptr_ConfigExternalGetParameter,
    config_has_unsaved_changes: ptr_ConfigHasUnsavedChanges,
    config_get_shared_data_filepath: ptr_ConfigGetSharedDataFilepath,
    config_get_user_config_path: ptr_ConfigGetUserConfigPath,
    config_get_user_data_path: ptr_ConfigGetUserDataPath,
    config_get_user_cache_path: ptr_ConfigGetUserCachePath,
    debug_set_callbacks: ptr_DebugSetCallbacks,
    debug_set_core_compare: ptr_DebugSetCoreCompare,
    debug_set_run_state: ptr_DebugSetRunState,
    debug_get_state: ptr_DebugGetState,
    debug_step: ptr_DebugStep,
    debug_decode_op: ptr_DebugDecodeOp,
    debug_mem_get_recomp_info: ptr_DebugMemGetRecompInfo,
    debug_mem_get_mem_info: ptr_DebugMemGetMemInfo,
    debug_mem_get_pointer: ptr_DebugMemGetPointer,
    debug_mem_read64: ptr_DebugMemRead64,
    debug_mem_read32: ptr_DebugMemRead32,
    debug_mem_read16: ptr_DebugMemRead16,
    debug_mem_read8: ptr_DebugMemRead8,
    debug_mem_write64: ptr_DebugMemWrite64,
    debug_mem_write32: ptr_DebugMemWrite32,
    debug_mem_write16: ptr_DebugMemWrite16,
    debug_mem_write8: ptr_DebugMemWrite8,
    debug_get_cpu_data_ptr: ptr_DebugGetCPUDataPtr,
    debug_breakpoint_lookup: ptr_DebugBreakpointLookup,
    debug_breakpoint_command: ptr_DebugBreakpointCommand,
    debug_breakpoint_triggered_by: ptr_DebugBreakpointTriggeredBy,
    debug_virtual_to_physical: ptr_DebugVirtualToPhysical,
}

/// A running instance of the emulator core, created with `Core::start`.
pub struct Mupen {
    core: Arc<Core>,
    plugins: Vec<Plugin>, // TODO: map for each plugin type
    is_rom_open: bool, // TODO: replace with state check call
}

impl Core {
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

        macro_rules! load_func {
            ($f:ident) => {{
                let fstr: String = stringify!($f).chars().skip(4).collect();
                let fbstr: Vec<u8> = fstr.bytes().collect();
                if let Ok(f) = unsafe { lib.get(&fbstr) } {
                    *f
                } else {
                    None
                }
            }};
        }

        let plugin = Self {
            plugin_get_version: load_func!(ptr_PluginGetVersion),

            core_error_message: load_func!(ptr_CoreErrorMessage),
            core_startup: load_func!(ptr_CoreStartup),
            core_shutdown: load_func!(ptr_CoreShutdown),
            core_attach_plugin: load_func!(ptr_CoreAttachPlugin),
            core_detach_plugin: load_func!(ptr_CoreDetachPlugin),
            core_do_command: load_func!(ptr_CoreDoCommand),
            core_override_vid_ext: load_func!(ptr_CoreOverrideVidExt),
            core_add_cheat: load_func!(ptr_CoreAddCheat),
            core_cheat_enabled: load_func!(ptr_CoreCheatEnabled),
            config_list_sections: load_func!(ptr_ConfigListSections),
            config_open_section: load_func!(ptr_ConfigOpenSection),
            config_delete_section: load_func!(ptr_ConfigDeleteSection),
            config_list_parameters: load_func!(ptr_ConfigListParameters),
            config_save_file: load_func!(ptr_ConfigSaveFile),
            config_set_parameter: load_func!(ptr_ConfigSetParameter),
            config_get_parameter: load_func!(ptr_ConfigGetParameter),
            config_get_parameter_type: load_func!(ptr_ConfigGetParameterType),
            config_get_parameter_help: load_func!(ptr_ConfigGetParameterHelp),
            config_set_default_int: load_func!(ptr_ConfigSetDefaultInt),
            config_set_default_float: load_func!(ptr_ConfigSetDefaultFloat),
            config_set_default_bool: load_func!(ptr_ConfigSetDefaultBool),
            config_set_default_string: load_func!(ptr_ConfigSetDefaultString),
            config_get_param_int: load_func!(ptr_ConfigGetParamInt),
            config_get_param_float: load_func!(ptr_ConfigGetParamFloat),
            config_get_param_bool: load_func!(ptr_ConfigGetParamBool),
            config_get_param_string: load_func!(ptr_ConfigGetParamString),
            config_external_open: load_func!(ptr_ConfigExternalOpen),
            config_external_close: load_func!(ptr_ConfigExternalClose),
            config_external_get_parameter: load_func!(ptr_ConfigExternalGetParameter),
            config_has_unsaved_changes: load_func!(ptr_ConfigHasUnsavedChanges),
            config_get_shared_data_filepath: load_func!(ptr_ConfigGetSharedDataFilepath),
            config_get_user_config_path: load_func!(ptr_ConfigGetUserConfigPath),
            config_get_user_data_path: load_func!(ptr_ConfigGetUserDataPath),
            config_get_user_cache_path: load_func!(ptr_ConfigGetUserCachePath),
            debug_set_callbacks: load_func!(ptr_DebugSetCallbacks),
            debug_set_core_compare: load_func!(ptr_DebugSetCoreCompare),
            debug_set_run_state: load_func!(ptr_DebugSetRunState),
            debug_get_state: load_func!(ptr_DebugGetState),
            debug_step: load_func!(ptr_DebugStep),
            debug_decode_op: load_func!(ptr_DebugDecodeOp),
            debug_mem_get_recomp_info: load_func!(ptr_DebugMemGetRecompInfo),
            debug_mem_get_mem_info: load_func!(ptr_DebugMemGetMemInfo),
            debug_mem_get_pointer: load_func!(ptr_DebugMemGetPointer),
            debug_mem_read64: load_func!(ptr_DebugMemRead64),
            debug_mem_read32: load_func!(ptr_DebugMemRead32),
            debug_mem_read16: load_func!(ptr_DebugMemRead16),
            debug_mem_read8: load_func!(ptr_DebugMemRead8),
            debug_mem_write64: load_func!(ptr_DebugMemWrite64),
            debug_mem_write32: load_func!(ptr_DebugMemWrite32),
            debug_mem_write16: load_func!(ptr_DebugMemWrite16),
            debug_mem_write8: load_func!(ptr_DebugMemWrite8),
            debug_get_cpu_data_ptr: load_func!(ptr_DebugGetCPUDataPtr),
            debug_breakpoint_lookup: load_func!(ptr_DebugBreakpointLookup),
            debug_breakpoint_command: load_func!(ptr_DebugBreakpointCommand),
            debug_breakpoint_triggered_by: load_func!(ptr_DebugBreakpointTriggeredBy),
            debug_virtual_to_physical: load_func!(ptr_DebugVirtualToPhysical),

            lib: {
                #[cfg(unix)]
                use libloading::os::unix::Library;
                #[cfg(windows)]
                use libloading::os::windows::Library;

                let lib: Library = lib.into();
                lib.into_raw()
            },
        };

        let version = plugin.get_version()?;

        if version.plugin_type != PluginType::Core {
            return Err(LoadError::BadPluginType(version.plugin_type));
        }

        if !version.is_compatible() {
            return Err(LoadError::IncompatibleVersion(version.api_version));
        }

        Ok(plugin)
    }

    pub fn get_version(&self) -> Result<PluginVersion, Error> {
        PluginVersion::from_ffi(self.plugin_get_version)
    }
}

impl Core {
    pub fn load_from_system() -> Result<Self, LoadError> {
        Self::load_from_library(unsafe {
            Library::new(libloading::library_filename("mupen64plus"))?
        })
    }

    pub fn load_from_directory(dir: &str) -> Result<Self, LoadError> {
        Self::load_from_path(format!("{}/{}", dir, libloading::library_filename("mupen64plus").to_string_lossy()))
    }

    /// Startup the core and load configuration/data.
    ///
    /// `data_dir` should contain `mupen64plus.ini` and `mupencheat.txt`.
    pub fn start<P1, P2>(
        self,
        config_dir: Option<P1>,
        data_dir: Option<P2>
    ) -> Result<Mupen, Error>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let config_dir = config_dir.and_then(|s| s.as_ref()
            .to_str()
            .and_then(|p| std::ffi::CString::new(p).ok()));
        let data_dir = data_dir.and_then(|s| s.as_ref()
            .to_str()
            .and_then(|p| std::ffi::CString::new(p).ok()));

        unsafe {
            let r = self.core_startup.unwrap()(
                crate::plugin::version_to_mupen(&crate::plugin::CORE_API_VERSION),
                config_dir.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null()),
                data_dir.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null()),
                std::ptr::null_mut(), // debug callback context
                Some(debug_callback),
                std::ptr::null_mut(), // state callback data
                None, // state callback fn
            );
            if r != m64p_error_M64ERR_SUCCESS {
                return Err(r.into());
            }
        }

        // Disallow dangling pointers in above call
        drop(config_dir);
        drop(data_dir);

        Ok(Mupen {
            core: Arc::new(self),
            plugins: Vec::with_capacity(4),
            is_rom_open: false,
        })
    }
}

/// Logging callback for plugins.
extern "C" fn debug_callback(
    _: *mut std::os::raw::c_void,
    level: std::os::raw::c_int,
    message: *const std::os::raw::c_char,
) {
    let level = level as m64p_msg_level;
    let message = unsafe { CStr::from_ptr(message).to_string_lossy() };

    #[allow(non_upper_case_globals)]
    match level {
        m64p_msg_level_M64MSG_INFO => log::info!("{}", message),
        m64p_msg_level_M64MSG_STATUS => log::info!("{}", message),
        m64p_msg_level_M64MSG_WARNING => log::warn!("{}", message),
        m64p_msg_level_M64MSG_ERROR => log::error!("{}", message),
        m64p_msg_level_M64MSG_VERBOSE => log::trace!("{}", message),
        _ => log::debug!("{}", message),
    }
}

impl Drop for Core {
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

impl Mupen {
    /// Attach a plugin, replacing any existing plugin of the same type.
    /// Plugins must be loaded in this order:
    /// 1. Video
    /// 2. Audio
    /// 3. Input
    /// 4. RSP
    pub fn attach_plugin(&mut self, plugin: Plugin) -> Result<(), Error> {
        // Without this check, we get an unhelpful InvalidState 
        if !self.is_rom_open() {
            return Err(Error::NoRomOpen);
        }

        // TODO: enforce plugin loading order

        let version = plugin.get_version()?;

        if let Some(f) = plugin.plugin_startup {
            log::trace!("plugin {:?} PluginStartup()", version.plugin_name);
            unsafe {
                f(self.core.lib, std::ptr::null_mut(), Some(debug_callback));
            };
        } else {
            return Err(Error::NoPluginStartup);
        }

        log::trace!("plugin {:?} CoreAttachPlugin()", version.plugin_name);

        let ret = unsafe {
            self.core.core_attach_plugin.unwrap()(version.plugin_type.into(), plugin.lib)
        };
        if ret != m64p_error_M64ERR_SUCCESS {
            return Err(ret.into());
        }

        log::trace!("attached plugin {:?} ok", version.plugin_name);

        self.plugins.push(plugin);

        Ok(())
    }

    // TODO: detach_plugin (by type?)

    pub fn is_rom_open(&self) -> bool {
        self.is_rom_open
    }

    /// Load an in-memory ROM into the core. It must be uncompressed but may be of any byte-order (v64, z64, n64).
    pub fn open_rom(&mut self, rom: &mut [u8]) -> Result<(), Error> {
        if self.is_rom_open() {
            self.close_rom()?
        }

        let ret = unsafe {
            // Makes a copy of `rom` internally - we're not giving it ownership.
            self.core.core_do_command.unwrap()(
                m64p_command_M64CMD_ROM_OPEN,
                rom.len() as i32,
                rom.as_ptr() as *mut std::os::raw::c_void,
            )
        };
        if ret != m64p_error_M64ERR_SUCCESS {
            Err(ret.into())
        } else {
            self.is_rom_open = true;
            Ok(())
        }
    }

    /// Execute the ROM. Blocking until the ROM is closed.
    pub fn execute(&self) -> Result<(), Error> {
        let ret = unsafe { self.core.core_do_command.unwrap()(m64p_command_M64CMD_EXECUTE, 0, std::ptr::null_mut()) };
        if ret != m64p_error_M64ERR_SUCCESS {
            Err(ret.into())
        } else {
            Ok(())
        }
    }

    /// Stop ROM execution.
    pub fn stop(&self) -> Result<(), Error> {
        let ret = unsafe { self.core.core_do_command.unwrap()(m64p_command_M64CMD_STOP, 0, std::ptr::null_mut()) };
        if ret != m64p_error_M64ERR_SUCCESS {
            Err(ret.into())
        } else {
            Ok(())
        }
    }

    /// Close the ROM.
    pub fn close_rom(&mut self) -> Result<(), Error> {
        let ret = unsafe { self.core.core_do_command.unwrap()(m64p_command_M64CMD_ROM_CLOSE, 0, std::ptr::null_mut()) };
        if ret != m64p_error_M64ERR_SUCCESS {
            Err(ret.into())
        } else {
            self.is_rom_open = false;
            Ok(())
        }
    }
}

impl Drop for Mupen {
    fn drop(&mut self) {
        // Shut down the core
        if let Some(f) = self.core.core_shutdown {
            unsafe {
                let _ = f();
            }
        }

        // Shut down the plugins
        for plugin in self.plugins.iter_mut() {
            if let Some(f) = plugin.plugin_shutdown {
                unsafe {
                    let _ = f();
                }
            }
        }
    }
}

unsafe impl Send for Core {}
unsafe impl Sync for Core {}
