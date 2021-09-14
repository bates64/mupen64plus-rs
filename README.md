# mupen64plus

[![crates.io badge][crates-io-badge]][crates-io-url]
[![docs.rs badge][docs-rs-badge]][docs-rs-url]

[crates-io-badge]: https://img.shields.io/crates/v/mupen64plus.svg
[crates-io-url]: https://crates.io/crates/mupen64plus
[docs-rs-badge]: https://docs.rs/mupen64plus/badge.svg
[docs-rs-url]: https://docs.rs/mupen64plus

High-level Rust bindings to the [Mupen64Plus Core API](https://mupen64plus.org/wiki/index.php?title=Mupen64Plus_v2.0_Core_API_v1.0#Core_API).

```rs
use mupen64plus::{Core, Plugin};

let core = Core::load_from_directory(&path)
    .or_else(|_| Core::load_from_system())?;

let mut mupen = core.start(Some(&path), Some(&path))?;

mupen.open_rom(&mut load_rom()?)?;

for name in &["video-glide64mk2", "audio-sdl", "input-sdl", "rsp-hle"] {
    let p = format!("mupen64plus-{}.{}", name, std::env::consts::DLL_EXTENSION);
    mupen.attach_plugin(Plugin::load_from_path(p)?)?;
}

mupen.execute()?;
```

A more detailed example can be found [here](examples/frontend.rs), which can be run with:
```bash
cargo run --example frontend
```
