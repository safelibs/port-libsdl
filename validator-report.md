# Final Clean Validator Run and Report

Phase ID: `impl_phase_07_final_validator_report`

Date: 2026-04-28

Validator repository: `https://github.com/safelibs/validator`

Validator commit: `1319bb0374ef66428a42dd71e49553c6d057feaf`

## Final validator run

Pass. The final local safe tests passed, the final unfiltered libsdl validator matrix passed, and no validator-bug skip was used.

Final validator summary from `validator/artifacts/.workspace/libsdl-safe-final/results/libsdl/summary.json`:

```text
mode=original
cases=85
source_cases=5
usage_cases=80
passed=85
failed=0
casts=85
validator_exit_code=0
override_debs_installed=true for 85/85 testcase result JSON files
```

The final exit code is recorded in `validator/artifacts/.workspace/libsdl-safe-final/validator-exit-code.txt`.

## Safe Commit Range

Port baseline tag: `libsdl/04-test` at `14609a9a584435d4a12c46aa11e969a0f0f8a7d5`.

Phase commits already present before this final report commit:

```text
376dfab Record phase 1 validator baseline
2f5ef1e Fix phase 1 report verification markers
5cce380 record phase 2 source packaging result
6a8988f Fix SDL event timer validator failures
3977111 Fix phase 4 surface render validator failures
1b8fa87 Fix phase 4 render and metadata regressions
42c7bff Report phase 5 audio runtime validator results
6b22d73 Document phase 6 validator clean run
167a1e8 Fix X11 stub syswm info
efcc51a Match phase 6 report section headings
```

This phase adds the final report commit on top of that history.

## Final Artifacts

- Local override debs: `validator/artifacts/debs/local/libsdl/`
- Final validator root: `validator/artifacts/.workspace/libsdl-safe-final/`
- Results: `validator/artifacts/.workspace/libsdl-safe-final/results/libsdl/`
- Logs: `validator/artifacts/.workspace/libsdl-safe-final/logs/libsdl/`
- Casts: `validator/artifacts/.workspace/libsdl-safe-final/casts/libsdl/`
- Summary JSON: `validator/artifacts/.workspace/libsdl-safe-final/results/libsdl/summary.json`
- Exit code: `validator/artifacts/.workspace/libsdl-safe-final/validator-exit-code.txt`

## Local Override Debs

| File | Package | Version | Architecture | SHA256 |
| --- | --- | --- | --- | --- |
| `libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-2.0-0` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `84c6b5fc32190c363857dd087094538bb130d81f639bf1ffaabd9166df48f336` |
| `libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-dev` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `d8b9f7362c42bc257ca030fe40226d0968cf13a0191e4511d5cc78c595d6c933` |
| `libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-tests` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `6d9e7172e5c48d7a0f831aacf64b37dc61ef06eb78e13554c9cab5c520e5af66` |

The override directory contains exactly these required runtime, development, and test packages.

## Checks executed

- Final safe Rust workspace test suite with host-video tests: passed.
- Final Debian package rebuild from `safe/`: passed.
- Local override package verification for exactly `libsdl2-2.0-0`, `libsdl2-dev`, and `libsdl2-tests`: passed.
- Validator testcase manifest count check: `libsdl 5 80 85`: passed.
- Final full unfiltered libsdl validator matrix with local override debs and casts: passed.
- Result JSON verification for 85 passed cases, 85 cast files, and `override_debs_installed: true` in every testcase JSON: passed.

No proof or evidence-site generation was run for this local override validator path.

## Commands Run

```bash
cargo test --manifest-path safe/Cargo.toml --workspace --features host-video-tests -- --test-threads=1
```

```bash
cd safe
dpkg-buildpackage -us -uc -b
cd ..
```

```bash
rm -rf validator/artifacts/debs/local/libsdl
```

```bash
mkdir -p validator/artifacts/debs/local/libsdl
```

```bash
cp -v \
  libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb \
  libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb \
  libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb \
  validator/artifacts/debs/local/libsdl/
```

```bash
python3 - <<'PY'
from pathlib import Path
import hashlib
import subprocess

root = Path('validator/artifacts/debs/local/libsdl')
rows = []
for path in sorted(root.glob('*.deb')):
    pkg = subprocess.check_output(['dpkg-deb', '--field', str(path), 'Package'], text=True).strip()
    ver = subprocess.check_output(['dpkg-deb', '--field', str(path), 'Version'], text=True).strip()
    arch = subprocess.check_output(['dpkg-deb', '--field', str(path), 'Architecture'], text=True).strip()
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    rows.append((path.name, pkg, ver, arch, digest))
packages = sorted(row[1] for row in rows)
assert packages == ['libsdl2-2.0-0', 'libsdl2-dev', 'libsdl2-tests'], packages
for row in rows:
    print('\t'.join(row))
PY
```

```bash
cd validator
rm -rf artifacts/.workspace/libsdl-safe-final
PYTHON=.work/venv/bin/python bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/.workspace/libsdl-safe-final \
  --mode original \
  --override-deb-root artifacts/debs/local \
  --library libsdl \
  --record-casts
cd ..
```

```bash
printf '0\n' > validator/artifacts/.workspace/libsdl-safe-final/validator-exit-code.txt
```

```bash
cd validator
.work/venv/bin/python tools/testcases.py \
  --config repositories.yml \
  --tests-root tests \
  --library libsdl \
  --list-summary
cd ..
```

```bash
python3 - <<'PY'
from pathlib import Path
import json

root = Path('validator/artifacts/.workspace/libsdl-safe-final')
results_root = root / 'results' / 'libsdl'
summary = json.loads((results_root / 'summary.json').read_text())
result_files = sorted(p for p in results_root.glob('*.json') if p.name != 'summary.json')
missing_override = []
not_passed = []
for path in result_files:
    data = json.loads(path.read_text())
    if data.get('override_debs_installed') is not True:
        missing_override.append(data.get('testcase_id') or path.stem)
    if data.get('status') != 'passed':
        not_passed.append((data.get('testcase_id') or path.stem, data.get('status'), data.get('exit_code')))
cast_files = sorted((root / 'casts' / 'libsdl').glob('*.cast'))
print(json.dumps(summary, sort_keys=True))
print(f'result_json={len(result_files)}')
print(f'cast_files={len(cast_files)}')
assert summary['cases'] == 85, summary
assert summary['passed'] == 85 and summary['failed'] == 0, summary
assert len(result_files) == 85, len(result_files)
assert len(cast_files) == 85, len(cast_files)
assert not missing_override, missing_override
assert not not_passed, not_passed
PY
```

## Failures found by phase

| Phase | Failures | Outcome |
| --- | --- | --- |
| Phase 1 baseline | Ten usage failures: `usage-python3-pygame-alpha-blit`, `usage-python3-pygame-custom-event`, `usage-python3-pygame-event-clear`, `usage-python3-pygame-event-peek`, `usage-python3-pygame-event-queue`, `usage-python3-pygame-key-event`, `usage-python3-pygame-mouse-event`, `usage-python3-pygame-timer-event`, `usage-python3-pygame-transform-scale`, and `usage-python3-pygame-transform-scale2x`. | Recorded as libsdl-safe compatibility failures, not validator bugs. |
| Phase 2 source/packaging | No source, ABI, header, `sdl2-config`, pkg-config, installed-test, or exported-symbol failures. The same ten usage failures remained. | No source or packaging fix required. |
| Phase 3 event/timer | Pygame event and timer cases timed out because event filters/watchers could re-enter SDL while the queue mutex was held. | Fixed in `safe/src/events/queue.rs`; event/timer validator cases passed. Three surface/transform usage failures remained. |
| Phase 4 surface/render | Alpha-blit crashed due to null ABI-visible blit-map state. Transform scale and scale2x failed due to safe-owned RGB888/BGR888 surface metadata mismatch. Verification bounce also exposed local YUV conversion state, YUV/NV texture update, local x11 stub, Vulkan unavailable-path, and optional GL lookup regressions. | Fixed in surface, pixel, blit, render, display, and window code. Phase 4 validator run became clean. |
| Phase 5 audio/runtime | No audio/runtime validator failures after the phase 4 fixes. | Rebuilt packages and reran the full matrix cleanly. No source change or new validator-specific test was required. |
| Phase 6 remaining and validator-bug triage | Local `xvfb_window_smoke` regression: `SDL_GetWindowWMInfo` returned `0` for an explicitly requested safe `x11` stub window when host SDL was unavailable. No remaining validator failures were present. | Fixed in `safe/src/video/syswm.rs`; full matrix remained clean. No validator-bug skip was required. |
| Phase 7 final | No new failure. | Rebuilt final packages and reran the unfiltered 85-case validator matrix cleanly. |

## Fixes applied

- `safe/src/events/queue.rs`: callbacks for filters/watchers now run outside the queue mutex while preserving queue behavior.
- `safe/src/video/surface.rs`: safe-owned surfaces expose compatible blit-map state and preserve requested 32-bit depth where needed for pygame-derived RGB888/BGR888 surfaces.
- `safe/src/video/pixels.rs`: SDL-compatible RGB888/BGR888 public metadata and mask conversion behavior.
- `safe/src/video/blit.rs`: local RGB conversion path avoids unnecessary host/YUV symbol resolution.
- `safe/src/render/local.rs`: local software-renderer YUV/NV texture update behavior.
- `safe/src/video/display.rs` and `safe/src/video/window.rs`: local stub video behavior for explicitly requested host drivers when host SDL is unavailable.
- `safe/src/video/syswm.rs`: X11 syswm metadata for valid safe-created X11 stub windows.

No source changes were required in phase 7.

## Regression Tests Added

- `safe/tests/validator_events_timers.rs`: event filter/watch re-entry, blocked key/mouse event queue behavior, wait timeout, ticks, delay, and timer-event coverage.
- `safe/tests/validator_surface_render.rs`: alpha blit state, RGB888/BGR888 metadata, mask-derived scaled blit compatibility, local YUV texture updates, and local YUV conversion-mode state.

Existing focused coverage also exercised final fixes:

- `safe/tests/original_apps_render.rs`
- `safe/tests/xvfb_window_smoke.rs`
- `safe/tests/upstream_port_render.rs`
- `safe/tests/upstream_port_surface.rs`
- `safe/tests/original_apps_audio.rs`
- `safe/tests/upstream_port_audio.rs`
- `safe/tests/security_wave_adpcm.rs`

No new phase 7 regression test was needed because the final run did not find a new failure.

## Validator Bug Skips

None.

The final run used the unfiltered validator tests root `validator/tests` and executed all `85` libsdl cases. No `validator/artifacts/.workspace/filtered-tests/` root was used, and no testcase IDs were skipped.

## Validator Source Integrity

No validator source, testcase files, `validator/tools/**`, or `validator/repositories.yml` were modified to pass checks. The validator checkout was not updated in this phase.

## Preexisting Input Handling

The prepared source snapshots, generated ABI/install/dynapi/runtime contracts, CVE data, dependent inventories, original-test manifests, performance thresholds, dependent regression reports, unsafe audit report, existing tests, prior validator artifacts, and upstream test tree were consumed in place. I did not refetch, recollect, rediscover, regenerate, or update these checked-in artifacts.

The unrelated preexisting `original/src/joystick/__pycache__/` remains untouched and untracked.
