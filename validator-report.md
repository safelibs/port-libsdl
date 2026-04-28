# Phase 4 Surface and Render Validator Report

Phase ID: `impl_phase_04_surface_render_fixes`

Date: 2026-04-28

## Outcome

- Prior phase evidence left three in-scope surface/transform failures: `usage-python3-pygame-alpha-blit`, `usage-python3-pygame-transform-scale`, and `usage-python3-pygame-transform-scale2x`.
- Root cause for the alpha-blit crash: safe-owned `SDL_Surface` values left the ABI-visible `surface->map` pointer null. Pygame can inspect SDL's blit-map state directly, especially for alpha/blend flags, so safe-created surfaces needed a compatible local blit-map shell.
- Root cause for the transform scale failures: local `SDL_AllocFormat` reported packed 4-byte `RGB888`/`BGR888` formats as 24-bit formats. Pygame derives destination surfaces from masks and bit depth; the wrong bit depth remapped `RGB888` to `BGR24`, triggering `ValueError: Source and destination surfaces need the same format.`
- Additional local regression fixed while running existing surface tests: RGB-to-RGB `SDL_ConvertPixels` attempted to load host SDL YUV and pixel map/get symbols even when no YUV conversion was involved. The safe implementation now uses local pixel APIs for local RGB conversion.
- Added `safe/tests/validator_surface_render.rs` with direct SDL-level reproducers for alpha blit state and mask-derived scaled blit format compatibility.
- Updated `safe/src/video/surface.rs`, `safe/src/video/pixels.rs`, and `safe/src/video/blit.rs`; no render module changes were needed.
- `safe/docs/unsafe-allowlist.md` did not need an update because the changed files remain covered by existing `safe/src/video/*.rs` and `safe/tests/*.rs` entries.
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
cargo test --manifest-path safe/Cargo.toml --test validator_surface_render -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test upstream_port_surface -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test security_surface_math -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test upstream_port_render -- --test-threads=1
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
| `libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-2.0-0` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `33f129b8c8f85c66d8b11f967f1c9193893949b0f0d9b4520ade5e7ac707c96f` |
| `libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-dev` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `c54a8c35e47c97bbd02a93efbb90e46040b43b3ac054181e40718579b1c2361b` |
| `libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-tests` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `6d9e7172e5c48d7a0f831aacf64b37dc61ef06eb78e13554c9cab5c520e5af66` |

## Surface and Render Results

All previously failing surface/transform cases passed in `validator/artifacts/.workspace/libsdl-safe-phase04/results/libsdl/`.

| Case ID | Status | Exit Code | Override Debs Installed |
| --- | --- | --- | --- |
| `usage-python3-pygame-alpha-blit` | `passed` | `0` | `true` |
| `usage-python3-pygame-transform-scale` | `passed` | `0` | `true` |
| `usage-python3-pygame-transform-scale2x` | `passed` | `0` | `true` |

No other surface, pixel, blit, image, mask, display, window, render, texture, draw, copy, or present validator case failed in the phase-04 run.

## Raw Artifacts

- Results: `validator/artifacts/.workspace/libsdl-safe-phase04/results/libsdl/`
- Logs: `validator/artifacts/.workspace/libsdl-safe-phase04/logs/libsdl/`
- Casts: `validator/artifacts/.workspace/libsdl-safe-phase04/casts/libsdl/`
- Summary JSON: `validator/artifacts/.workspace/libsdl-safe-phase04/results/libsdl/summary.json`

## Preexisting Input Handling

The prepared source snapshots, generated contracts/manifests, CVE data, dependent inventories, performance evidence, dependent regression reports, unsafe audit report, existing integration tests, prior validator artifacts, and upstream test tree were consumed in place. I did not refetch, recollect, rediscover, or regenerate those checked-in artifacts.
