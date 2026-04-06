# Unsafe Allowlist

Remaining `unsafe` in `safe/` is limited to documented FFI shims, raw OS/platform glue, generated ABI surfaces, performance-critical render code, and ABI-level tests.

Phase 10 keeps this file as the source of truth for `cargo run --manifest-path safe/Cargo.toml -p xtask -- unsafe-audit`. Every remaining `unsafe` file must match one of the rules below, and the generated audit report summarizes the covered files by category.

- `safe/sdl2main/src/lib.rs` [ffi]: Exports the `SDL_main` compatibility shim consumed from C toolchains.
- `safe/src/abi/generated_types.rs` [generated]: Bindgen-generated foreign types, extern declarations, and layout assertions.
- `safe/src/audio/*.rs` [ffi]: Exported C ABI wrappers, raw audio buffers, and decoder/stream pointer interop.
- `safe/src/core/*.rs` [ffi]: Exported C ABI wrappers around libc, threading, allocation, logging, and platform runtime entrypoints.
- `safe/src/events/*.rs` [ffi]: Exported C ABI wrappers around event callbacks and raw SDL pointer types.
- `safe/src/exports/generated_linux_stubs.rs` [generated]: Generated aborting ABI stubs for unimplemented exports.
- `safe/src/input/*.rs` [ffi]: Exported C ABI wrappers over joystick/controller/haptic/sensor raw handles.
- `safe/src/input/linux/*.rs` [os]: Direct Linux device, evdev, udev, and ioctl interaction.
- `safe/src/main_archive.rs` [ffi]: Linker-visible archive helpers and exported C shims.
- `safe/src/render/core.rs` [ffi]: Renderer ABI surface and raw texture/renderer pointer ownership.
- `safe/src/render/gl.rs` [ffi]: Raw GL and Metal function-pointer forwarding across the SDL ABI.
- `safe/src/render/gles.rs` [performance]: Raw GLES proc-table access and texture upload kernels kept close to the ABI and hot paths.
- `safe/src/render/local.rs` [performance]: Local software renderer keeps raw SDL surface, texture, and window pointers in hot-path drawing code so the packaged fallback works without a host SDL runtime.
- `safe/src/render/software.rs` [ffi]: Raw renderer and surface pointer forwarding for the software backend.
- `safe/src/testsupport/*.rs` [ffi]: SDL_test support ports mirroring upstream pointer-heavy helpers.
- `safe/src/video/*.rs` [ffi]: Exported C ABI wrappers over windows, displays, surfaces, pixels, clipboard, and platform video handles.
- `safe/src/video/linux/*.rs` [os]: Direct Linux window-system, KMS/DRM, Wayland/X11, and IME integration.
- `safe/tests/*.rs` [tests]: Integration tests intentionally exercise ABI-level entrypoints, raw pointers, and external runtimes.
- `safe/tests/common/*.rs` [tests]: Shared ABI-level test harness helpers.
- `safe/xtask/src/contracts.rs` [generated]: Contract/codegen verification parses generated Rust and C signatures plus emitted unsafe export stubs.
