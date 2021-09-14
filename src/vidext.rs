use mupen64plus_sys::*;
use std::os::raw::*;
use std::ffi::CStr;
use bitflags::bitflags;
use crate::MupenError;

pub type GLProc = *const c_void;
pub type GLAttr = m64p_GLattr;

pub trait Video {
    /// Initialize the video extension.
    /// This is called by [crate::core::Mupen::open_rom()].
    fn init() -> Result<(), MupenError> {
        Ok(())
    }

    /// Close any open rendering window and shut down the video system.
    /// This is called by [crate::core::Mupen::close_rom()].
    fn quit() -> Result<(), MupenError> {
        Ok(())
    }

    /// This function is used to enumerate the available resolution(s) for fullscreen video.
    /// `max_len` is the suggested number of resolutions to return.
    fn get_fullscreen_sizes(max_len: usize) -> Result<(), MupenError>;

    /// This function is used to enumerate the available refresh rate(s) for a given screen size.
    /// `max_len` is the suggested number of refresh rates to return.
    fn get_refresh_rates(_screen_size: ScreenSize, _max_len: usize) -> Result<(), MupenError> {
        return Err(MupenError::Unsupported);
    }

    /// This function creates a rendering window or switches into a fullscreen video mode. Any desired OpenGL attributes should be set before calling this function.
    fn set_video_mode(
        width: i32,
        height: i32,
        refresh_rate: Option<i32>,
        bits_per_pixel: BitsPerPixel,
        video_mode: VideoMode,
        flags: VideoFlags,
    ) -> Result<(), MupenError>;

    /// This function is used to get a pointer to an OpenGL extension function.
    fn gl_get_proc_address(proc_name: &str) -> GLProc;

    /// This function is used to set certain OpenGL attributes which must be specified before creating the rendering window with `set_video_mode`.
    fn gl_set_attribute(attr: GLAttr, value: i32) -> Result<(), MupenError>;

    /// This function may be used to check that OpenGL attributes were successfully set to the rendering window after the `set_video_mode` function call.
    fn gl_get_attribute(attr: GLAttr) -> Result<i32, MupenError>;

    /// This function is used to swap the front/back buffers after rendering an output video frame.
    fn gl_swap_buffers() -> Result<(), MupenError>;

    /// On some platforms (for instance, iOS) the default framebuffer object
    /// depends on the surface being rendered to, and might be different from 0.
    fn gl_get_default_framebuffer() -> u32 {
        0
    }

    /// This function is used to set the desired window title.
    fn set_caption(_title: &str) -> Result<(), MupenError> {
        // Ignore it.
        Ok(())
    }

    /// This function toggles between fullscreen and windowed rendering modes.
    fn toggle_fullscreen() -> Result<(), MupenError> {
        Err(MupenError::Unsupported)
    }

    /// This function is called when the video plugin has resized its OpenGL output viewport in response to a ResizeVideoOutput() call, and requests that the window manager update the OpenGL rendering window size to match. If a front-end application does not support resizable windows and never sets the M64CORE_VIDEO_SIZE core variable with the M64CMD_CORE_STATE_SET command, then this function should not be called.
    fn resize_window(width: i32, height: i32) -> Result<(), MupenError>;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
}

impl From<m64p_2d_size> for ScreenSize {
    fn from(s: m64p_2d_size) -> Self {
        // Safety: the types are structurally identical; they just have different field names
        unsafe {
            std::mem::transmute(s)
        }
    }
}

bitflags! {
    pub struct VideoFlags: m64p_video_flags {
        const SUPPORT_RESIZING = m64p_video_flags_M64VIDEOFLAG_SUPPORT_RESIZING;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    Fullscreen,
    Windowed,
}

impl From<m64p_video_mode> for VideoMode {
    fn from(m: m64p_video_mode) -> Self {
        #[allow(non_upper_case_globals)]
        match m {
            m64p_video_mode_M64VIDEO_WINDOWED => VideoMode::Windowed,
            m64p_video_mode_M64VIDEO_FULLSCREEN => VideoMode::Fullscreen,
            _ => panic!("invalid m64p_video_mode {}", m),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitsPerPixel {
    Bits16,
    Bits24,
    Bits32,
}

impl From<i32> for BitsPerPixel {
    fn from(b: i32) -> Self {
        match b {
            16 => BitsPerPixel::Bits16,
            24 => BitsPerPixel::Bits24,
            _ => BitsPerPixel::Bits32,
            //_ => panic!("invalid bits per pixel {}", b),
        }
    }
}

impl Into<i32> for BitsPerPixel {
    fn into(self) -> i32 {
        match self {
            BitsPerPixel::Bits16 => 16,
            BitsPerPixel::Bits24 => 24,
            BitsPerPixel::Bits32 => 32,
        }
    }
}

fn cvt_result<T>(result: Result<T, MupenError>) -> m64p_error {
    match result {
        Ok(_) => m64p_error_M64ERR_SUCCESS,
        Err(MupenError::AlreadyInit) => m64p_error_M64ERR_ALREADY_INIT,
        Err(MupenError::NotInit) => m64p_error_M64ERR_NOT_INIT,
        Err(MupenError::Incompatible) => m64p_error_M64ERR_INCOMPATIBLE,
        Err(MupenError::InputAssert) => m64p_error_M64ERR_INPUT_ASSERT,
        Err(MupenError::InputInvalid) => m64p_error_M64ERR_INPUT_INVALID,
        Err(MupenError::InputNotFound) => m64p_error_M64ERR_INPUT_NOT_FOUND,
        Err(MupenError::NoMemory) => m64p_error_M64ERR_NO_MEMORY,
        Err(MupenError::Files) => m64p_error_M64ERR_FILES,
        Err(MupenError::Internal) => m64p_error_M64ERR_INTERNAL,
        Err(MupenError::InvalidState) => m64p_error_M64ERR_INVALID_STATE,
        Err(MupenError::PluginFail) => m64p_error_M64ERR_PLUGIN_FAIL,
        Err(MupenError::SystemFail) => m64p_error_M64ERR_SYSTEM_FAIL,
        Err(MupenError::Unsupported) => m64p_error_M64ERR_UNSUPPORTED,
        Err(MupenError::WrongType) => m64p_error_M64ERR_WRONG_TYPE,
    }
}

pub(crate) fn override_video<V: Video>() -> m64p_video_extension_functions {
    m64p_video_extension_functions {
        Functions: 14,
        VidExtFuncInit: Some(func_init::<V>),
        VidExtFuncQuit: Some(func_quit::<V>),
        VidExtFuncListModes: Some(func_list_modes::<V>),
        VidExtFuncListRates: Some(func_list_rates::<V>),
        VidExtFuncSetMode: Some(func_set_mode::<V>),
        VidExtFuncSetModeWithRate: Some(func_set_mode_with_rate::<V>),
        VidExtFuncGLGetProc: Some(func_gl_get_proc::<V>),
        VidExtFuncGLSetAttr: Some(func_gl_set_attr::<V>),
        VidExtFuncGLGetAttr: Some(func_gl_get_attr::<V>),
        VidExtFuncGLSwapBuf: Some(func_gl_swap_buf::<V>),
        VidExtFuncSetCaption: Some(func_set_caption::<V>),
        VidExtFuncToggleFS: Some(func_toggle_fs::<V>),
        VidExtFuncResizeWindow: Some(func_resize_window::<V>),
        VidExtFuncGLGetDefaultFramebuffer: Some(func_gl_get_default_framebuffer::<V>),
    }
}

unsafe extern "C" fn func_init<V: Video>() -> m64p_error {
    cvt_result(V::init())
}

unsafe extern "C" fn func_quit<V: Video>() -> m64p_error {
    cvt_result(V::quit())
}

// TODO
unsafe extern "C" fn func_list_modes<V: Video>(array: *mut m64p_2d_size, len: *mut c_int) -> m64p_error {
    let max_len = *len as usize;
    let result = V::get_fullscreen_sizes(max_len);
    /*if let Ok(modes) = result {
        array = modes.as_mut_ptr() as *mut m64p_2d_size;
        if max_len == 0 || modes.len() < max_len {
            *len = modes.len() as c_int;
        }
    }*/
    cvt_result(result)
}

// TODO
// len is both input (max no of rates to list) and output (no of rates in array)
unsafe extern "C" fn func_list_rates<V: Video>(size: m64p_2d_size, len: *mut c_int, array: *mut c_int) -> m64p_error {
    let max_len = *len;
    let result = V::get_refresh_rates(size.into(), max_len as usize);
    /*if let Ok(rates) = result {
        *len = rates.len();
        if *len > max_len {
            *len = max_len;
        }

        for i in 0..*len {
            array[i] = rates[i].into();
        }
    }*/
    cvt_result(result)
}

unsafe extern "C" fn func_set_mode<V: Video>(
    width: c_int,
    height: c_int,
    bits_per_pixel: c_int,
    video_mode: c_int, 
    flags: c_int,
) -> m64p_error {
    cvt_result(V::set_video_mode(width, height, None, bits_per_pixel.into(), VideoMode::from(video_mode as u32), VideoFlags::from_bits_truncate(flags as u32)))
}

unsafe extern "C" fn func_set_mode_with_rate<V: Video>(
    width: c_int,
    height: c_int,
    refresh_rate: c_int,
    bits_per_pixel: c_int,
    video_mode: c_int, 
    flags: c_int,
) -> m64p_error {
    cvt_result(V::set_video_mode(width, height, Some(refresh_rate), bits_per_pixel.into(), VideoMode::from(video_mode as u32), VideoFlags::from_bits_truncate(flags as u32)))
}

unsafe extern "C" fn func_gl_get_proc<V: Video>(proc_name: *const c_char) -> m64p_function {
    let proc_name = CStr::from_ptr(proc_name).to_str().ok()?;
    let ptr = V::gl_get_proc_address(proc_name);

    if ptr.is_null() {
        None
    } else {
        Some(std::mem::transmute(ptr))
    }
}

unsafe extern "C" fn func_gl_set_attr<V: Video>(attr: m64p_GLattr, value: c_int) -> m64p_error {
    cvt_result(V::gl_set_attribute(attr, value))
}

unsafe extern "C" fn func_gl_get_attr<V: Video>(attr: m64p_GLattr, out: *mut c_int) -> m64p_error {
    cvt_result({
        let result = V::gl_get_attribute(attr);
        if let Ok(value) = result {
            *out = value;
        }
        result
    })
}

unsafe extern "C" fn func_gl_swap_buf<V: Video>() -> m64p_error {
    cvt_result(V::gl_swap_buffers())
}

unsafe extern "C" fn func_set_caption<V: Video>(title: *const c_char) -> m64p_error {
    cvt_result(if let Ok(title) = CStr::from_ptr(title).to_str() {
        V::set_caption(title)
    } else {
        Err(MupenError::InputInvalid)
    })
}

unsafe extern "C" fn func_toggle_fs<V: Video>() -> m64p_error {
    cvt_result(V::toggle_fullscreen())
}

unsafe extern "C" fn func_resize_window<V: Video>(w: i32, h: i32) -> m64p_error {
    cvt_result(V::resize_window(w, h))
}

unsafe extern "C" fn func_gl_get_default_framebuffer<V: Video>() -> u32 {
    V::gl_get_default_framebuffer()
}
