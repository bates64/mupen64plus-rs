use std::env::consts::DLL_EXTENSION;
use std::io::prelude::*;
use std::fs::File;
use std::ffi::CString;
use sdl2::sys::*;

use mupen64plus::{Core, Plugin, MupenError};
use mupen64plus::vidext::{Video, VideoMode, VideoFlags, BitsPerPixel, ScreenSize, GLAttr, GLProc};

struct CustomVideo;

impl Video for CustomVideo {
    fn get_fullscreen_sizes(max_len: usize) -> Result<(), MupenError> {
        Err(MupenError::Unsupported)
    }

    fn set_video_mode(
        width: i32,
        height: i32,
        refresh_rate: Option<i32>,
        bits_per_pixel: BitsPerPixel,
        _video_mode: VideoMode,
        _flags: VideoFlags,
    ) -> Result<(), MupenError> {
        // TODO...?
        Ok(())
    }

    fn gl_get_proc_address(proc_name: &str) -> GLProc {
        unsafe {
            let proc_name = CString::new(proc_name).unwrap();
            SDL_GL_GetProcAddress(proc_name.as_ptr()) as *const std::ffi::c_void
        }
    }

    fn gl_set_attribute(attr: GLAttr, value: i32) -> Result<(), MupenError> {
        Err(MupenError::Unsupported)
    }

    fn gl_get_attribute(attr: GLAttr) -> Result<i32, MupenError> {
        Err(MupenError::Unsupported)
    }

    fn gl_swap_buffers() -> Result<(), MupenError> {
        Err(MupenError::Unsupported)
    }

    fn resize_window(width: i32, height: i32) -> Result<(), MupenError> {
        Err(MupenError::Unsupported)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let path = format!("{}/libs", env!("CARGO_MANIFEST_DIR"));

    // Load the core library.
    let core = Core::load_from_directory(&path)
        .or_else(|_| Core::load_from_system())?;

    // Launch the core and load configuration.
    let mut mupen = core.start(Some(&path), Some(&path))?;

    let sdl_context = sdl2::init()?;
    let sdl_video = sdl_context.video()?;
    let sdl_window = sdl_video.window("mupen64plus", 800, 600)
        .opengl()
        .position_centered()
        .build()
        .unwrap();

    //mupen.use_video_extension::<CustomVideo>();

    // Load the test ROM and give it to mupen64plus.
    mupen.open_rom(&mut load_rom()?)?;

    // Load the plugins - the order is important.
    for name in &["video-glide64mk2", "audio-sdl", "input-sdl", "rsp-hle"] {
        let p = format!("{}/mupen64plus-{}.{}", &path, name, DLL_EXTENSION);
        mupen.attach_plugin(Plugin::load_from_path(p)?)?;
    }

    // Run the ROM!
    mupen.execute()?;

    let mut event_pump = sdl_context.event_pump()?;
    'main: loop {
        use sdl2::event::Event;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main,
                _ => {}
            }
        }
    }

    Ok(())
}

// Load the test ROM file into memory.
fn load_rom() -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(format!("{}/examples/m64p_test_rom.v64", env!("CARGO_MANIFEST_DIR")))?;
    let mut buf = Vec::with_capacity(file.metadata()?.len() as usize);
    file.read_to_end(&mut buf)?;
    Ok(buf)
}
