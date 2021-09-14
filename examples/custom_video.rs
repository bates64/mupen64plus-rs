use std::env::consts::DLL_EXTENSION;
use std::io::prelude::*;
use std::fs::File;
use std::cell::RefCell;

use mupen64plus::{Core, Plugin, MupenError};
use mupen64plus::vidext::{Video, VideoMode, VideoFlags, BitsPerPixel, GLAttr, GLProc};

struct CustomVideo;

/// Custom video implementation state. We have to store this in a global variable because
/// there is nothing in the Mupen64Plus API to pass a context pointer around :(
struct CustomVideoCtx {
    video: sdl2::VideoSubsystem,
    window: Option<sdl2::video::Window>,
}

// Despite being marked thread-local, VIDEO_CTX will only ever be accessed on the main thread.
thread_local! {
    static VIDEO_CTX: RefCell<Option<CustomVideoCtx>> = RefCell::new(Default::default());
    static SDL_CTX: RefCell<sdl2::Sdl> = RefCell::new(sdl2::init().unwrap());
}

impl Video for CustomVideo {
    fn init() -> Result<(), MupenError> {
        SDL_CTX.with(|sdl| {
            VIDEO_CTX.with(|ctx| {
                let video = sdl.borrow().video().unwrap();

                ctx.replace(Some(CustomVideoCtx {
                    window: None,
                    video,
                }));
            });
        });
        Ok(())
    }

    fn quit() -> Result<(), MupenError> {
        VIDEO_CTX.with(|ctx| {
            ctx.take();
        });
        Ok(())
    }

    fn get_fullscreen_sizes(_: usize) -> Result<(), MupenError> {
        Err(MupenError::Unsupported)
    }

    fn set_video_mode(
        width: i32,
        height: i32,
        _refresh_rate: Option<i32>,
        _bits_per_pixel: BitsPerPixel,
        video_mode: VideoMode,
        _flags: VideoFlags,
    ) -> Result<(), MupenError> {
        dbg!(width, height, _refresh_rate, _bits_per_pixel, video_mode, _flags);
        VIDEO_CTX.with(|ctx| {
            let mut ctx = ctx.borrow_mut();
            let ctx = ctx.as_mut().unwrap();

            let mut window = ctx.video.window("mupen64plus", width as u32, height as u32);
        
            window.opengl();
            window.resizable();

            if video_mode == VideoMode::Fullscreen {
                window.fullscreen();
            }

            let window = window.build().unwrap();
            let gl_context = window.gl_create_context().unwrap();
            window.gl_make_current(&gl_context).unwrap();

            ctx.window = Some(window);
        });
        Ok(())
    }

    // XXX: is this ever called? why not?
    fn gl_get_proc_address(proc_name: &str) -> GLProc {
        dbg!("gl_get_proc_address {}", proc_name);

        VIDEO_CTX.with(|ctx| {
            let ctx = ctx.borrow();
            let ctx = ctx.as_ref().unwrap();

            ctx.video.gl_get_proc_address(proc_name) as *const _
        })
    }

    fn gl_set_attribute(attr: GLAttr, value: i32) -> Result<(), MupenError> {
        VIDEO_CTX.with(|ctx| {
            let ctx = ctx.borrow();
            let ctx = ctx.as_ref().unwrap();

            match attr {
                0 => ctx.video.gl_attr().set_red_size(value as _),
                1 => ctx.video.gl_attr().set_green_size(value as _),
                2 => ctx.video.gl_attr().set_blue_size(value as _),
                3 => ctx.video.gl_attr().set_alpha_size(value as _),
                4 => ctx.video.gl_attr().set_buffer_size(value as _),
                5 => ctx.video.gl_attr().set_depth_size(value as _),
                6 => ctx.video.gl_attr().set_stencil_size(value as _),
                7 => ctx.video.gl_attr().set_double_buffer(value != 0),
                _ => return Err(MupenError::Unsupported),
            };

            Ok(())
        })
    }

    fn gl_get_attribute(_attr: GLAttr) -> Result<i32, MupenError> {
        Err(MupenError::Unsupported)
    }

    fn gl_swap_buffers() -> Result<(), MupenError> {
        VIDEO_CTX.with(|ctx| {
            let ctx = ctx.borrow();
            let ctx = ctx.as_ref().unwrap();

            ctx.window.as_ref().unwrap().gl_swap_window();
        });
        Ok(())
    }

    fn resize_window(width: i32, height: i32) -> Result<(), MupenError> {
        VIDEO_CTX.with(|ctx| {
            let mut ctx = ctx.borrow_mut();
            let ctx = ctx.as_mut().unwrap();

            ctx.window.as_mut().unwrap().set_size(width as u32, height as u32).unwrap();
        });
        Ok(())
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

    // Use our custom video implementation.
    mupen.use_video_extension::<CustomVideo>();

    // Load the test ROM and give it to mupen64plus.
    mupen.open_rom(&mut load_rom()?)?;

    // Load the plugins - the order is important.
    for name in &["audio-sdl", "input-sdl", "rsp-hle"] {
        let p = format!("{}/mupen64plus-{}.{}", &path, name, DLL_EXTENSION);
        mupen.attach_plugin(Plugin::load_from_path(p)?)?;
    }

    SDL_CTX.with(|sdl| {
        let mut event_pump = sdl.borrow().event_pump()?;

        // Run the ROM!
        mupen.execute()?;

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
    })
}

// Load the test ROM file into memory.
fn load_rom() -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(format!("{}/examples/m64p_test_rom.v64", env!("CARGO_MANIFEST_DIR")))?;
    let mut buf = Vec::with_capacity(file.metadata()?.len() as usize);
    file.read_to_end(&mut buf)?;
    Ok(buf)
}
