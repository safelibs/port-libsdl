# Phase 2 Source and Packaging Validator Report

Phase ID: `impl_phase_02_source_packaging_fixes`

Date: 2026-04-28

## Outcome

- Source/packaging classification: no source/packaging failures in baseline.
- Fresh phase-02 validator evidence also has no source/packaging failures: all `5` source cases passed.
- No `safe/tests/validator_packaging_contract.rs` was created because there was no source, ABI, header, `sdl2-config`, pkg-config, installed-test, or exported-symbol failure to regress.
- No `safe/` source, packaging, generated contract, or unsafe allowlist files were changed.
- Local safe package build: passed.
- Local override package verification: passed for exactly `libsdl2-2.0-0`, `libsdl2-dev`, and `libsdl2-tests`.
- ABI check: passed against `safe/target/release/libsafe_sdl.so` with required SONAME `libSDL2-2.0.so.0`.
- Install contract check: passed against `safe/debian/tmp`.
- Full phase-02 validator run: completed with validator exit code `1`.
- Result summary: `85` cases, `75` passed, `10` failed, `5` source cases passed, `80` usage cases, `85` casts.
- Override install verification: all `85` testcase JSON files have `override_debs_installed: true`.
- True validator bug: none identified for source/packaging in this phase.

The remaining validator failures are usage-level pygame behavior failures and are outside the source/packaging scope for this phase.
The unrelated preexisting `original/src/joystick/__pycache__/` remains untouched and untracked.

## Commands Run

```bash
dpkg-checkbuilddeps safe/debian/control
```

```bash
rm -f libsdl2*.deb libsdl2*.ddeb libsdl2*.buildinfo libsdl2*.changes
cd safe
dpkg-buildpackage -us -uc -b
cd ..
```

```bash
rm -rf validator/artifacts/debs/local/libsdl
mkdir -p validator/artifacts/debs/local/libsdl
cp -v libsdl2*.deb validator/artifacts/debs/local/libsdl/
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
cargo run --manifest-path safe/Cargo.toml -p xtask -- \
  abi-check \
  --generated safe/generated \
  --library safe/target/release/libsafe_sdl.so \
  --require-soname libSDL2-2.0.so.0
```

```bash
cargo run --manifest-path safe/Cargo.toml -p xtask -- \
  verify-install-contract \
  --generated safe/generated \
  --original original \
  --package-root safe/debian/tmp \
  --mode staged
```

```bash
cd validator
. .work/venv/bin/activate
rm -rf artifacts/.workspace/libsdl-safe-phase02
validator_status=0
bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/.workspace/libsdl-safe-phase02 \
  --mode original \
  --override-deb-root artifacts/debs/local \
  --library libsdl \
  --record-casts || validator_status=$?
cd ..
```

## Local Override Debs

Artifact directory: `validator/artifacts/debs/local/libsdl/`

| File | Package | Version | Architecture | SHA256 |
| --- | --- | --- | --- | --- |
| `libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-2.0-0` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `635c09f0a1254027e76381c3827fc5a40fe2d521d2b8f76144253ac19a8cf363` |
| `libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-dev` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `b66769e5b697d261e8ae657fbbc8f9e1a0a667a84437f73a152a69377f3931fd` |
| `libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-tests` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `6d9e7172e5c48d7a0f831aacf64b37dc61ef06eb78e13554c9cab5c520e5af66` |

## Raw Artifacts

- Results: `validator/artifacts/.workspace/libsdl-safe-phase02/results/libsdl/`
- Logs: `validator/artifacts/.workspace/libsdl-safe-phase02/logs/libsdl/`
- Casts: `validator/artifacts/.workspace/libsdl-safe-phase02/casts/libsdl/`
- Summary JSON: `validator/artifacts/.workspace/libsdl-safe-phase02/results/libsdl/summary.json`

## Source and packaging Results

No source/packaging failures in baseline or in the phase-02 rerun.

| Case ID | Kind | Status | Exit Code | Override Debs Installed |
| --- | --- | --- | --- | --- |
| `dummy-audio-queue` | `source` | `passed` | `0` | `true` |
| `headless-event-timer` | `source` | `passed` | `0` | `true` |
| `installed-test-binary` | `source` | `passed` | `0` | `true` |
| `surface-blit-pixel-format` | `source` | `passed` | `0` | `true` |
| `version-query-compile` | `source` | `passed` | `0` | `true` |

## Remaining Usage Failures

| Case ID | Symptom |
| --- | --- |
| `usage-python3-pygame-alpha-blit` | Exit `139`; segmentation fault in the Python pygame alpha-blit script. |
| `usage-python3-pygame-custom-event` | Timed out after `180` seconds. |
| `usage-python3-pygame-event-clear` | Timed out after `120` seconds. |
| `usage-python3-pygame-event-peek` | Timed out after `180` seconds. |
| `usage-python3-pygame-event-queue` | Timed out after `180` seconds. |
| `usage-python3-pygame-key-event` | Timed out after `180` seconds. |
| `usage-python3-pygame-mouse-event` | Timed out after `180` seconds. |
| `usage-python3-pygame-timer-event` | Timed out after `180` seconds. |
| `usage-python3-pygame-transform-scale` | Exit `1`; traceback reports `ValueError: Source and destination surfaces need the same format.` |
| `usage-python3-pygame-transform-scale2x` | Exit `1`; traceback reports `ValueError: Source and destination surfaces need the same format.` |

## Preexisting Input Handling

The prepared source snapshots, generated contracts/manifests, CVE data, dependent inventories, performance evidence, dependent regression reports, unsafe audit report, existing integration tests, and upstream test tree were consumed in place. I did not refetch, recollect, rediscover, or regenerate those checked-in artifacts.
