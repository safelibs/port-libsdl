## High-level architecture.
The phase structure was read from `.plan/workflow-structure.yaml`: `impl_phase_01_document_port` is an `implement` phase with fixed `bounce_target: null`, followed in order by `check_phase_01_port_structure`, `check_phase_01_unsafe_inventory`, `check_phase_01_ffi_dependencies`, `check_phase_01_remaining_issues`, and `check_phase_01_build_and_commit`. Baseline started at git commit `1328c74dae007a00bec58ff81b678e241af56bbc`; `safe/PORT.md` was absent, and the optional upgrade report was absent at implementation start.

**Workspace and packages.**
| Area | Evidence | Port fact |
| --- | --- | --- |
| Workspace root | `safe/Cargo.toml:1` | `members = [".", "xtask", "sdl2-test", "sdl2main"]` at `safe/Cargo.toml:2`; resolver 2 at `safe/Cargo.toml:3`. |
| Main package | `safe/Cargo.toml:5` | `safe-sdl` at `safe/Cargo.toml:6`, version `0.1.0`, edition 2021. |
| Library outputs | `safe/Cargo.toml:12` | Library output name is `safe_sdl` at `safe/Cargo.toml:13`; crate types are `cdylib`, `staticlib`, and `rlib` at `safe/Cargo.toml:14`. |
| Feature gate | `safe/Cargo.toml:17` | `host-video-tests` is declared at `safe/Cargo.toml:18` for tests that need host video infrastructure. |
| Test helper static library | `safe/sdl2-test/Cargo.toml:1` | `safe-sdl2-test` builds static library `safe_sdl2_test` with `libc` and build-time `cc` at `safe/sdl2-test/Cargo.toml:7` through `safe/sdl2-test/Cargo.toml:15`. |
| SDL main static library | `safe/sdl2main/Cargo.toml:1` | `safe-sdl2main` builds static library `safe_sdl2main` and depends on `libc` at `safe/sdl2main/Cargo.toml:7` through `safe/sdl2main/Cargo.toml:12`. |
| xtask tooling crate | `safe/xtask/Cargo.toml:1` | `xtask` owns capture, audit, ABI, staging, dependent, and final-check tooling; direct dependencies are declared at `safe/xtask/Cargo.toml:7` through `safe/xtask/Cargo.toml:13`. |

The public Rust module tree starts at `safe/src/lib.rs:9`: generated ABI types, `audio`, `core`, `events`, `input`, `security::checked_math`, `dynapi::generated`, `render`, `video`, `main_archive`, and `exports::generated_linux_stubs` are exposed through `safe/src/lib.rs:9` through `safe/src/lib.rs:37`.

**Directory map.**
| Path | Purpose |
| --- | --- |
| `safe/src/abi` | Generated Rust declarations from upstream public headers; `safe/src/lib.rs:9` exposes `abi::generated_types`. |
| `safe/src/audio` | Safe port of SDL audio device, stream, conversion, resampling, WAVE, and backend modules. |
| `safe/src/core` | Core SDL runtime behavior: error, logging, allocation, RWops, init, timers, filesystem, threads, mutexes, dynload, platform, and C variadic shim source. |
| `safe/src/dynapi` | Generated dynapi slot table exposed by `safe/src/lib.rs:21` through `safe/src/lib.rs:23`. |
| `safe/src/events` | Keyboard, mouse, touch, gesture, and queue behavior exported through the SDL ABI. |
| `safe/src/exports` | Generated Linux export stubs and the abort helper exposed by `safe/src/lib.rs:29` through `safe/src/lib.rs:37`. |
| `safe/src/input` | Game controller, joystick, haptic, HIDAPI, sensor, GUID, and Linux evdev/udev support. |
| `safe/src/render` | Renderer dispatch, local software renderer state, GL/GLES integration, and software renderer forwarding. |
| `safe/src/security` | Checked arithmetic support for surface, copy, pitch, and allocation validation via `safe/src/lib.rs:18` through `safe/src/lib.rs:20`. |
| `safe/src/testsupport` | Rust port of SDL test support routines used by upstream-style tests and installed test artifacts. |
| `safe/src/video` | Window, display, surface, blit, pixels, EGL, Vulkan, Linux video, clipboard, BMP, shape, syswm, offscreen, and dummy video support. |
| `safe/sdl2-test` | Companion static library for SDL2 test ABI helpers; its build script compiles `src/variadic_shims.c` with `cc::Build` at `safe/sdl2-test/build.rs:9`. |
| `safe/sdl2main` | Companion static library exporting the SDL main compatibility symbol at `safe/sdl2main/src/lib.rs:7`. |
| `safe/xtask` | Repository-local tooling for ABI checks, contract capture, staging install, original test handling, unsafe audit, performance, dependents, and final checks. |
| `safe/generated` | Checked-in generated contracts and manifests including symbol, dynapi, header, install, test map, dependent, CVE, performance, and final report data. |
| `safe/docs` | Documentation inputs such as `safe/docs/unsafe-allowlist.md`, consumed by unsafe audit. |
| `safe/tests` | Integration, security, validator regression, host-gated, evdev fixture, and upstream-port test coverage. |
| `safe/debian` | Debian packaging source; `safe/debian/rules:30` builds release Cargo artifacts and `safe/debian/rules:35` stages a full install through `xtask stage-install --mode full`. |

**ABI, build, and packaging surface.**
| Evidence area | Finding |
| --- | --- |
| Symbol manifest | `safe/generated/linux_symbol_manifest.json` records SONAME `libSDL2-2.0.so.0` and 839 Linux symbols. |
| Release artifact | `cargo build --manifest-path safe/Cargo.toml --release` completed; `/tmp/libsdl-port-defined-symbols.txt` contains 839 defined dynamic symbols. |
| ABI check | `cargo run --manifest-path safe/Cargo.toml -p xtask -- abi-check --library safe/target/release/libsafe_sdl.so --require-soname libSDL2-2.0.so.0` passed. |
| SONAME and dynamic section | `/tmp/libsdl-port-readelf-dynamic.txt` reports SONAME `libSDL2-2.0.so.0` and NEEDED entries `libgcc_s.so.1, libm.so.6, libc.so.6, ld-linux-x86-64.so.2`. |
| Versioned exports | `/tmp/libsdl-port-objdump-T.txt` contains 1045 rows including SDL exports in version `Base`; representative exported symbols in `/tmp/libsdl-port-defined-symbols.txt` include `SDL_Init`, `SDL_CreateWindow`, `SDL_iconv_open`, `SDL_ComposeCustomBlendMode`, and `SDL_DYNAPI_entry`. |
| Headers and install surface | `safe/generated/public_header_inventory.json` lists 91 public headers; `safe/generated/install_contract.json` drives the staged runtime, development, and installed-test layout. |

| Build/packaging item | Finding |
| --- | --- |
| Version script | `safe/build.rs:21` reads `safe/generated/linux_symbol_manifest.json`; `safe/build.rs:31` through `safe/build.rs:43` writes and links `libSDL2.version.map`. |
| SONAME | `safe/build.rs:43` emits `-Wl,--soname,<manifest soname>`, and readelf confirms `libSDL2-2.0.so.0`. |
| Dynamic loader link | `safe/build.rs:44` emits `cargo:rustc-link-lib=dylib=dl`; glibc folds the loader symbols into `libc.so.6` on this build, so readelf NEEDED does not show a separate `libdl.so`. |
| C variadic shim | `safe/build.rs:46` through `safe/build.rs:50` compiles `safe/src/core/phase2_variadic_shims.c` with `cc::Build`. |
| Debian package build | `safe/debian/rules:30` builds `safe-sdl`, `safe-sdl2-test`, and `safe-sdl2main` release artifacts; `safe/debian/rules:35` runs `xtask stage-install --generated safe/generated --original original --root safe/debian/tmp --mode full`. |
| cbindgen status | No `cbindgen` configuration or invocation was found in `/tmp/libsdl-port-build-tools.txt`; ABI declarations are generated by `bindgen` in `safe/xtask/src/contracts.rs:2699` through `safe/xtask/src/contracts.rs:2713`. |

`cargo metadata` wrote `/tmp/libsdl-port-cargo-metadata.json`, `cargo tree` wrote `/tmp/libsdl-port-cargo-tree.txt`, and the build-tool grep wrote `/tmp/libsdl-port-build-tools.txt`. The build-tool grep found `bindgen`, `pkg-config`, `cc::Build`, `crate-type`, and `cargo:rustc-link` references; a direct `rg -n 'cbindgen' /tmp/libsdl-port-build-tools.txt` check produced no matches.

## Where the unsafe Rust lives.
The checked-in audit report `safe/generated/reports/unsafe-audit.json:7` and `safe/generated/reports/unsafe-audit.json:8` reports 96 unsafe files and zero undocumented files. The current audit command also passed and wrote `/tmp/libsdl-port-unsafe-audit.json`; its zero-undocumented result is used as coverage evidence, while the exhaustive line inventory below is taken from `/tmp/libsdl-port-unsafe-source.txt` exactly as required. The line inventory has 2779 matches grouped by purpose.

| Purpose group | Line count | Justification used for rows |
| --- | --- | --- |
| C variadic shim references | 6 | Binds Rust code to the C shim required for SDL variadic formatting, error storage, and log callbacks that Rust cannot express directly. |
| OS and dynamic-loader glue | 41 | Calls Linux device APIs through raw descriptors, ioctl payloads, or C buffers required by SDL input compatibility. |
| allocation and string/memory helpers | 32 | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| external GL/EGL/Vulkan function pointers | 57 | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| generated bindgen ABI declarations | 861 | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| integration tests | 397 | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| public SDL ABI exports | 757 | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| raw pointer/object ownership | 61 | Owns or validates SDL audio device, stream, decoder, or buffer pointers crossing the C ABI. |
| renderer hot paths | 441 | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| test support | 124 | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| xtask code generation/audit support | 2 | Generation and audit tooling emits or matches unsafe Rust source text; this is tooling data rather than runtime FFI. |

**Exhaustive unsafe line inventory from `/tmp/libsdl-port-unsafe-source.txt`.**
| Location | Purpose group | One-sentence justification |
| --- | --- | --- |
| `safe/tests/xvfb_window_smoke.rs:25` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/render/software.rs:9` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/software.rs:11` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/software.rs:23` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/software.rs:43` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/software.rs:59` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/sdl2main/src/lib.rs:7` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/upstream_port_core.rs:46` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:62` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:72` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:82` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:94` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:118` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:163` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:192` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:221` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:231` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:250` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:275` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:291` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:342` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:377` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:402` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:432` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:456` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_core.rs:524` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/xtask/src/contracts.rs:2739` | xtask code generation/audit support | Generation and audit tooling emits or matches unsafe Rust source text; this is tooling data rather than runtime FFI. |
| `safe/xtask/src/contracts.rs:3155` | xtask code generation/audit support | Generation and audit tooling emits or matches unsafe Rust source text; this is tooling data rather than runtime FFI. |
| `safe/tests/security_bmp_parser.rs:38` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_bmp_parser.rs:48` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_bmp_parser.rs:62` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_bmp_parser.rs:77` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_bmp_parser.rs:88` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/render/local.rs:35` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:46` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:80` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:88` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:100` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:107` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:114` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:118` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:139` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:157` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:169` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:178` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:182` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:206` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:217` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:231` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:300` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:317` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:345` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:388` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:415` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:461` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:483` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:519` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:540` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:597` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:601` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:606` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:618` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:631` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:642` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:646` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:667` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:681` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:698` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:742` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:772` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:798` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:807` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:816` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:820` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:827` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:834` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:841` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:849` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:860` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:868` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:872` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:922` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1022` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1098` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1132` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1144` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1146` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1150` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1174` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1178` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1187` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1201` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1209` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1213` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1221` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1228` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1236` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1248` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1252` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1261` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1275` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1293` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1311` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1322` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1340` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1348` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1359` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1395` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1413` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1434` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1456` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1478` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1519` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1539` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1560` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1580` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1603` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1615` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1623` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1644` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1660` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1682` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1692` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1716` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1726` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1750` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1762` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1797` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1860` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1877` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1922` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1924` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1928` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1934` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1947` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1962` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1974` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1978` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1985` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/local.rs:1993` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/tests/upstream_port_surface.rs:42` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:52` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:65` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:85` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:93` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:105` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:184` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:251` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:323` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:354` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:377` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:436` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:498` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:545` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:567` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:611` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_surface.rs:644` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/render/gles.rs:163` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:164` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:165` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:183` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:184` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:185` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:186` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:187` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:188` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:199` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:212` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:223` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:240` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:257` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:278` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:411` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:467` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:480` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:490` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:540` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gles.rs:549` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/audio/wave.rs:565` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/wave.rs:625` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/original_apps_core.rs:45` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:49` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:66` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:77` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:89` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:108` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:116` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:129` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:153` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:177` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:200` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:226` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:245` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:263` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:287` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:302` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:334` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:358` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:360` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:368` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:404` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:422` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:434` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_core.rs:440` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/render/gl.rs:7` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:8` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:9` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:10` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:11` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:12` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:13` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:14` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:15` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:16` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:17` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:18` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:19` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:20` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:21` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:22` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:23` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:24` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:25` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:27` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:89` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/render/gl.rs:118` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:128` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:138` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:146` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:156` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:167` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:178` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:195` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:211` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:249` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:259` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:269` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:289` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:300` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:310` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:325` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:348` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:365` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:373` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/gl.rs:383` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/convert.rs:323` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/convert.rs:439` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/convert.rs:500` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/convert.rs:532` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/dependent_regressions.rs:128` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/input/linux/evdev.rs:183` | OS and dynamic-loader glue | Calls Linux device APIs through raw descriptors, ioctl payloads, or C buffers required by SDL input compatibility. |
| `safe/src/input/linux/evdev.rs:192` | OS and dynamic-loader glue | Calls Linux device APIs through raw descriptors, ioctl payloads, or C buffers required by SDL input compatibility. |
| `safe/src/input/linux/evdev.rs:563` | OS and dynamic-loader glue | Calls Linux device APIs through raw descriptors, ioctl payloads, or C buffers required by SDL input compatibility. |
| `safe/src/input/linux/evdev.rs:583` | OS and dynamic-loader glue | Calls Linux device APIs through raw descriptors, ioctl payloads, or C buffers required by SDL input compatibility. |
| `safe/src/input/linux/evdev.rs:685` | OS and dynamic-loader glue | Calls Linux device APIs through raw descriptors, ioctl payloads, or C buffers required by SDL input compatibility. |
| `safe/src/input/linux/evdev.rs:694` | OS and dynamic-loader glue | Calls Linux device APIs through raw descriptors, ioctl payloads, or C buffers required by SDL input compatibility. |
| `safe/src/input/linux/evdev.rs:707` | OS and dynamic-loader glue | Calls Linux device APIs through raw descriptors, ioctl payloads, or C buffers required by SDL input compatibility. |
| `safe/src/audio/stream.rs:28` | raw pointer/object ownership | Owns or validates SDL audio device, stream, decoder, or buffer pointers crossing the C ABI. |
| `safe/src/audio/stream.rs:105` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/stream.rs:157` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/stream.rs:216` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/stream.rs:248` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/stream.rs:262` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/stream.rs:281` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/stream.rs:295` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:27` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:28` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:29` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:30` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:31` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:32` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:33` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:34` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:35` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:36` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:37` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:38` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:39` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:40` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:41` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:42` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:43` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:44` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:45` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:46` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:47` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:48` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:49` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:50` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:51` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:52` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:53` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:54` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:55` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:56` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:57` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:58` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:59` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:60` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:61` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:62` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:63` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:64` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:65` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:66` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:67` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:68` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:69` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:70` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:71` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:72` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:73` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:74` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:75` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:76` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:77` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:78` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:79` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:80` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:81` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:82` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:83` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:84` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:85` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:86` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:87` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:88` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:89` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:90` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:91` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:92` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:93` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:94` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:95` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:96` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:97` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:98` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:99` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:100` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:101` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:102` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:103` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:104` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:105` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:144` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:158` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:173` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:192` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:207` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:226` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:253` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:269` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:287` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:293` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:308` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:315` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/render/core.rs:330` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:342` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:354` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:366` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:378` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:390` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:405` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:421` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:436` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:452` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:515` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:549` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:560` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:571` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:580` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:592` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:639` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:688` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:697` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/render/core.rs:712` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/common/testutils.rs:61` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/common/testutils.rs:70` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/common/testutils.rs:175` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/common/testutils.rs:184` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/common/testutils.rs:195` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/common/testutils.rs:206` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/common/testutils.rs:271` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/common/testutils.rs:284` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/audio/device.rs:193` | raw pointer/object ownership | Owns or validates SDL audio device, stream, decoder, or buffer pointers crossing the C ABI. |
| `safe/src/audio/device.rs:339` | raw pointer/object ownership | Owns or validates SDL audio device, stream, decoder, or buffer pointers crossing the C ABI. |
| `safe/src/audio/device.rs:515` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:520` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:531` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:544` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:549` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:559` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:567` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:585` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:625` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:670` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:692` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:716` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:721` | raw pointer/object ownership | Owns or validates SDL audio device, stream, decoder, or buffer pointers crossing the C ABI. |
| `safe/src/audio/device.rs:727` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:736` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:747` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:760` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:771` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:785` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:796` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:807` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:818` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:823` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:851` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:874` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/audio/device.rs:886` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/original_apps_render.rs:56` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:73` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:92` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:115` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:126` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:145` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:177` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:196` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:292` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:347` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:567` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_render.rs:639` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_surface.rs:19` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_surface.rs:55` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/input/gamecontroller.rs:168` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/gamecontroller.rs:175` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/gamecontroller.rs:410` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/gamecontroller.rs:456` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/gamecontroller.rs:465` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/gamecontroller.rs:473` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:521` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:544` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:550` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:567` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:594` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:614` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:623` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:635` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:649` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:664` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:682` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:716` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:729` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:744` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:758` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:771` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:786` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:797` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:808` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:819` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:830` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:841` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:852` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:863` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:870` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:882` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:893` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:906` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:911` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:921` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:930` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:945` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:956` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:975` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:985` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:994` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1009` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1020` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1039` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1050` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1068` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1122` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1136` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1161` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1176` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1189` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1205` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1236` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1251` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1266` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1273` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1280` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1289` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1304` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1317` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1326` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/gamecontroller.rs:1334` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/security_gles_texture_lifecycle.rs:38` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_gles_texture_lifecycle.rs:49` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_gles_texture_lifecycle.rs:131` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_gles_texture_lifecycle.rs:135` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_gles_texture_lifecycle.rs:182` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_gles_texture_lifecycle.rs:205` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_events_timers.rs:30` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_events_timers.rs:44` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_events_timers.rs:55` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_events_timers.rs:74` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_events_timers.rs:90` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_events_timers.rs:114` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_events_timers.rs:241` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_events_timers.rs:295` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/openurl_smoke.rs:15` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/openurl_smoke.rs:27` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/input/haptic.rs:40` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/haptic.rs:59` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/haptic.rs:86` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:96` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:114` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:142` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:163` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:176` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:181` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:187` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:196` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:230` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:239` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:246` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:259` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:268` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:275` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:289` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:311` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:333` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:363` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:379` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:389` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:402` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:411` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:423` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:432` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:441` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:449` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:467` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:479` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/haptic.rs:495` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/upstream_port_input.rs:46` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:82` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:89` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:91` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:111` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:147` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:150` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:154` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:158` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:162` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:166` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:169` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:170` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:171` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:173` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:177` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:180` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:181` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:183` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:187` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:189` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:192` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:199` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:208` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:217` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:221` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:231` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:240` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:244` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:253` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:256` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_input.rs:258` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/input/guid.rs:10` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/guid.rs:36` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/guid.rs:55` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/guid.rs:64` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/guid.rs:69` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:28` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:86` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:97` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:108` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:115` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:120` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:135` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:164` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:177` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:196` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:209` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:222` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:240` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:246` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:270` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:275` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:282` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:286` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:314` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:319` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:328` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:362` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:373` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/queue.rs:393` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:410` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:413` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:421` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:444` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:457` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:462` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/queue.rs:509` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:30` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/sensor.rs:81` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/sensor.rs:116` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/sensor.rs:124` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:127` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:130` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:143` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:156` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:168` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:180` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:192` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:215` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:226` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:236` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:246` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:256` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:266` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:275` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:306` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/sensor.rs:315` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:21` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:29` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:36` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:37` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:38` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:39` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:40` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:47` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:49` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:50` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:52` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:53` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:54` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:56` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:57` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:58` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:59` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:61` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:63` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:64` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:65` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:66` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:67` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:68` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:69` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:71` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:72` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:73` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:74` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:75` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:76` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:77` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:78` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:79` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:80` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:81` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:82` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:88` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:89` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:90` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:92` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:93` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:94` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:95` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:96` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:97` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:98` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:99` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:100` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:101` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:102` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:103` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:104` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:105` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:106` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:107` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:109` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:113` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:119` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:208` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:250` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:292` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:305` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:340` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:358` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:426` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:473` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:575` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:592` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:605` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/window.rs:617` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:633` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:666` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:676` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:699` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:713` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:726` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:756` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:765` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:789` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:798` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:819` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:828` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:838` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:858` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:873` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:882` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:894` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:915` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:936` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:948` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:963` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:982` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:991` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1012` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1033` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1047` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1066` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1075` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1088` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1101` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1114` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1124` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1137` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1153` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1169` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1185` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1217` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1233` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1272` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1290` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1308` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1318` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1331` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1347` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1375` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1403` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1416` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1434` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1450` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1469` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1487` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1503` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1535` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1550` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1563` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/window.rs:1576` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/touch.rs:4` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/touch.rs:9` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/touch.rs:14` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/touch.rs:22` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/touch.rs:27` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/upstream_port_audio.rs:34` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_audio.rs:47` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_audio.rs:71` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_audio.rs:117` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_audio.rs:195` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_audio.rs:271` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_audio.rs:348` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_audio.rs:427` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/events/mouse.rs:101` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:106` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:119` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:135` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:140` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:156` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:161` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:166` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:181` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:186` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:191` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:203` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:208` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:225` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:238` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:244` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:258` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/mouse.rs:268` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/gesture.rs:28` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/gesture.rs:39` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/gesture.rs:58` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/gesture.rs:64` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/gesture.rs:70` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/original_apps_audio.rs:39` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:46` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:66` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:116` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:136` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:160` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:220` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:240` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:278` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:310` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:358` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:415` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:495` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_audio.rs:560` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/evdev_fixtures.rs:20` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/evdev_fixtures.rs:66` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/evdev_fixtures.rs:110` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/events/keyboard.rs:10` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:11` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:12` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:13` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:14` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:15` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:19` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:25` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:103` | raw pointer/object ownership | Owns or validates raw event, callback, mouse, keyboard, gesture, or touch data crossing the SDL C ABI. |
| `safe/src/events/keyboard.rs:354` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:359` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:368` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:373` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:378` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:404` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:416` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:428` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:440` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:452` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/events/keyboard.rs:461` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/mod.rs:39` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:40` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:41` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:42` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:43` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:44` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:48` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:195` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:315` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:570` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:584` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:594` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:604` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:624` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:637` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:651` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:664` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:710` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/mod.rs:719` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/video/shape.rs:6` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/shape.rs:15` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/shape.rs:16` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/shape.rs:17` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/shape.rs:25` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/shape.rs:31` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/shape.rs:45` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/shape.rs:69` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/shape.rs:81` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/shape.rs:90` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/main_archive.rs:17` | public SDL ABI exports | Provides archive-visible SDL compatibility entry points for C linkers. |
| `safe/src/main_archive.rs:33` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/main_archive.rs:38` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/main_archive.rs:50` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/main_archive.rs:55` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:65` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/hidapi.rs:78` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/hidapi.rs:98` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/hidapi.rs:238` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/hidapi.rs:263` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:268` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:273` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:287` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:326` | raw pointer/object ownership | Owns or validates SDL raw controller, joystick, haptic, or sensor handles crossing the C ABI. |
| `safe/src/input/hidapi.rs:338` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:347` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:373` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:397` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:417` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:441` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:460` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:472` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:492` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:511` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:519` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:532` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:545` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:558` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/hidapi.rs:578` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/validator_surface_render.rs:31` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_surface_render.rs:68` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_surface_render.rs:108` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_surface_render.rs:190` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_surface_render.rs:219` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/validator_surface_render.rs:271` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/input/joystick.rs:63` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:66` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:69` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:79` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:89` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:99` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:108` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:117` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:126` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:135` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:144` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:153` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:159` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:177` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:190` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:203` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:233` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:309` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:322` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:331` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:354` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:381` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:404` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:414` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:424` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:432` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:448` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:456` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:464` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:472` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:480` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:488` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:496` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:504` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:510` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:519` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:527` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:537` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:545` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:553` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:574` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:587` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:600` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:624` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:637` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:659` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:690` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:712` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:730` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:741` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:752` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:763` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:781` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:798` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/input/joystick.rs:810` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:846` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:851` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:856` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:861` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:866` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:871` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:876` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:881` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:886` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:891` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:896` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:901` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:906` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:911` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:916` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:921` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:926` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:931` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:936` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:941` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/exports/generated_linux_stubs.rs:946` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/egl.rs:11` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/egl.rs:14` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/egl.rs:15` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/egl.rs:27` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/egl.rs:34` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/egl.rs:36` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/egl.rs:42` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/egl.rs:53` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/egl.rs:59` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/abi/generated_types.rs:392` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:411` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:414` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:417` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:423` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:427` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:429` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:432` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:438` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:439` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:448` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:457` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:466` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:470` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:473` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:480` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:486` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:493` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:500` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:507` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:510` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:513` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:516` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:519` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:522` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:525` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:528` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:531` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:534` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:537` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:540` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:543` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:546` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:549` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:552` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:555` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:558` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:565` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:572` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:579` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:586` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:589` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:592` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:595` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:598` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:601` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:604` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:611` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:614` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:621` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:624` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:631` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:638` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:645` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:648` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:651` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:654` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:657` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:663` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:669` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:675` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:681` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:688` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:691` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:694` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:701` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:708` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:715` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:722` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:729` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:736` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:739` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:742` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:749` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:756` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:763` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:770` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:776` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:782` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:789` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:795` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:802` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:809` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:816` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:824` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:832` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:839` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:846` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:850` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:853` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:856` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:859` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:862` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:865` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:868` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:871` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:874` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:877` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:880` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:883` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:886` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:889` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:892` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:895` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:898` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:901` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:904` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:907` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:910` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:913` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:916` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:919` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:922` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:925` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:928` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:931` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:934` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:937` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:940` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:943` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:946` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:949` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:952` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:955` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:958` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:961` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:964` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:967` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:970` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:979` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:985` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:988` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:997` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1008` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1013` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1019` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1045` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1055` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1060` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1067` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1071` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1077` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1081` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1086` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1090` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1094` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1098` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1102` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1111` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1119` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1123` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1127` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1131` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1139` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1146` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1150` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1154` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1158` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1165` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1176` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1184` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1188` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1192` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1196` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1200` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1210` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1214` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1218` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1222` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1226` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1230` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1234` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1243` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1247` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1251` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1255` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1259` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1263` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1286` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1288` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1296` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1305` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1309` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1313` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1317` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1321` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1325` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1329` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1333` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1337` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1342` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1345` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1354` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1357` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1365` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1374` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1383` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1407` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1414` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1418` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1425` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1432` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1436` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1440` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1444` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1452` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1456` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1465` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1474` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1478` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1486` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1493` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1497` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1501` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1505` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1509` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1513` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1517` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1521` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1525` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1529` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1533` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1537` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1541` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1545` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1553` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1583` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1610` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1614` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1618` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1622` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1626` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1630` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1639` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1643` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1650` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1658` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1666` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1680` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1684` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1688` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1692` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1696` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1706` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1710` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1722` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1732` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1743` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1751` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1759` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1763` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1767` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1771` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1775` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1784` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1794` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1802` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1810` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1814` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1818` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1822` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1826` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1830` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1834` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1838` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1842` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1846` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1850` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1854` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1860` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1864` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1868` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1872` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1876` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1880` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1884` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1888` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1892` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1896` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1900` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1904` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1908` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1912` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1916` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1920` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1924` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1928` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1932` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1936` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1940` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1944` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1948` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1952` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:1959` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2120` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2124` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2135` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2145` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2149` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2153` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2157` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2164` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2173` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2177` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2181` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2191` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2201` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2212` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2248` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2252` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2260` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2264` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2273` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2283` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2287` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2295` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2299` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2308` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2365` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2412` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2429` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2442` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2452` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2466` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2477` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2481` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2488` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2492` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2496` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2500` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2508` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2515` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2519` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2527` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2531` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2535` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2544` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2553` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2558` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2565` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2572` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2579` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2583` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2587` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2590` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2598` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2606` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2619` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2632` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2640` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2649` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2658` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2667` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2676` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2685` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2694` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2703` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2707` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2711` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2911` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2915` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2919` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2923` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2927` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2931` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2935` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2940` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2947` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2954` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2963` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2968` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2972` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2980` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2987` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:2994` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3002` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3006` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3010` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3014` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3021` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3028` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3035` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3039` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3050` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3054` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3058` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3062` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3066` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3070` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3074` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3078` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3086` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3093` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3101` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3109` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3117` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3125` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3135` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3143` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3151` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3159` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3167` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3175` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3179` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3183` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3187` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3191` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3195` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3199` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3203` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3207` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3211` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3216` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3220` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3224` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3228` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3236` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3240` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3244` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3248` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3252` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3256` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3260` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3264` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3268` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3275` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3279` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3286` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3290` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3294` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3301` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3308` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3312` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3321` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3346` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3352` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3360` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3367` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3371` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3375` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3379` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3383` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3387` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3393` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3397` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3401` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3405` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3412` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3419` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3423` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3430` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3434` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3438` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3446` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3450` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3454` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:3458` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4042` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4046` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4050` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4054` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4058` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4062` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4066` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4070` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4074` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4078` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4082` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4086` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4090` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4094` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4098` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4102` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4106` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4110` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4114` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4156` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4160` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4167` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4174` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4181` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4189` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4196` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4200` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4204` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4208` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4219` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4227` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4231` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4235` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4239` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4243` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4247` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4257` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4265` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4298` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4302` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4306` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4310` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4316` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4322` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4328` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4332` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4336` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4340` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4344` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4348` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4352` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4356` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4360` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4364` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4402` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4405` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4412` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4420` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4428` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4437` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4444` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4450` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4454` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4458` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4466` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4474` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4482` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4486` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4490` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4494` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4501` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4505` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4509` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4513` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4517` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4521` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4525` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4529` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4537` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4543` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4553` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4557` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4561` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4565` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4569` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4573` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4577` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4581` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4585` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4589` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4597` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4601` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4610` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4617` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4626` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4635` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4639` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4643` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4647` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4656` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4664` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4668` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4698` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4702` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4705` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4709` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4715` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4719` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4725` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4729` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4733` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4737` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4741` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4745` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4749` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4753` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4761` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4770` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4774` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4830` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4837` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4843` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4847` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4853` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4857` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4863` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4867` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4873` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4879` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4885` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4891` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4896` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4900` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4906` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4912` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4918` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4924` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4930` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4937` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4941` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4945` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4949` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4953` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4959` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4963` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4967` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4973` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4977` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4991` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:4997` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5003` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5010` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5017` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5050` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5056` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5062` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5069` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5076` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5083` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5089` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5096` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5108` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5115` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5123` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5130` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5137` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5146` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5156` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5165` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5174` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5178` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5182` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5187` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5196` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5204` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5208` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5215` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5237` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5241` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5245` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5249` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5253` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5257` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5265` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5269` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5273` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5280` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5985` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:5993` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6003` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6007` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6011` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6015` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6019` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6023` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6027` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6034` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6040` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6045` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6049` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6056` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6060` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6064` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6068` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6072` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6076` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6080` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6286` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6290` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6294` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6298` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6302` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6306` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6310` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6314` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6318` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6322` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6326` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6330` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6334` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6338` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6342` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6349` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6356` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6364` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6372` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6379` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6383` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6390` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6397` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6404` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6408` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6412` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6416` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6420` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6424` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6432` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6474` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6478` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6482` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6486` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6493` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6497` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6505` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6512` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6520` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6529` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6537` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6544` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6552` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6560` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6564` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6572` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6580` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6588` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6597` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6606` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6614` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6621` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6625` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6629` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6633` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6642` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6649` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6657` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6665` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6669` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6673` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6680` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6715` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6719` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6723` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6727` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6731` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6735` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6739` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6743` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6747` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6751` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6755` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6763` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6772` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6783` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6790` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6797` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6874` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6881` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6892` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6896` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6900` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6904` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:6924` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7013` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7017` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7024` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7034` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7042` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7046` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7050` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7054` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7061` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7069` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7079` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7086` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7096` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7105` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7114` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7119` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7126` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7133` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7140` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7147` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7154` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7161` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7165` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7174` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7187` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7198` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7207` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7215` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7219` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7223` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7230` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7234` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7242` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7250` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7257` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7261` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7268` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7272` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7279` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7283` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7287` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7295` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7299` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7309` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7319` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7329` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7339` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7346` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7353` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7357` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7365` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7373` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7383` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7391` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7398` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7406` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7413` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7421` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7430` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7442` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7450` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7458` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7468` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7476` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7483` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7491` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7498` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7506` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7515` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7527` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7538` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7555` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7565` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7569` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7573` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7577` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7581` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7589` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7593` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7597` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7603` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7610` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7621` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7652` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7660` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7667` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7674` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7682` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7686` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7689` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7692` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7695` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7698` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7701` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7704` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7708` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7712` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7716` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7720` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7726` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7730` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7738` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7753` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7757` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7761` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7773` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7777` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7781` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7785` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7789` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7793` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/src/abi/generated_types.rs:7797` | generated bindgen ABI declarations | Captures upstream C declarations and callback signatures generated from public headers for ABI-compatible layout. |
| `safe/tests/upstream_port_render.rs:50` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_render.rs:64` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_render.rs:93` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_render.rs:125` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_render.rs:167` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_render.rs:233` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_render.rs:342` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/video/rect.rs:153` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/rect.rs:180` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/rect.rs:230` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/rect.rs:523` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/rect.rs:550` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/rect.rs:600` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/rect.rs:769` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:780` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:798` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:819` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:835` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:861` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:875` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:893` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:914` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/rect.rs:930` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:112` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:113` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:114` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:115` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:116` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:117` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:118` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:119` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:120` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:122` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:124` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:126` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:127` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:132` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:133` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:134` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:135` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:136` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:140` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:146` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:203` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:261` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:280` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/display.rs:349` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:354` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:365` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:372` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:380` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:385` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:397` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:415` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:437` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:459` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:488` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:502` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:515` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:538` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:560` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:582` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:605` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:620` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:635` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:650` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/display.rs:662` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/testsupport/crc32.rs:8` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/crc32.rs:27` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/crc32.rs:42` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/crc32.rs:67` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/crc32.rs:82` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/crc32.rs:98` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/random.rs:7` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/random.rs:23` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/random.rs:35` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/video/blit.rs:28` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:46` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:68` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:251` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:264` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:290` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:316` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:343` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:422` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:585` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:602` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:690` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:712` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:716` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:781` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/blit.rs:883` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1078` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1084` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1090` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1099` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1199` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1230` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1262` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1299` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1309` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/blit.rs:1340` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:82` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:83` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:612` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:619` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:624` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:627` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:628` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:635` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:671` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:695` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:920` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/pixels.rs:952` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:965` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:974` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:987` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1006` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1024` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1033` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1049` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1069` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1084` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1105` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1127` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1157` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/pixels.rs:1197` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/misc.rs:5` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/testsupport/compare.rs:7` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/compare.rs:25` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/compare.rs:113` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/compare.rs:121` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/original_apps_video.rs:49` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:65` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:73` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:91` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:106` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:136` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:171` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:222` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:252` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:272` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:328` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:367` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:399` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:439` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:472` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_video.rs:506` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/testsupport/harness.rs:18` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/harness.rs:28` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/harness.rs:48` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/harness.rs:131` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/harness.rs:158` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/video/vulkan.rs:6` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/vulkan.rs:7` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/vulkan.rs:8` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/vulkan.rs:10` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/vulkan.rs:11` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/vulkan.rs:12` | external GL/EGL/Vulkan function pointers | Stores or calls dynamically resolved GL, GLES, EGL, Vulkan, or host SDL function pointers. |
| `safe/src/video/vulkan.rs:30` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/vulkan.rs:36` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/vulkan.rs:42` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/vulkan.rs:48` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/vulkan.rs:65` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/vulkan.rs:82` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:32` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:35` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:77` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:78` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:95` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:113` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:114` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:127` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:128` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:129` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:130` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:131` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:132` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:133` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:134` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:135` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:136` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:137` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:138` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:139` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:140` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:141` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:142` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:143` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:144` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:145` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:146` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:147` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:148` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:149` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:150` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:151` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:152` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:153` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:154` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:155` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:156` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:157` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:158` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:159` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:160` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:161` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:162` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:163` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:164` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:165` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:166` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:167` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:168` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:169` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:170` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:171` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:172` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:173` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:174` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:175` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:176` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:177` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:178` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:179` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:180` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:181` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:182` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:183` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:184` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:185` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:186` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:187` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:188` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:196` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:198` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:258` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:274` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:325` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:402` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:412` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:441` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:508` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:563` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:572` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:591` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:604` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:617` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:628` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:638` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:701` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:807` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:889` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:908` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:979` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:1005` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:1033` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:1107` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:1175` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:1189` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:1205` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:1282` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/surface.rs:1309` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1338` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1357` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1392` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1417` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1435` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1463` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1483` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1497` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1526` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1537` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1569` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1580` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1607` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1632` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1660` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1683` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1706` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1737` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1760` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1786` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1798` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1806` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1832` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1853` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/surface.rs:1882` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/security_surface_math.rs:42` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_surface_math.rs:137` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/video/mod.rs:45` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/video/mod.rs:51` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/testsupport/images.rs:187` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:210` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:218` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:226` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:234` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:242` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:250` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:258` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:266` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:274` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:282` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/images.rs:290` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/assert.rs:12` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/assert.rs:27` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/assert.rs:37` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/assert.rs:52` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/assert.rs:58` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/assert.rs:64` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/assert.rs:80` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/video/messagebox.rs:4` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/messagebox.rs:15` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/original_apps_input.rs:104` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:108` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:112` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:118` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:128` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:141` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:157` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:165` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:173` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:180` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:186` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:187` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:244` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:250` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:254` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:258` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:262` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:266` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:275` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:278` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:283` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:285` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:286` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:287` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:289` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:298` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:302` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:306` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:310` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:314` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:318` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:322` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:324` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:328` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:338` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:339` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:349` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:350` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:360` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:361` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:364` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:370` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:379` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:383` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:392` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:401` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:410` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:419` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:424` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:438` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:440` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:443` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:445` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:449` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:453` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:456` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:460` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:464` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:468` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:473` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:474` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:478` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:482` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:486` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:490` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:494` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:496` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:499` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:500` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:502` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:506` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:507` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:510` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:520` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:524` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:528` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:532` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:536` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:540` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:544` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:547` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:552` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:560` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:562` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:566` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:575` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:584` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:593` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:603` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:605` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:609` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:628` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:632` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:636` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:640` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:646` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:662` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:672` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:676` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:686` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:696` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:708` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:712` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:715` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:717` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:728` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:732` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:735` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:737` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:747` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:751` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:752` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:754` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:757` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:758` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:776` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:777` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:780` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:781` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:783` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:787` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:791` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:795` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:799` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:802` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:804` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:807` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:808` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:812` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:813` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:814` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:815` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:817` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:823` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:824` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:825` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:826` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:827` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:828` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:829` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:830` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:831` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:832` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:835` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:847` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:849` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:852` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:853` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:854` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:855` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:856` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:857` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:858` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:860` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:864` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:865` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:867` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:871` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:874` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:876` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:879` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:883` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:884` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:887` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:891` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:895` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:901` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:916` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:926` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:930` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:934` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:938` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:942` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:952` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:962` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:972` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:985` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:986` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:987` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:989` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:990` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:992` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:995` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:998` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1002` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1007` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1011` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1014` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1019` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1027` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1034` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1042` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1048` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1051` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1056` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1058` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/original_apps_input.rs:1065` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/core/filesystem.rs:5` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/filesystem.rs:20` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/testsupport/memory.rs:40` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/memory.rs:51` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/memory.rs:64` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/memory.rs:79` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/memory.rs:96` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/memory.rs:136` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/video/clipboard.rs:24` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/clipboard.rs:29` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/clipboard.rs:34` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/clipboard.rs:39` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/clipboard.rs:44` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/clipboard.rs:59` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/testsupport/mod.rs:83` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:84` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:85` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:174` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:181` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:185` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:194` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:209` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:220` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:240` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:247` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:251` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:258` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/mod.rs:264` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/md5.rs:138` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/md5.rs:158` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/md5.rs:168` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/md5.rs:183` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/core/power.rs:17` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/testsupport/log.rs:11` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/log.rs:34` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/log.rs:54` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/log.rs:59` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:84` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:117` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:135` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:169` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:261` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:580` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:590` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:595` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:698` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:720` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:755` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/common.rs:768` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:56` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:85` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:112` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:135` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:153` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:171` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:237` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:252` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/font.rs:262` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/tests/security_wave_adpcm.rs:63` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_wave_adpcm.rs:86` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_wave_adpcm.rs:110` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_wave_adpcm.rs:135` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/security_wave_adpcm.rs:157` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/testsupport/fuzzer.rs:12` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:25` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:35` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:43` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:49` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:55` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:61` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:67` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:73` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:79` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:85` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:93` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:99` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:112` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:159` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:208` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:222` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:236` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:250` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:259` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:274` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:289` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:304` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:313` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:318` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:323` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:328` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:346` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:351` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/testsupport/fuzzer.rs:366` | test support | Test support constructs raw SDL fixtures, callbacks, or pointers to verify ABI behavior that safe Rust callers cannot exercise. |
| `safe/src/core/rwops.rs:16` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:20` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:24` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:28` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:42` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:54` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:64` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:74` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:89` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:93` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:114` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:141` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:163` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:173` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:178` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:200` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:210` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:236` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:261` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:277` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:293` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:298` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:305` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:314` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:330` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:335` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:352` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:369` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:378` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:425` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:438` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:443` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:448` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:453` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:458` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:463` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:468` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:473` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/rwops.rs:483` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:488` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:493` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:498` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:503` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:508` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/rwops.rs:513` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/tests/upstream_port_video_events.rs:61` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:67` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:73` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:91` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:133` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:194` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:248` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:317` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:355` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/tests/upstream_port_video_events.rs:481` | integration tests | Integration coverage uses raw SDL pointers or callbacks to assert C ABI compatibility and regression behavior. |
| `safe/src/video/syswm.rs:89` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/syswm.rs:93` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/syswm.rs:99` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/syswm.rs:110` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/platform.rs:4` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/timer.rs:39` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/timer.rs:44` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/timer.rs:49` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/timer.rs:54` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/timer.rs:59` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/timer.rs:64` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/timer.rs:84` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/timer.rs:96` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/loadso.rs:4` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/loadso.rs:24` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/loadso.rs:51` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:30` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:43` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:57` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:66` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:83` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:92` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:101` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:110` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:127` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:139` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:148` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/system.rs:153` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/memory.rs:17` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/memory.rs:21` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/memory.rs:25` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/memory.rs:29` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/memory.rs:59` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/memory.rs:68` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/memory.rs:75` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/memory.rs:87` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/memory.rs:99` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/memory.rs:115` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/memory.rs:127` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/memory.rs:149` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/memory.rs:171` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/memory.rs:200` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/linux/ime.rs:28` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/linux/ime.rs:31` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/linux/ime.rs:36` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/linux/ime.rs:41` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/linux/ime.rs:46` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/linux/ime.rs:52` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/linux/ime.rs:57` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:15` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/thread.rs:34` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/thread.rs:44` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/thread.rs:60` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:69` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:106` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/thread.rs:127` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:140` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:145` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:154` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:163` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:178` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:187` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:192` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:202` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:205` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/thread.rs:229` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:234` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/thread.rs:246` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:17` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/hints.rs:38` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/hints.rs:52` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/hints.rs:78` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/hints.rs:105` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/hints.rs:106` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/hints.rs:107` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/hints.rs:108` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/hints.rs:131` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:186` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:194` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:235` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:272` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:293` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:305` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:352` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/hints.rs:369` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:16` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/cpuinfo.rs:28` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/cpuinfo.rs:31` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/cpuinfo.rs:34` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/cpuinfo.rs:41` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/cpuinfo.rs:46` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:53` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:58` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:70` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:74` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:85` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:89` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:100` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:111` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:122` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:133` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:144` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:155` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:166` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:177` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:181` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:196` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:200` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:205` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:216` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:236` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:256` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/cpuinfo.rs:265` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:4` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:13` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:20` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:24` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:28` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:32` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:36` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:40` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:44` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:48` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:54` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:58` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:62` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:66` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:70` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:74` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:78` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:82` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:90` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:94` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:98` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:102` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:106` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:110` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:114` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:118` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:122` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:126` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:130` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:134` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:138` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:142` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:146` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:150` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:154` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:158` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:162` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/libm.rs:166` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/error.rs:9` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/error.rs:24` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/error.rs:28` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/error.rs:32` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/error.rs:39` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/error.rs:54` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/error.rs:68` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/error.rs:87` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/error.rs:93` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/bmp.rs:22` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/bmp.rs:36` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/bmp.rs:59` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/bmp.rs:121` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/bmp.rs:136` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/bmp.rs:330` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/bmp.rs:343` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/bmp.rs:360` | renderer hot paths | Touches renderer, surface, pixel, window, or texture pointers where the SDL ABI exposes raw C data structures. |
| `safe/src/video/bmp.rs:372` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/video/bmp.rs:400` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/locale.rs:19` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/assert.rs:16` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/assert.rs:37` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/assert.rs:45` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/assert.rs:51` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/assert.rs:89` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/assert.rs:99` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/assert.rs:104` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/assert.rs:115` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/assert.rs:120` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/init.rs:259` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/init.rs:264` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/init.rs:284` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/init.rs:290` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/init.rs:319` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/log.rs:21` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/log.rs:42` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/log.rs:115` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/log.rs:149` | C variadic shim references | Binds Rust code to the C shim required for SDL variadic formatting, error storage, and log callbacks that Rust cannot express directly. |
| `safe/src/core/log.rs:156` | C variadic shim references | Binds Rust code to the C shim required for SDL variadic formatting, error storage, and log callbacks that Rust cannot express directly. |
| `safe/src/core/log.rs:162` | C variadic shim references | Binds Rust code to the C shim required for SDL variadic formatting, error storage, and log callbacks that Rust cannot express directly. |
| `safe/src/core/log.rs:182` | C variadic shim references | Binds Rust code to the C shim required for SDL variadic formatting, error storage, and log callbacks that Rust cannot express directly. |
| `safe/src/core/log.rs:189` | C variadic shim references | Binds Rust code to the C shim required for SDL variadic formatting, error storage, and log callbacks that Rust cannot express directly. |
| `safe/src/core/log.rs:203` | C variadic shim references | Binds Rust code to the C shim required for SDL variadic formatting, error storage, and log callbacks that Rust cannot express directly. |
| `safe/src/core/log.rs:212` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/log.rs:232` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/stdlib.rs:7` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/stdlib.rs:10` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/stdlib.rs:26` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/stdlib.rs:83` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/stdlib.rs:97` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/stdlib.rs:133` | allocation and string/memory helpers | Implements SDL allocation, string, RWops, assertion, and memory helpers over raw C buffers supplied by the ABI. |
| `safe/src/core/stdlib.rs:151` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:159` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:175` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:188` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:202` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:207` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:216` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:225` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:234` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:243` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:252` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:270` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:288` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:293` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:306` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:320` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:333` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:347` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:359` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:371` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:382` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:406` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:420` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:433` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:446` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:455` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:464` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:473` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:482` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:491` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:500` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:514` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:532` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:553` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:558` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:580` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:601` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:629` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:637` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:676` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:685` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:694` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:709` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:723` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:738` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:753` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:768` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:773` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:778` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:783` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:788` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:793` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:798` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:803` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:808` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:813` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:818` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:823` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:828` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:833` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:857` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:865` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/stdlib.rs:886` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:33` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/mutex.rs:37` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/mutex.rs:41` | OS and dynamic-loader glue | Calls platform APIs or loader entry points through raw descriptors, handles, or C pointers required by SDL compatibility. |
| `safe/src/core/mutex.rs:45` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:68` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:81` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:95` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:108` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:116` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:129` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:137` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:155` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:171` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:197` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:211` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:224` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:236` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:244` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:258` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:272` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |
| `safe/src/core/mutex.rs:289` | public SDL ABI exports | Keeps the exported SDL C ABI callable with raw pointers and C layout exactly as the upstream headers require. |

## Remaining unsafe FFI beyond the original ABI/API boundary.
The intended boundary is the exported SDL2 C ABI: `safe/generated/linux_symbol_manifest.json` declares 839 symbols and `readelf` confirms the shared object has SONAME `libSDL2-2.0.so.0`. The table separates that intended export surface from additional consumed FFI used internally or by tooling.

| Boundary item | Symbols and source evidence | Provider | Why needed | Plausible safe-Rust replacement |
| --- | --- | --- | --- | --- |
| Intended exported SDL ABI | SDL symbols from `safe/generated/linux_symbol_manifest.json` and Rust exports under concrete modules such as `safe/src/core`, `safe/src/events`, `safe/src/input`, `safe/src/render`, `safe/src/video`, and `safe/src/exports`; `nm -D` reports 839 defined symbols. | Root crate `safe-sdl` builds `safe_sdl` as `cdylib`/`staticlib`/`rlib`. | The port must present the Ubuntu SDL2 C ABI, including raw pointers, callbacks, and aborting compatibility stubs. | No safe-Rust replacement exists at the library boundary; internal wrappers can reduce unsafe behind the ABI. |
| glibc/libc and libm calls | `malloc`, `free`, `memcpy`, `strlen`, `qsort`, `bsearch`, `pthread_create`, `pthread_mutex_lock`, `pthread_cond_wait`, `pthread_cond_timedwait`, `pthread_join`, `sem_wait`, `sem_post`, `clock_gettime`, math functions, and process/file APIs appear in `/tmp/libsdl-port-undefined-symbols.txt`. | `libc` crate plus linked `libc.so.6` and `libm.so.6` from `/tmp/libsdl-port-readelf-dynamic.txt`. | SDL exposes C allocation/string/runtime behavior and this port must preserve libc-compatible semantics. | Some internals could move to `std` or `rustix`, but exported C allocation, callback, and process behavior still requires FFI at the edge. |
| iconv | `iconv_open`, `iconv`, and `iconv_close` are declared at `safe/src/core/stdlib.rs:10` through `safe/src/core/stdlib.rs:19` and exported through `SDL_iconv_open`, `SDL_iconv_close`, `SDL_iconv`, and `SDL_iconv_string` at `safe/src/core/stdlib.rs:833` through `safe/src/core/stdlib.rs:886`. | `libc` crate and glibc `iconv*` symbols from `/tmp/libsdl-port-undefined-symbols.txt`. | SDL has public iconv APIs and applications can pass opaque conversion descriptors through the C ABI. | `encoding_rs` or Rust codecs could replace selected conversions internally, but not the opaque platform `SDL_iconv_t` ABI without a compatibility layer. |
| dlopen/dlsym/dlclose | `safe/src/core/loadso.rs:9`, `safe/src/core/loadso.rs:36`, `safe/src/core/loadso.rs:53`, `safe/src/video/mod.rs:44`, `safe/src/video/mod.rs:45`, `safe/src/video/surface.rs:195`, and `safe/src/video/surface.rs:196` load host symbols; `/tmp/libsdl-port-undefined-symbols.txt` lists `dlopen`, `dlsym`, and `dlclose`. | glibc dynamic loader APIs through `libc`; `safe/build.rs:44` requests `dl` linkage. | SDL exposes `SDL_LoadObject` and also forwards selected host video/renderer functionality by symbol lookup. | A safe wrapper can narrow pointer casts, but symbol loading itself remains an OS FFI boundary. |
| evdev open/ioctl/read | `safe/src/input/linux/evdev.rs:184` opens devices, `safe/src/input/linux/evdev.rs:564` reads input events, and `safe/src/input/linux/evdev.rs:684` through `safe/src/input/linux/evdev.rs:707` wrap ioctls. | `libc` crate and glibc `open`, `read`, and `ioctl` from `/tmp/libsdl-port-undefined-symbols.txt`. | Linux joystick/input compatibility needs evdev discovery and event decoding. | `rustix` can wrap `open` and `read`, but evdev ioctl payloads still need checked raw FFI or a dedicated safe evdev crate. |
| C variadic shims | `safe/src/core/phase2_variadic_shims.c:79` through `safe/src/core/phase2_variadic_shims.c:239` implements `SDL_SetError`, `SDL_snprintf`, `SDL_asprintf`, `SDL_sscanf`, and `SDL_Log*`; Rust declarations in `safe/src/core/error.rs:9` and `safe/src/core/log.rs:212` bind to the shim. | C object compiled by `cc::Build` from `safe/build.rs:46` through `safe/build.rs:50`. | Rust cannot define C variadic functions with stable safe Rust, while SDL exposes variadic logging, formatting, and error APIs. | No direct safe Rust replacement for variadic exported C functions; keep the shim small and hidden except for intended SDL symbols. |
| EGL dynamic loading | `safe/src/video/egl.rs:28` probes EGL runtime candidates, `safe/src/video/egl.rs:34` resolves `eglGetProcAddress`, and `safe/src/video/egl.rs:37`/`safe/src/video/egl.rs:60` close handles. | glibc dynamic loader APIs through `libc`; Debian build dependencies include `libegl-dev` at `safe/debian/control:17`. | SDL must locate an EGL implementation at runtime without hard-linking every video backend. | A typed EGL loader crate could reduce manual casts, but proc address discovery remains dynamic FFI. |
| GL/GLES/Vulkan proc tables | `safe/src/render/gl.rs:46` through `safe/src/render/gl.rs:65`, `safe/src/render/gles.rs:177` through `safe/src/render/gles.rs:179`, and `safe/src/video/vulkan.rs:18` through `safe/src/video/vulkan.rs:25` resolve host SDL or graphics entry points. | Function pointers loaded through `crate::video::load_symbol` and graphics system libraries provided by Debian build-deps such as `libgl-dev`, `libgles-dev`, and `libvulkan-dev`. | SDL renderer/video APIs expose runtime-selected GL, GLES, Metal, and Vulkan behavior. | Typed loader wrappers can reduce unsafe calls, but runtime proc tables and foreign handles stay at the FFI boundary. |
| tool/build-time cc and bindgen | `safe/build.rs:5` and `safe/sdl2-test/build.rs:9` use `cc::Build`; `safe/xtask/src/contracts.rs:2699` through `safe/xtask/src/contracts.rs:2713` use `bindgen::Builder`; `safe/xtask/src/original_tests.rs:1021` calls `pkg-config`. | Rust crates `cc` and `bindgen`; transitive `clang-sys` and `libloading` from `/tmp/libsdl-port-cargo-tree.txt`; system `pkg-config` from `safe/debian/control:43`. | The port captures C header layout, compiles C variadic helpers, and stages original-test build flags. | These are build/tooling dependencies; generated output should be reviewed and kept checked in rather than generated opportunistically during normal builds. |

`/tmp/libsdl-port-readelf-dynamic.txt` reports non-SDL dynamic dependencies on `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and `ld-linux-x86-64.so.2`; `/tmp/libsdl-port-undefined-symbols.txt` contains 200 undefined dynamic symbols, including loader, iconv, pthread, semaphore, ioctl, file, allocator, math, and unwind symbols.

## Remaining issues.
The issue scan in `/tmp/libsdl-port-issues-grep.txt` found no `TODO`, `FIXME`, `todo!`, or `unimplemented!` matches in the requested tree. The remaining matches are aborting compatibility stubs, gated tests, one production guard panic, one test assertion panic, generated-stub tooling text, and `always_ignore` field assignments in the SDL assert implementation.

**Aborting exported stubs.**
| Location | Stub symbol | Status |
| --- | --- | --- |
| `safe/src/exports/generated_linux_stubs.rs:847` | `SDL_ComposeCustomBlendMode` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:852` | `SDL_DYNAPI_entry` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:857` | `SDL_GetTouchDeviceType` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:862` | `SDL_GetWindowICCProfile` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:867` | `SDL_HasWindowSurface` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:872` | `SDL_IsTablet` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:877` | `SDL_IsTextInputShown` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:882` | `SDL_OnApplicationDidBecomeActive` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:887` | `SDL_OnApplicationDidEnterBackground` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:892` | `SDL_OnApplicationDidReceiveMemoryWarning` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:897` | `SDL_OnApplicationWillEnterForeground` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:902` | `SDL_OnApplicationWillResignActive` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:907` | `SDL_OnApplicationWillTerminate` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:912` | `SDL_SetWindowGammaRamp` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:917` | `SDL_crc16` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:922` | `SDL_crc32` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:927` | `SDL_isblank` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:932` | `SDL_strtokr` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:937` | `SDL_utf8strlcpy` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:942` | `SDL_utf8strlen` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |
| `safe/src/exports/generated_linux_stubs.rs:947` | `SDL_utf8strnlen` | Exported by the Linux ABI surface and currently aborts through `safe/src/lib.rs:31` rather than returning a partial implementation. |

**Other issue-scan findings.**
| Finding | Evidence | Disposition |
| --- | --- | --- |
| Host-gated tests | `safe/tests/upstream_port_core.rs:112` and `safe/tests/original_apps_core.rs:257` are ignored unless run with `--features host-video-tests`. | Expected gating for tests that need host video or original app environment. |
| Evdev fixture tests | `safe/tests/evdev_fixtures.rs:52` and `safe/tests/evdev_fixtures.rs:94` are ignored unless run via `xtask run-evdev-fixture-tests`. | Expected because these tests need fixture orchestration and wrapped Linux syscalls. |
| Perf-only panic | `safe/src/video/mod.rs:25` panics when the host SDL2 compatibility runtime is unavailable outside perf validation. | Deliberate guard around the host runtime path. |
| Test panic | `safe/tests/validator_surface_render.rs:52` panics on unsupported bytes-per-pixel while reading test pixels. | Test assertion helper; not production code. |
| Code generation template | `safe/xtask/src/contracts.rs:2739` generates aborting stubs that call `abort_unimplemented`. | This is the source of the generated remaining ABI stubs listed below. |
| Assert dialog state | `safe/src/core/assert.rs:83` and `safe/src/core/assert.rs:125` matched the issue grep because they assign `always_ignore`, not because they mark ignored tests. | Not an unresolved issue; it is SDL assert-state implementation. |
| Abort helper | `safe/src/lib.rs:31` defines `abort_unimplemented(symbol: &str) -> !` for generated stubs. | Known remaining compatibility surface: listed stubs abort deliberately rather than silently returning invalid values. |

**Prepared report coverage.**
| Area | Evidence and status |
| --- | --- |
| Unsafe audit | Checked-in `safe/generated/reports/unsafe-audit.json:7` and `safe/generated/reports/unsafe-audit.json:8` report 96 unsafe files and zero undocumented files. The current rerun to `/tmp/libsdl-port-unsafe-audit.json` also passed with zero undocumented files; it reports 98 files because it includes `safe/tests/validator_events_timers.rs` and `safe/tests/validator_surface_render.rs`. |
| Dependent matrix | `safe/generated/reports/dependent-matrix-results.json` reports 12/12 passing dependents with zero failures; `safe/generated/dependent_regression_manifest.json` has 0 issues. |
| Dependent list | `dependents.json` lists `qemu`, `ffmpeg`, `scrcpy`, `love`, `pygame`, `scummvm`, `supertuxkart`, `tuxpaint`, `openttd`, `0ad`, `imgui`, and `libtcod`. |
| Original tests | `safe/generated/original_test_port_map.json` has 116 entries and zero incomplete entries by `completion_state`. |
| CVE contract | `relevant_cves.json` lists `CVE-2022-4743`, `CVE-2020-14409`, `CVE-2019-13626`, and `CVE-2017-2888`; `safe/generated/cve_contract.json` records Rust-port focus areas for all four. |
| Performance | Accepted perf waivers are `renderer_safe_shim_copy_upload`, `audio_pure_rust_decode_resample`, and `events_safe_queue_bookkeeping` in `safe/generated/reports/perf-waivers.md:9`, `safe/generated/reports/perf-waivers.md:18`, and `safe/generated/reports/perf-waivers.md:27`; the measured CPU ratios are 2.060, 1.724, and 3.814 at `safe/generated/reports/perf-waivers.md:16`, `safe/generated/reports/perf-waivers.md:25`, and `safe/generated/reports/perf-waivers.md:34`. |
| Final package evidence | `safe/generated/reports/phase10-final-check.json:2` through `safe/generated/reports/phase10-final-check.json:13` record phase `impl_phase_10_packaging_dependents_final`, package version `2.30.0+dfsg-1ubuntu3.1+safelibs1`, built runtime/dev/tests debs, installed library `/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0`, and the checked-in dependent/audit report paths. |
| Validator matrix | `validator-report.md:13` through `validator-report.md:27` report the final local validator matrix passed with 85 cases, 85 passed, zero failed, and local override debs installed for 85/85 testcase result JSON files. |

**CVE mitigation status.**
| CVE | Risk class | Workspace evidence |
| --- | --- | --- |
| `CVE-2022-4743` | Renderer/GLES resource lifecycle | `safe/tests/security_gles_texture_lifecycle.rs:125` verifies successful streaming texture resources are released; `safe/tests/security_gles_texture_lifecycle.rs:207` through `safe/tests/security_gles_texture_lifecycle.rs:227` inject failures and checks created resources equal destroyed resources. |
| `CVE-2020-14409` | BMP blit/copy integer overflow class | `safe/tests/security_bmp_parser.rs:38` through `safe/tests/security_bmp_parser.rs:41` load crafted BMP data through `SDL_LoadBMP_RW`; `safe/tests/security_surface_math.rs:19` through `safe/tests/security_surface_math.rs:35` reject wrapping dimensions, pitches, and copy lengths. |
| `CVE-2019-13626` | WAVE IMA ADPCM overflow/over-read class | `safe/tests/security_wave_adpcm.rs:139` asserts impossible decompression requests fail before allocation. |
| `CVE-2017-2888` | RGB surface allocation overflow class | `safe/tests/security_surface_math.rs:43` through `safe/tests/security_surface_math.rs:50` assert overflowing RGB surface creation fails; `safe/tests/security_surface_math.rs:134` through `safe/tests/security_surface_math.rs:175` reject hostile null-pixel and pitch states before host calls. |

**Performance report status.**
| Workload | Status | Median CPU ratio | Peak allocation ratio | Waiver |
| --- | --- | --- | --- | --- |
| `surface_create_fill_convert_blit` | `pass` | 0.810 | 1.000 | `none` |
| `renderer_queue_copy_texture_upload` | `pass_with_waiver` | 2.060 | 1.000 | `renderer_safe_shim_copy_upload` |
| `audio_stream_convert_resample_wave` | `pass_with_waiver` | 1.724 | 1.000 | `audio_pure_rust_decode_resample` |
| `event_queue_throughput` | `pass_with_waiver` | 3.814 | 1.000 | `events_safe_queue_bookkeeping` |
| `controller_mapping_guid` | `pass` | 0.193 | 1.000 | `none` |

The final local validator report `validator-report.md:13` through `validator-report.md:27` is the package-level evidence for 85/85 passing cases with local override debs installed. The final package report `safe/generated/reports/phase10-final-check.json:4` through `safe/generated/reports/phase10-final-check.json:13` ties the built `libsdl2-2.0-0`, `libsdl2-dev`, and `libsdl2-tests` debs to the checked-in unsafe, dependent, and final report artifacts.

## Dependencies and other libraries used.
Resolved Rust dependency versions come from `/tmp/libsdl-port-cargo-metadata.json` and `/tmp/libsdl-port-cargo-tree.txt`; direct dependency declarations come from the Cargo manifests cited in the table.

| Crate/package | Dependency | Resolved version | Declaration evidence | Purpose |
| --- | --- | --- | --- | --- |
| `safe-sdl` normal | `libc` | 0.2.184 | `safe/Cargo.toml:20` through `safe/Cargo.toml:21` | C ABI, POSIX, math, loader, pthread, semaphore, memory, file, and syscall types/functions. |
| `safe-sdl` build | `cc` | 1.2.59 | `safe/Cargo.toml:23` through `safe/Cargo.toml:24` | Compiles `safe/src/core/phase2_variadic_shims.c` for variadic C ABI exports. |
| `safe-sdl` build | `serde` | 1.0.228 | `safe/Cargo.toml:23` through `safe/Cargo.toml:25` | Deserializes generated JSON manifests in the build script. |
| `safe-sdl` build | `serde_json` | 1.0.149 | `safe/Cargo.toml:23` through `safe/Cargo.toml:26` | Reads `safe/generated/linux_symbol_manifest.json` in `safe/build.rs`. |
| `safe-sdl` dev | `serde` | 1.0.228 | `safe/Cargo.toml:28` through `safe/Cargo.toml:29` | Integration tests and report parsing. |
| `safe-sdl` dev | `serde_json` | 1.0.149 | `safe/Cargo.toml:28` through `safe/Cargo.toml:30` | Integration tests and report parsing. |
| `safe-sdl` dev | `tempfile` | 3.27.0 | `safe/Cargo.toml:28` through `safe/Cargo.toml:31` | Temporary test directories and files. |
| `xtask` | `anyhow` | 1.0.102 | `safe/xtask/Cargo.toml:7` through `safe/xtask/Cargo.toml:8` | Tooling error context. |
| `xtask` | `bindgen` | 0.71.1 | `safe/xtask/Cargo.toml:7` through `safe/xtask/Cargo.toml:9` | Generates ABI declarations from original public headers. |
| `xtask` | `regex` | 1.12.3 | `safe/xtask/Cargo.toml:7` through `safe/xtask/Cargo.toml:10` | Contract parsing, source scanning, and generated code verification. |
| `xtask` | `serde` | 1.0.228 | `safe/xtask/Cargo.toml:7` through `safe/xtask/Cargo.toml:11` | Manifest/report serialization. |
| `xtask` | `serde_json` | 1.0.149 | `safe/xtask/Cargo.toml:7` through `safe/xtask/Cargo.toml:12` | Manifest/report JSON handling. |
| `xtask` | `tempfile` | 3.27.0 | `safe/xtask/Cargo.toml:7` through `safe/xtask/Cargo.toml:13` | Temporary staging and bindgen directories. |
| `safe-sdl2-test` normal | `libc` | 0.2.184 | `safe/sdl2-test/Cargo.toml:11` through `safe/sdl2-test/Cargo.toml:12` | C ABI types for the SDL2 test helper staticlib. |
| `safe-sdl2-test` build | `cc` | 1.2.59 | `safe/sdl2-test/Cargo.toml:14` through `safe/sdl2-test/Cargo.toml:15` | Compiles `safe/sdl2-test/src/variadic_shims.c`. |
| `safe-sdl2main` normal | `libc` | 0.2.184 | `safe/sdl2main/Cargo.toml:11` through `safe/sdl2main/Cargo.toml:12` | C ABI types for the SDL main staticlib. |

**FFI/OS-sensitive Rust dependencies and transitive crates.**
| Dependency | Resolved version | Tree evidence | Boundary risk |
| --- | --- | --- | --- |
| `clang-sys` | 1.8.1 | `bindgen` transitive dependency in `/tmp/libsdl-port-cargo-tree.txt` | Loads libclang and crosses the system C tooling boundary during contract capture. |
| `libloading` | 0.8.9 | `clang-sys` transitive dependency in `/tmp/libsdl-port-cargo-tree.txt` | Runtime dynamic loading support for libclang during bindgen execution. |
| `getrandom` | 0.4.2 | `tempfile` transitive dependency in `/tmp/libsdl-port-cargo-tree.txt` | Uses OS randomness for temporary-name generation. |
| `rustix` | 1.1.4 | `tempfile` transitive dependency in `/tmp/libsdl-port-cargo-tree.txt` | Uses Linux/Unix system interfaces behind safe APIs for temporary file handling. |
| `cc` | 1.2.59 | Direct build dependency in root and `sdl2-test` | Invokes a C compiler and links C objects into the Rust build. |
| `libc` | 0.2.184 | Direct dependency in root, `sdl2-test`, and `sdl2main`; transitive under `getrandom` and `clang-sys` | Primary runtime FFI boundary for POSIX, glibc, pthread, semaphore, loader, and math symbols. |

`cargo geiger --version` failed with `error: no such command: geiger`; cargo-geiger was not available, so no cargo-geiger unsafe-code report was produced in this workspace. Based on `/tmp/libsdl-port-cargo-tree.txt`, the dependencies that cross FFI or OS boundaries are `libc`, `bindgen`, `clang-sys`, `libloading`, `rustix`, `getrandom`, and `cc`; none of those should be treated as pure safe-Rust logic when auditing the port.

**System libraries observed in the release artifact.**
| Library | Evidence | Purpose |
| --- | --- | --- |
| `libgcc_s.so.1` | NEEDED from `/tmp/libsdl-port-readelf-dynamic.txt` | Rust/C unwinding support; undefined symbols include `_Unwind_*` in `/tmp/libsdl-port-undefined-symbols.txt`. |
| `libm.so.6` | NEEDED from `/tmp/libsdl-port-readelf-dynamic.txt` | Math functions such as `acos`, `sin`, `pow`, and related symbols in `/tmp/libsdl-port-undefined-symbols.txt`. |
| `libc.so.6` | NEEDED from `/tmp/libsdl-port-readelf-dynamic.txt` | C runtime, allocator, iconv, pthread, semaphore, dlopen/dlsym/dlclose, file, process, and syscall symbols. |
| `ld-linux-x86-64.so.2` | NEEDED from `/tmp/libsdl-port-readelf-dynamic.txt` | ELF dynamic loader support. |

**Build-time system dependencies.**
`debhelper-compat (= 13)`, `build-essential`, `cargo`, `cmake`, `dpkg-dev`, `fcitx-libs-dev`, `libasound2-dev`, `libdbus-1-dev`, `libdecor-0-dev`, `libdrm-dev`, `libegl-dev`, `libgbm-dev`, `libgl-dev`, `libgles-dev`, `libibus-1.0-dev`, `libpipewire-0.3-dev`, `libpulse-dev`, `libsamplerate0-dev`, `libsndio-dev`, `libudev-dev`, `libusb2-dev`, `libusbhid-dev`, `libvulkan-dev`, `libwayland-dev`, `libx11-dev`, `libxcursor-dev`, `libxext-dev`, `libxfixes-dev`, `libxi-dev`, `libxinerama-dev`, `libxkbcommon-dev`, `libxrandr-dev`, `libxss-dev`, `libxt-dev`, `libxv-dev`, `libxxf86vm-dev`, `pkg-config`, `rustc`, and `wayland-protocols` at `safe/debian/control:6` through `safe/debian/control:45`.

`safe/xtask/src/stage_install.rs:568` and `safe/xtask/src/stage_install.rs:1082` preserve installed link metadata containing `dl`, `m`, `pthread`, and `rt`, even though the current glibc-linked release artifact reports only `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and `ld-linux-x86-64.so.2` as direct NEEDED entries.

## How this document was produced.
This document was produced from checked-in source snapshots, generated manifests, generated reports, CVE data, dependent inventories, test manifests, install manifests, performance reports, unsafe audit reports, dependent matrix reports, the validator report, and the local `/tmp` command outputs listed below with the `libsdl-port-` prefix. No external source was fetched, no checked-in generated artifact was regenerated, and `.plan/plan.md` and files under `.plan/` were not modified.

**Temporary evidence files.**
| Path | Purpose |
| --- | --- |
| `/tmp/libsdl-port-cargo-metadata.json` | Resolved package metadata and versions. |
| `/tmp/libsdl-port-cargo-tree.txt` | Workspace dependency graph including normal, build, and dev dependencies. |
| `/tmp/libsdl-port-build-tools.txt` | Search evidence for build tools, link directives, crate types, bindgen/pkg-config, and absence of cbindgen. |
| `/tmp/libsdl-port-defined-symbols.txt` | Defined dynamic SDL symbols from the release artifact. |
| `/tmp/libsdl-port-undefined-symbols.txt` | Undefined dynamic system symbols from the release artifact. |
| `/tmp/libsdl-port-readelf-dynamic.txt` | SONAME, NEEDED libraries, and dynamic section flags. |
| `/tmp/libsdl-port-objdump-T.txt` | Dynamic symbol table and version evidence. |
| `/tmp/libsdl-port-unsafe-source.txt` | Exact unsafe line inventory used in this document. |
| `/tmp/libsdl-port-unsafe-grep.txt` | Broader unsafe grep coverage aid; Debian staging symlink warnings were ignored. |
| `/tmp/libsdl-port-unsafe-audit.json` | Fresh unsafe-audit pass against `safe/docs/unsafe-allowlist.md`. |
| `/tmp/libsdl-port-ffi-grep.txt` | FFI-boundary search evidence. |
| `/tmp/libsdl-port-issues-grep.txt` | Remaining issues search evidence. |

**Commands run before writing this document.**
```bash
git status --short
git rev-parse HEAD
test -f safe/PORT.md && sed -n '1,220p' safe/PORT.md || true
sed -n '1,220p' .plan/workflow-structure.yaml
find safe/src -maxdepth 3 -type f \( -name '*.rs' -o -name '*.c' \) | sort
cargo metadata --manifest-path safe/Cargo.toml --format-version 1 > /tmp/libsdl-port-cargo-metadata.json
cargo tree --manifest-path safe/Cargo.toml --workspace -e normal,build,dev > /tmp/libsdl-port-cargo-tree.txt
rg -n 'cbindgen|bindgen|pkg-config|pkg_config|cc::Build|crate-type|cargo:rustc-link' safe/Cargo.toml safe/build.rs safe/sdl2-test safe/sdl2main safe/xtask safe/generated safe/debian -g '!target' -g '!*.log' > /tmp/libsdl-port-build-tools.txt || true
cargo build --manifest-path safe/Cargo.toml --release
cargo run --manifest-path safe/Cargo.toml -p xtask -- abi-check --library safe/target/release/libsafe_sdl.so --require-soname libSDL2-2.0.so.0
nm -D --defined-only safe/target/release/libsafe_sdl.so > /tmp/libsdl-port-defined-symbols.txt
nm -D --undefined-only safe/target/release/libsafe_sdl.so > /tmp/libsdl-port-undefined-symbols.txt
readelf -d safe/target/release/libsafe_sdl.so > /tmp/libsdl-port-readelf-dynamic.txt
objdump -T safe/target/release/libsafe_sdl.so > /tmp/libsdl-port-objdump-T.txt
rg -n '\bunsafe(\s+extern|\s+fn|\s+impl|\s+trait|\s*\{)' safe/src safe/tests safe/sdl2main safe/xtask/src -g '*.rs' > /tmp/libsdl-port-unsafe-source.txt
grep -RIn --exclude-dir=target --exclude-dir=.git --exclude='*.log' '\bunsafe\b' safe > /tmp/libsdl-port-unsafe-grep.txt || true
cargo run --manifest-path safe/Cargo.toml -p xtask -- unsafe-audit --allowlist safe/docs/unsafe-allowlist.md --report /tmp/libsdl-port-unsafe-audit.json
rg -n 'unsafe extern "C" \{|dlopen|dlsym|dlclose|iconv|ioctl|libc::open|libc::read|libc::qsort|libc::bsearch|pthread|sem_|eglGetProcAddress|load_symbol|cc::Build|cargo:rustc-link' safe/src safe/build.rs safe/sdl2-test safe/sdl2main safe/xtask/src -g '*.rs' -g '*.c' > /tmp/libsdl-port-ffi-grep.txt
rg -n 'TODO|FIXME|todo!|unimplemented!|panic!|abort_unimplemented|#\[ignore|ignore =' safe/src safe/tests safe/xtask safe/docs safe/generated -g '!target' -g '!*.log' > /tmp/libsdl-port-issues-grep.txt || true
cargo geiger --version
```

The final sanity commands for this phase are recorded by the git history and terminal outputs after this file was written: `cargo fmt --manifest-path safe/Cargo.toml --all -- --check`, `cargo test --manifest-path safe/Cargo.toml --workspace --no-run`, `git diff --check`, and the final scoped git commit.
