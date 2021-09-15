use std::env::consts::DLL_EXTENSION;
use std::io::prelude::*;
use std::fs::File;

use mupen64plus::{Core, Plugin};
use mupen64plus::core::debug::Breakpoint;

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

    if let Ok(debug) = mupen.debug() {
        // When debug() is used, the emulator starts paused. Unpause it.
        let d = debug.clone(); // This is cheap - debug uses reference-counting.
        debug.on_init(Box::new(move || {
            println!("Starting emulation!");
            d.run().unwrap();
        }));

        // Add some breakpoints
        debug.add_breakpoint(0x800FB4A8);
        debug.add_breakpoint(
            Breakpoint::range(0..=u32::MAX).read()
        );

        // on_update is called whenever a breakpoint is hit or the emulation is stepped.
        let d = debug.clone();
        debug.on_update(Box::new(move |pc| {
            // Print out the instruction and registers.
            let (op, args) = d.disassemble(d.read_u32(pc), pc);
            println!("hit breakpoint at {:#X}", pc);
            println!("{} {}", op, args);
            println!("{:#X?}", d.registers());

            // We hit a breakpoint, so emulation was paused. Unpause it.
            d.run().unwrap();
        }));
    } else {
        eprintln!("Mupen64Plus core does not support debugging");
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
