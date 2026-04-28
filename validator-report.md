# Phase 4 Surface and Render Validator Report

Phase ID: `impl_phase_04_surface_render_fixes`

Date: 2026-04-28

## Outcome

- Prior phase evidence left three in-scope surface/transform failures: `usage-python3-pygame-alpha-blit`, `usage-python3-pygame-transform-scale`, and `usage-python3-pygame-transform-scale2x`.
- Root cause for the alpha-blit crash: safe-owned `SDL_Surface` values left the ABI-visible `surface->map` pointer null. Pygame can inspect SDL's blit-map state directly, especially for alpha/blend flags, so safe-created surfaces needed a compatible local blit-map shell.
- Root cause for the transform scale failures: safe-created `RGB888`/`BGR888` surfaces inherited the standalone 24-bit `SDL_AllocFormat` metadata. Pygame derives destination surfaces from `surface->format->BitsPerPixel` plus masks; with a 24-bit surface depth, `SDL_MasksToPixelFormatEnum` correctly remaps the masks to `BGR24`, triggering `ValueError: Source and destination surfaces need the same format.`
- Senior-checker regression fixed after the first implementation: standalone `SDL_AllocFormat` and `SDL_PixelFormatEnumToMasks` metadata for `RGB888`/`BGR888` now match SDL2's public contract (`BitsPerPixel = 24`, `BytesPerPixel = 4`, no alpha mask). Surface-owned `RGB888`/`BGR888` formats preserve a requested 32-bit depth only when the surface is created with depth `32`, which keeps pygame's derived destination format aligned without changing public pixel-format enum metadata.
- Additional local regression fixed while running existing surface tests: RGB-to-RGB `SDL_ConvertPixels` attempted to load host SDL YUV and pixel map/get symbols even when no YUV conversion was involved. The safe implementation now uses local pixel APIs for local RGB conversion.
- Required checker render regressions from the verification bounce were fixed:
  - `SDL_SetYUVConversionMode`, `SDL_GetYUVConversionMode`, and `SDL_GetYUVConversionModeForResolution` now use local SDL-compatible state instead of host SDL symbols.
  - Local software-renderer textures now retain the requested texture format while using an ARGB backing surface for YUV formats, and local `SDL_UpdateYUVTexture`/`SDL_UpdateNVTexture` convert planar/NV input into that backing surface.
  - A single explicitly requested host video driver such as `SDL_VIDEODRIVER=x11` can activate as a local stub when the host SDL runtime is unavailable; Vulkan loader paths report unavailable instead of failing video initialization, and the local render checker treats missing GL procedure lookup as an unavailable optional GL path.
- Added `safe/tests/validator_surface_render.rs` with direct SDL-level reproducers for alpha blit state, `RGB888`/`BGR888` public metadata, mask-derived scaled blit format compatibility, local YUV texture updates, and local YUV conversion-mode state.
- Updated `safe/src/video/surface.rs`, `safe/src/video/pixels.rs`, `safe/src/video/blit.rs`, `safe/src/render/local.rs`, `safe/src/video/display.rs`, `safe/src/video/window.rs`, `safe/tests/validator_surface_render.rs`, and the render regression test expectation in `safe/tests/original_apps_render.rs`.
- `safe/docs/unsafe-allowlist.md` did not need an update because the changed files remain covered by existing `safe/src/video/*.rs`, `safe/src/render/*.rs`, and `safe/tests/*.rs` entries.
- Full phase-04 validator run completed cleanly with validator exit code `0`: `85` cases, `85` passed, `0` failed, `5` source cases passed, `80` usage cases, `85` casts.
- Override install verification: all `85` testcase JSON files have `override_debs_installed: true`.
- Surface/render outcome: no surface/render validator failures remain in `validator/artifacts/.workspace/libsdl-safe-phase04/results/libsdl/`.
- True validator bug: none identified for surface/render behavior in this phase.
- The unrelated preexisting `original/src/joystick/__pycache__/` remains untouched and untracked.

## Commands Run

```bash
cargo fmt --manifest-path safe/Cargo.toml
```

```bash
cargo fmt --manifest-path safe/Cargo.toml --check
```

```bash
cargo test --manifest-path safe/Cargo.toml --test validator_surface_render -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test upstream_port_surface -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test original_apps_surface -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test security_surface_math --features host-video-tests -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test upstream_port_render --features host-video-tests -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test original_apps_render --features host-video-tests -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test security_gles_texture_lifecycle --features host-video-tests -- --test-threads=1
```

```bash
cargo run --manifest-path safe/Cargo.toml -p xtask -- security-regressions
```

```bash
cd safe
dpkg-buildpackage -us -uc -b
cd ..
```

```bash
rm -rf validator/artifacts/debs/local/libsdl
mkdir -p validator/artifacts/debs/local/libsdl
cp -v libsdl2-2.0-0_*.deb libsdl2-dev_*.deb libsdl2-tests_*.deb validator/artifacts/debs/local/libsdl/
python3 - <<'PY'
from pathlib import Path
import subprocess
root = Path("validator/artifacts/debs/local/libsdl")
packages = sorted(
    subprocess.check_output(["dpkg-deb", "--field", str(path), "Package"], text=True).strip()
    for path in root.glob("*.deb")
)
assert packages == ["libsdl2-2.0-0", "libsdl2-dev", "libsdl2-tests"], packages
PY
```

```bash
cd validator
rm -rf artifacts/.workspace/libsdl-safe-phase04
validator_status=0
bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/.workspace/libsdl-safe-phase04 \
  --mode original \
  --override-deb-root artifacts/debs/local \
  --library libsdl \
  --record-casts || validator_status=$?
echo "validator_status=${validator_status}"
exit ${validator_status}
```

```bash
python3 - <<'PY'
from pathlib import Path
import json
root = Path("validator/artifacts/.workspace/libsdl-safe-phase04/results/libsdl")
summary = json.loads((root / "summary.json").read_text())
results = []
missing_override = []
not_passed = []
for path in sorted(root.glob("*.json")):
    if path.name == "summary.json":
        continue
    data = json.loads(path.read_text())
    results.append(data)
    if data.get("override_debs_installed") is not True:
        missing_override.append(data.get("testcase_id"))
    if data.get("status") != "passed":
        not_passed.append((data.get("testcase_id"), data.get("status"), data.get("exit_code")))
assert summary["cases"] == 85, summary
assert summary["passed"] == 85 and summary["failed"] == 0, summary
assert len(results) == 85, len(results)
assert not missing_override, missing_override
assert not not_passed, not_passed
PY
```

## Local Override Debs

Artifact directory: `validator/artifacts/debs/local/libsdl/`

| File | Package | Version | Architecture | SHA256 |
| --- | --- | --- | --- | --- |
| `libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-2.0-0` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `48bda642be7d4bd70cfae450c2db3d3ebfc0dd33e11e2a416de067a884db965b` |
| `libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-dev` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `1c35bf70b2cb508afc6cefebbfdc063b4879643476cfcc5540c22583a3fb47ad` |
| `libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-tests` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `6d9e7172e5c48d7a0f831aacf64b37dc61ef06eb78e13554c9cab5c520e5af66` |

## Surface and render results

All previously failing surface/transform cases passed in `validator/artifacts/.workspace/libsdl-safe-phase04/results/libsdl/`.

| Case ID | Status | Exit Code | Override Debs Installed |
| --- | --- | --- | --- |
| `usage-python3-pygame-alpha-blit` | `passed` | `0` | `true` |
| `usage-python3-pygame-transform-scale` | `passed` | `0` | `true` |
| `usage-python3-pygame-transform-scale2x` | `passed` | `0` | `true` |

No other surface, pixel, blit, image, mask, display, window, render, texture, draw, copy, or present validator case failed in the phase-04 run.

Required checker render tests also passed locally:

| Checker | Status |
| --- | --- |
| `original_apps_render` with `host-video-tests` | `passed` |
| `upstream_port_render` with `host-video-tests` | `passed` |

## Raw Artifacts

- Results: `validator/artifacts/.workspace/libsdl-safe-phase04/results/libsdl/`
- Logs: `validator/artifacts/.workspace/libsdl-safe-phase04/logs/libsdl/`
- Casts: `validator/artifacts/.workspace/libsdl-safe-phase04/casts/libsdl/`
- Summary JSON: `validator/artifacts/.workspace/libsdl-safe-phase04/results/libsdl/summary.json`

## Preexisting Input Handling

The prepared source snapshots, generated contracts/manifests, CVE data, dependent inventories, performance evidence, dependent regression reports, unsafe audit report, existing integration tests, prior validator artifacts, and upstream test tree were consumed in place. I did not refetch, recollect, rediscover, or regenerate those checked-in artifacts.
