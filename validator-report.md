# Phase 1 Validator Baseline Report

Phase ID: `impl_phase_01_validator_checkout_baseline`

Date: 2026-04-28

## Outcome

- Validator checkout: `validator/`
- Validator commit: `1319bb0374ef66428a42dd71e49553c6d057feaf`
- Safe port commit: `14609a9a584435d4a12c46aa11e969a0f0f8a7d5`
- Validator unit tests: passed, `112` tests.
- Validator libsdl manifest check: passed with `85` cases, `5` source cases, and `80` usage cases.
- Local safe package build: passed.
- Local override package verification: passed for exactly `libsdl2-2.0-0`, `libsdl2-dev`, and `libsdl2-tests`.
- Baseline validator run: completed with validator exit code `1`.
- Result summary: `85` cases, `75` passed, `10` failed, `85` casts.
- Override install verification: all `85` testcase JSON files have `override_debs_installed: true`.
- True validator bug: none identified in this phase. The failures are recorded as local safe override compatibility symptoms.

No `safe/` source files were changed. The unrelated preexisting `original/src/joystick/__pycache__/` remains untouched and untracked.

## Environment Setup

- `dpkg-checkbuilddeps safe/debian/control` returned success before package build.
- No host apt package installation was needed.
- Validator Python tooling was installed only under `validator/.work/venv/`.
- Python packages installed in the validator venv: upgraded `pip`, installed `PyYAML`.

## Commands Run

```bash
if [ -d validator/.git ]; then
  git -C validator pull --ff-only
else
  git clone https://github.com/safelibs/validator validator
fi
git -C validator rev-parse HEAD
```

```bash
cd validator
python3 -m venv .work/venv
. .work/venv/bin/activate
python -m pip install --upgrade pip PyYAML
python -m unittest discover -s unit -v
python tools/testcases.py --config repositories.yml --tests-root tests --check --library libsdl --min-source-cases 5 --min-usage-cases 80 --min-cases 85
cd ..
```

```bash
dpkg-checkbuilddeps safe/debian/control
rm -f libsdl2*.deb libsdl2*.ddeb libsdl2*.buildinfo libsdl2*.changes
cd safe
dpkg-buildpackage -us -uc -b
cd ..
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
cd validator
. .work/venv/bin/activate
rm -rf artifacts/.workspace/libsdl-safe
validator_status=0
bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/.workspace/libsdl-safe \
  --mode original \
  --override-deb-root artifacts/debs/local \
  --library libsdl \
  --record-casts || validator_status=$?
printf '%s\n' "$validator_status" > artifacts/.workspace/libsdl-safe/validator-exit-code.txt
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

- Results: `validator/artifacts/.workspace/libsdl-safe/results/libsdl/`
- Logs: `validator/artifacts/.workspace/libsdl-safe/logs/libsdl/`
- Casts: `validator/artifacts/.workspace/libsdl-safe/casts/libsdl/`
- Validator exit code: `validator/artifacts/.workspace/libsdl-safe/validator-exit-code.txt`
- Summary JSON: `validator/artifacts/.workspace/libsdl-safe/results/libsdl/summary.json`

## Failures

| Case ID | Symptom |
| --- | --- |
| `usage-python3-pygame-alpha-blit` | Exit `139`; log shows `Segmentation fault (core dumped)` in the Python pygame alpha-blit script. |
| `usage-python3-pygame-custom-event` | Timed out after `180` seconds after pygame startup. |
| `usage-python3-pygame-event-clear` | Timed out after `120` seconds after pygame startup. |
| `usage-python3-pygame-event-peek` | Timed out after `180` seconds after pygame startup. |
| `usage-python3-pygame-event-queue` | Timed out after `180` seconds after pygame startup. |
| `usage-python3-pygame-key-event` | Timed out after `180` seconds after pygame startup. |
| `usage-python3-pygame-mouse-event` | Timed out after `180` seconds after pygame startup. |
| `usage-python3-pygame-timer-event` | Timed out after `180` seconds after pygame startup. |
| `usage-python3-pygame-transform-scale` | Exit `1`; traceback reports `ValueError: Source and destination surfaces need the same format.` |
| `usage-python3-pygame-transform-scale2x` | Exit `1`; traceback reports `ValueError: Source and destination surfaces need the same format.` |

## Case Results

| Case ID | Kind | Status | Command | Log | Cast | Observed symptom |
| --- | --- | --- | --- | --- | --- | --- |
| `dummy-audio-queue` | `source` | `passed` | `bash /validator/tests/libsdl/tests/cases/source/dummy-audio-queue.sh` | `logs/libsdl/dummy-audio-queue.log` | `casts/libsdl/dummy-audio-queue.cast` | passed |
| `headless-event-timer` | `source` | `passed` | `bash /validator/tests/libsdl/tests/cases/source/headless-event-timer.sh` | `logs/libsdl/headless-event-timer.log` | `casts/libsdl/headless-event-timer.cast` | passed |
| `installed-test-binary` | `source` | `passed` | `bash /validator/tests/libsdl/tests/cases/source/installed-test-binary.sh` | `logs/libsdl/installed-test-binary.log` | `casts/libsdl/installed-test-binary.cast` | passed |
| `surface-blit-pixel-format` | `source` | `passed` | `bash /validator/tests/libsdl/tests/cases/source/surface-blit-pixel-format.sh` | `logs/libsdl/surface-blit-pixel-format.log` | `casts/libsdl/surface-blit-pixel-format.cast` | passed |
| `usage-python3-pygame-alpha-blit` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-alpha-blit` | `logs/libsdl/usage-python3-pygame-alpha-blit.log` | `casts/libsdl/usage-python3-pygame-alpha-blit.cast` | testcase command exited with status 139; log shows segmentation fault/core dumped in python pygame alpha-blit script. |
| `usage-python3-pygame-audio-dummy` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-audio-dummy.sh` | `logs/libsdl/usage-python3-pygame-audio-dummy.log` | `casts/libsdl/usage-python3-pygame-audio-dummy.cast` | passed |
| `usage-python3-pygame-clock-tick` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-clock-tick` | `logs/libsdl/usage-python3-pygame-clock-tick.log` | `casts/libsdl/usage-python3-pygame-clock-tick.cast` | passed |
| `usage-python3-pygame-color-hsva` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-color-hsva` | `logs/libsdl/usage-python3-pygame-color-hsva.log` | `casts/libsdl/usage-python3-pygame-color-hsva.cast` | passed |
| `usage-python3-pygame-color-lerp` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-color-lerp` | `logs/libsdl/usage-python3-pygame-color-lerp.log` | `casts/libsdl/usage-python3-pygame-color-lerp.cast` | passed |
| `usage-python3-pygame-custom-event` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-custom-event` | `logs/libsdl/usage-python3-pygame-custom-event.log` | `casts/libsdl/usage-python3-pygame-custom-event.cast` | testcase timed out after 180 seconds; log reaches pygame import/runtime warning and then validator timeout. |
| `usage-python3-pygame-display-caption` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-display-caption` | `logs/libsdl/usage-python3-pygame-display-caption.log` | `casts/libsdl/usage-python3-pygame-display-caption.cast` | passed |
| `usage-python3-pygame-display-get-driver` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-display-get-driver` | `logs/libsdl/usage-python3-pygame-display-get-driver.log` | `casts/libsdl/usage-python3-pygame-display-get-driver.cast` | passed |
| `usage-python3-pygame-display-set-mode` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-display-set-mode` | `logs/libsdl/usage-python3-pygame-display-set-mode.log` | `casts/libsdl/usage-python3-pygame-display-set-mode.cast` | passed |
| `usage-python3-pygame-draw-aaline` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-draw-aaline` | `logs/libsdl/usage-python3-pygame-draw-aaline.log` | `casts/libsdl/usage-python3-pygame-draw-aaline.cast` | passed |
| `usage-python3-pygame-draw-arc` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-draw-arc` | `logs/libsdl/usage-python3-pygame-draw-arc.log` | `casts/libsdl/usage-python3-pygame-draw-arc.cast` | passed |
| `usage-python3-pygame-draw-circle` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-draw-circle` | `logs/libsdl/usage-python3-pygame-draw-circle.log` | `casts/libsdl/usage-python3-pygame-draw-circle.cast` | passed |
| `usage-python3-pygame-draw-ellipse` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-draw-ellipse` | `logs/libsdl/usage-python3-pygame-draw-ellipse.log` | `casts/libsdl/usage-python3-pygame-draw-ellipse.cast` | passed |
| `usage-python3-pygame-draw-line` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-draw-line` | `logs/libsdl/usage-python3-pygame-draw-line.log` | `casts/libsdl/usage-python3-pygame-draw-line.cast` | passed |
| `usage-python3-pygame-draw-polygon` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-draw-polygon` | `logs/libsdl/usage-python3-pygame-draw-polygon.log` | `casts/libsdl/usage-python3-pygame-draw-polygon.cast` | passed |
| `usage-python3-pygame-draw-rect` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-draw-rect.sh` | `logs/libsdl/usage-python3-pygame-draw-rect.log` | `casts/libsdl/usage-python3-pygame-draw-rect.cast` | passed |
| `usage-python3-pygame-event-clear` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-event-clear` | `logs/libsdl/usage-python3-pygame-event-clear.log` | `casts/libsdl/usage-python3-pygame-event-clear.cast` | testcase timed out after 120 seconds; log reaches pygame import/runtime warning and then validator timeout. |
| `usage-python3-pygame-event-name-keydown` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-event-name-keydown` | `logs/libsdl/usage-python3-pygame-event-name-keydown.log` | `casts/libsdl/usage-python3-pygame-event-name-keydown.cast` | passed |
| `usage-python3-pygame-event-peek` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-event-peek` | `logs/libsdl/usage-python3-pygame-event-peek.log` | `casts/libsdl/usage-python3-pygame-event-peek.cast` | testcase timed out after 180 seconds; log reaches pygame import/runtime warning and then validator timeout. |
| `usage-python3-pygame-event-queue` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-event-queue.sh` | `logs/libsdl/usage-python3-pygame-event-queue.log` | `casts/libsdl/usage-python3-pygame-event-queue.cast` | testcase timed out after 180 seconds; log reaches pygame import/runtime warning and then validator timeout. |
| `usage-python3-pygame-event-set-blocked` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-event-set-blocked` | `logs/libsdl/usage-python3-pygame-event-set-blocked.log` | `casts/libsdl/usage-python3-pygame-event-set-blocked.cast` | passed |
| `usage-python3-pygame-font-linesize` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-font-linesize` | `logs/libsdl/usage-python3-pygame-font-linesize.log` | `casts/libsdl/usage-python3-pygame-font-linesize.cast` | passed |
| `usage-python3-pygame-font-metrics` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-font-metrics` | `logs/libsdl/usage-python3-pygame-font-metrics.log` | `casts/libsdl/usage-python3-pygame-font-metrics.cast` | passed |
| `usage-python3-pygame-font-module` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-font-module.sh` | `logs/libsdl/usage-python3-pygame-font-module.log` | `casts/libsdl/usage-python3-pygame-font-module.cast` | passed |
| `usage-python3-pygame-font-render` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-font-render` | `logs/libsdl/usage-python3-pygame-font-render.log` | `casts/libsdl/usage-python3-pygame-font-render.cast` | passed |
| `usage-python3-pygame-font-size` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-font-size` | `logs/libsdl/usage-python3-pygame-font-size.log` | `casts/libsdl/usage-python3-pygame-font-size.cast` | passed |
| `usage-python3-pygame-image-fromstring` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-image-fromstring` | `logs/libsdl/usage-python3-pygame-image-fromstring.log` | `casts/libsdl/usage-python3-pygame-image-fromstring.cast` | passed |
| `usage-python3-pygame-image-load` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-image-load` | `logs/libsdl/usage-python3-pygame-image-load.log` | `casts/libsdl/usage-python3-pygame-image-load.cast` | passed |
| `usage-python3-pygame-image-save-bmp` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-image-save-bmp` | `logs/libsdl/usage-python3-pygame-image-save-bmp.log` | `casts/libsdl/usage-python3-pygame-image-save-bmp.cast` | passed |
| `usage-python3-pygame-image-save` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-image-save.sh` | `logs/libsdl/usage-python3-pygame-image-save.log` | `casts/libsdl/usage-python3-pygame-image-save.cast` | passed |
| `usage-python3-pygame-image-tostring` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-image-tostring` | `logs/libsdl/usage-python3-pygame-image-tostring.log` | `casts/libsdl/usage-python3-pygame-image-tostring.cast` | passed |
| `usage-python3-pygame-key-event` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-key-event.sh` | `logs/libsdl/usage-python3-pygame-key-event.log` | `casts/libsdl/usage-python3-pygame-key-event.cast` | testcase timed out after 180 seconds; log reaches pygame import/runtime warning and then validator timeout. |
| `usage-python3-pygame-mask-bounding-rects` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-mask-bounding-rects` | `logs/libsdl/usage-python3-pygame-mask-bounding-rects.log` | `casts/libsdl/usage-python3-pygame-mask-bounding-rects.cast` | passed |
| `usage-python3-pygame-mask-centroid` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-mask-centroid` | `logs/libsdl/usage-python3-pygame-mask-centroid.log` | `casts/libsdl/usage-python3-pygame-mask-centroid.cast` | passed |
| `usage-python3-pygame-mask-collision` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-mask-collision.sh` | `logs/libsdl/usage-python3-pygame-mask-collision.log` | `casts/libsdl/usage-python3-pygame-mask-collision.cast` | passed |
| `usage-python3-pygame-mask-count-tenth` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-mask-count-tenth` | `logs/libsdl/usage-python3-pygame-mask-count-tenth.log` | `casts/libsdl/usage-python3-pygame-mask-count-tenth.cast` | passed |
| `usage-python3-pygame-mask-count` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-mask-count` | `logs/libsdl/usage-python3-pygame-mask-count.log` | `casts/libsdl/usage-python3-pygame-mask-count.cast` | passed |
| `usage-python3-pygame-mask-from-threshold` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-mask-from-threshold` | `logs/libsdl/usage-python3-pygame-mask-from-threshold.log` | `casts/libsdl/usage-python3-pygame-mask-from-threshold.cast` | passed |
| `usage-python3-pygame-mask-outline` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-mask-outline` | `logs/libsdl/usage-python3-pygame-mask-outline.log` | `casts/libsdl/usage-python3-pygame-mask-outline.cast` | passed |
| `usage-python3-pygame-mask-overlap-area` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-mask-overlap-area` | `logs/libsdl/usage-python3-pygame-mask-overlap-area.log` | `casts/libsdl/usage-python3-pygame-mask-overlap-area.cast` | passed |
| `usage-python3-pygame-mouse-event` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-mouse-event` | `logs/libsdl/usage-python3-pygame-mouse-event.log` | `casts/libsdl/usage-python3-pygame-mouse-event.cast` | testcase timed out after 180 seconds; log reaches pygame import/runtime warning and then validator timeout. |
| `usage-python3-pygame-pixelarray` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-pixelarray` | `logs/libsdl/usage-python3-pygame-pixelarray.log` | `casts/libsdl/usage-python3-pygame-pixelarray.cast` | passed |
| `usage-python3-pygame-rect-clamp-ip` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-rect-clamp-ip` | `logs/libsdl/usage-python3-pygame-rect-clamp-ip.log` | `casts/libsdl/usage-python3-pygame-rect-clamp-ip.cast` | passed |
| `usage-python3-pygame-rect-clamp` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-rect-clamp` | `logs/libsdl/usage-python3-pygame-rect-clamp.log` | `casts/libsdl/usage-python3-pygame-rect-clamp.cast` | passed |
| `usage-python3-pygame-rect-clip` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-rect-clip` | `logs/libsdl/usage-python3-pygame-rect-clip.log` | `casts/libsdl/usage-python3-pygame-rect-clip.cast` | passed |
| `usage-python3-pygame-rect-collision` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-rect-collision` | `logs/libsdl/usage-python3-pygame-rect-collision.log` | `casts/libsdl/usage-python3-pygame-rect-collision.cast` | passed |
| `usage-python3-pygame-rect-contains` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-rect-contains` | `logs/libsdl/usage-python3-pygame-rect-contains.log` | `casts/libsdl/usage-python3-pygame-rect-contains.cast` | passed |
| `usage-python3-pygame-rect-fit` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-rect-fit` | `logs/libsdl/usage-python3-pygame-rect-fit.log` | `casts/libsdl/usage-python3-pygame-rect-fit.cast` | passed |
| `usage-python3-pygame-rect-inflate-ip` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-rect-inflate-ip` | `logs/libsdl/usage-python3-pygame-rect-inflate-ip.log` | `casts/libsdl/usage-python3-pygame-rect-inflate-ip.cast` | passed |
| `usage-python3-pygame-rect-move-ip` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-rect-move-ip` | `logs/libsdl/usage-python3-pygame-rect-move-ip.log` | `casts/libsdl/usage-python3-pygame-rect-move-ip.cast` | passed |
| `usage-python3-pygame-rect-normalize` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-rect-normalize` | `logs/libsdl/usage-python3-pygame-rect-normalize.log` | `casts/libsdl/usage-python3-pygame-rect-normalize.cast` | passed |
| `usage-python3-pygame-rect-union` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-rect-union` | `logs/libsdl/usage-python3-pygame-rect-union.log` | `casts/libsdl/usage-python3-pygame-rect-union.cast` | passed |
| `usage-python3-pygame-subsurface-size` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-subsurface-size` | `logs/libsdl/usage-python3-pygame-subsurface-size.log` | `casts/libsdl/usage-python3-pygame-subsurface-size.cast` | passed |
| `usage-python3-pygame-surface-bounding-rect` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-surface-bounding-rect` | `logs/libsdl/usage-python3-pygame-surface-bounding-rect.log` | `casts/libsdl/usage-python3-pygame-surface-bounding-rect.cast` | passed |
| `usage-python3-pygame-surface-clip-fill` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-surface-clip-fill` | `logs/libsdl/usage-python3-pygame-surface-clip-fill.log` | `casts/libsdl/usage-python3-pygame-surface-clip-fill.cast` | passed |
| `usage-python3-pygame-surface-colorkey` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-surface-colorkey` | `logs/libsdl/usage-python3-pygame-surface-colorkey.log` | `casts/libsdl/usage-python3-pygame-surface-colorkey.cast` | passed |
| `usage-python3-pygame-surface-copy` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-surface-copy` | `logs/libsdl/usage-python3-pygame-surface-copy.log` | `casts/libsdl/usage-python3-pygame-surface-copy.cast` | passed |
| `usage-python3-pygame-surface-fill-rect` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-surface-fill-rect` | `logs/libsdl/usage-python3-pygame-surface-fill-rect.log` | `casts/libsdl/usage-python3-pygame-surface-fill-rect.cast` | passed |
| `usage-python3-pygame-surface-fill` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-surface-fill.sh` | `logs/libsdl/usage-python3-pygame-surface-fill.log` | `casts/libsdl/usage-python3-pygame-surface-fill.cast` | passed |
| `usage-python3-pygame-surface-map-rgb` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-surface-map-rgb` | `logs/libsdl/usage-python3-pygame-surface-map-rgb.log` | `casts/libsdl/usage-python3-pygame-surface-map-rgb.cast` | passed |
| `usage-python3-pygame-surface-scroll` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-surface-scroll` | `logs/libsdl/usage-python3-pygame-surface-scroll.log` | `casts/libsdl/usage-python3-pygame-surface-scroll.cast` | passed |
| `usage-python3-pygame-surface-set-colorkey` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-surface-set-colorkey` | `logs/libsdl/usage-python3-pygame-surface-set-colorkey.log` | `casts/libsdl/usage-python3-pygame-surface-set-colorkey.cast` | passed |
| `usage-python3-pygame-surface-subsurface` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-surface-subsurface` | `logs/libsdl/usage-python3-pygame-surface-subsurface.log` | `casts/libsdl/usage-python3-pygame-surface-subsurface.cast` | passed |
| `usage-python3-pygame-surfarray` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-surfarray` | `logs/libsdl/usage-python3-pygame-surfarray.log` | `casts/libsdl/usage-python3-pygame-surfarray.cast` | passed |
| `usage-python3-pygame-time-delay` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-time-delay` | `logs/libsdl/usage-python3-pygame-time-delay.log` | `casts/libsdl/usage-python3-pygame-time-delay.cast` | passed |
| `usage-python3-pygame-time-wait` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-time-wait` | `logs/libsdl/usage-python3-pygame-time-wait.log` | `casts/libsdl/usage-python3-pygame-time-wait.cast` | passed |
| `usage-python3-pygame-timer-event` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-timer-event` | `logs/libsdl/usage-python3-pygame-timer-event.log` | `casts/libsdl/usage-python3-pygame-timer-event.cast` | testcase timed out after 180 seconds; log reaches pygame import/runtime warning and then validator timeout. |
| `usage-python3-pygame-transform-average-color` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-transform-average-color` | `logs/libsdl/usage-python3-pygame-transform-average-color.log` | `casts/libsdl/usage-python3-pygame-transform-average-color.cast` | passed |
| `usage-python3-pygame-transform-flip` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-transform-flip` | `logs/libsdl/usage-python3-pygame-transform-flip.log` | `casts/libsdl/usage-python3-pygame-transform-flip.cast` | passed |
| `usage-python3-pygame-transform-rotate` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-extra-client-behavior.sh usage-python3-pygame-transform-rotate` | `logs/libsdl/usage-python3-pygame-transform-rotate.log` | `casts/libsdl/usage-python3-pygame-transform-rotate.cast` | passed |
| `usage-python3-pygame-transform-rotozoom` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-transform-rotozoom` | `logs/libsdl/usage-python3-pygame-transform-rotozoom.log` | `casts/libsdl/usage-python3-pygame-transform-rotozoom.cast` | passed |
| `usage-python3-pygame-transform-scale` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-transform-scale.sh` | `logs/libsdl/usage-python3-pygame-transform-scale.log` | `casts/libsdl/usage-python3-pygame-transform-scale.cast` | testcase command exited with status 1; traceback: ValueError: Source and destination surfaces need the same format. |
| `usage-python3-pygame-transform-scale2x` | `usage` | `failed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-transform-scale2x` | `logs/libsdl/usage-python3-pygame-transform-scale2x.log` | `casts/libsdl/usage-python3-pygame-transform-scale2x.cast` | testcase command exited with status 1; traceback: ValueError: Source and destination surfaces need the same format. |
| `usage-python3-pygame-transform-smoothscale` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-more-client-behavior.sh usage-python3-pygame-transform-smoothscale` | `logs/libsdl/usage-python3-pygame-transform-smoothscale.log` | `casts/libsdl/usage-python3-pygame-transform-smoothscale.cast` | passed |
| `usage-python3-pygame-vector2-dot-product` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-expanded-client-behavior.sh usage-python3-pygame-vector2-dot-product` | `logs/libsdl/usage-python3-pygame-vector2-dot-product.log` | `casts/libsdl/usage-python3-pygame-vector2-dot-product.cast` | passed |
| `usage-python3-pygame-vector2-length` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-even-more-client-behavior.sh usage-python3-pygame-vector2-length` | `logs/libsdl/usage-python3-pygame-vector2-length.log` | `casts/libsdl/usage-python3-pygame-vector2-length.cast` | passed |
| `usage-python3-pygame-vector2-magnitude` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-vector2-magnitude` | `logs/libsdl/usage-python3-pygame-vector2-magnitude.log` | `casts/libsdl/usage-python3-pygame-vector2-magnitude.cast` | passed |
| `usage-python3-pygame-vector2-rotate` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-further-client-behavior.sh usage-python3-pygame-vector2-rotate` | `logs/libsdl/usage-python3-pygame-vector2-rotate.log` | `casts/libsdl/usage-python3-pygame-vector2-rotate.cast` | passed |
| `usage-python3-pygame-vector3-cross` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-tenth-batch-behavior.sh usage-python3-pygame-vector3-cross` | `logs/libsdl/usage-python3-pygame-vector3-cross.log` | `casts/libsdl/usage-python3-pygame-vector3-cross.cast` | passed |
| `usage-python3-pygame-xvfb-display` | `usage` | `passed` | `bash /validator/tests/libsdl/tests/cases/usage/usage-python3-pygame-xvfb-display.sh` | `logs/libsdl/usage-python3-pygame-xvfb-display.log` | `casts/libsdl/usage-python3-pygame-xvfb-display.cast` | passed |
| `version-query-compile` | `source` | `passed` | `bash /validator/tests/libsdl/tests/cases/source/version-query-compile.sh` | `logs/libsdl/version-query-compile.log` | `casts/libsdl/version-query-compile.cast` | passed |

## Preexisting Input Handling

The prepared source snapshots, generated contracts/manifests, CVE data, dependent inventories, performance evidence, dependent regression reports, unsafe audit report, existing integration tests, and upstream test tree were consumed in place. I did not refetch, recollect, rediscover, or regenerate those checked-in artifacts.
