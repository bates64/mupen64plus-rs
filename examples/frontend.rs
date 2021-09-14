use std::env::consts::DLL_EXTENSION;
use std::io::prelude::*;
use std::fs::File;

use mupen64plus::{Core, Plugin};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let path = format!("{}/libs", env!("CARGO_MANIFEST_DIR"));

    // Load the core library.
    let core = Core::load_from_directory(&path)
        .or_else(|_| Core::load_from_system())?;

    // Launch the core and load configuration.
    let mut mupen = core.start(Some(&path), Some(&path))?;

    // Load the test ROM and give it to mupen64plus.
    mupen.open_rom(&mut load_rom()?)?;

    // Load the plugins - the order is important.
    for name in &["video-glide64mk2", "audio-sdl", "input-sdl", "rsp-hle"] {
        let p = format!("{}/mupen64plus-{}.{}", &path, name, DLL_EXTENSION);
        mupen.attach_plugin(Plugin::load_from_path(p)?)?;
    }

    // Run the ROM!
    mupen.execute()?;

    Ok(())
}

// Load the test ROM file into memory.
fn load_rom() -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(format!("{}/examples/m64p_test_rom.v64", env!("CARGO_MANIFEST_DIR")))?;
    let mut buf = Vec::with_capacity(file.metadata()?.len() as usize);
    file.read_to_end(&mut buf)?;
    Ok(buf)
}
