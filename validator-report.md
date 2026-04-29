# Phase 6 Remaining Validator Failure Triage Report

Phase ID: `impl_phase_06_remaining_and_validator_bug_triage`

Date: 2026-04-28

Validator commit: `1319bb0374ef66428a42dd71e49553c6d057feaf`

## Outcome

- Consumed the prior phase report and prepared source snapshots, generated contracts, manifests, CVE data, dependent reports, performance evidence, unsafe audit, local safe tree, and prior validator artifacts in place.
- Reproduced the software-tester failure in `safe/tests/xvfb_window_smoke.rs`: `SDL_GetWindowWMInfo` returned `0` for an explicitly requested `x11` safe-stub window when no real host SDL runtime was loaded.
- Fixed `safe/src/video/syswm.rs` so the non-forwarded `x11` stub path validates stub windows, preserves SDL null-info/version error behavior, and returns `SDL_SYSWM_X11` metadata for safe-created X11 stub windows.
- Ran the full safe Rust workspace test suite with host video tests from the current safe source tree. Result: passed.
- Rebuilt the local safe Debian override packages and refreshed `validator/artifacts/debs/local/libsdl/`.
- Reran the full libsdl validator suite into `validator/artifacts/.workspace/libsdl-safe-phase06/`.
- Full phase-06 validator run completed cleanly with validator exit code `0`: `85` cases, `85` passed, `0` failed, `5` source cases, `80` usage cases, `85` casts.
- Override install verification: all `85` testcase JSON files have `override_debs_installed: true`.
- No remaining validator failures were present after the syswm fix, so no additional `safe/tests/validator_*.rs` regression files were required in this phase. The regression is covered by the existing `xvfb_window_smoke` host-video test.
- No validator bug skip was required. No filtered tests root was created.
- The unrelated preexisting `original/src/joystick/__pycache__/` remains untouched and untracked.

## Remaining Failures

None.

Fixed local safe-suite regression:

| Case | Symptom | Fix | Regression Coverage |
| --- | --- | --- | --- |
| `xvfb_window_smoke::xvfb_backed_x11_window_smoke_replaces_manual_window_demos` | `SDL_GetWindowWMInfo` returned `0` for the safe `x11` stub driver | `safe/src/video/syswm.rs` now returns X11 syswm metadata for valid safe-created X11 stub windows | `cargo test --manifest-path safe/Cargo.toml --workspace --features host-video-tests -- --test-threads=1` |

The clean phase-06 full validator run is recorded under:

- Results: `validator/artifacts/.workspace/libsdl-safe-phase06/results/libsdl/`
- Logs: `validator/artifacts/.workspace/libsdl-safe-phase06/logs/libsdl/`
- Casts: `validator/artifacts/.workspace/libsdl-safe-phase06/casts/libsdl/`
- Summary JSON: `validator/artifacts/.workspace/libsdl-safe-phase06/results/libsdl/summary.json`

Validated summary:

```text
cases=85
source_cases=5
usage_cases=80
passed=85
failed=0
casts=85
override_debs_installed=true for 85/85 testcase JSON files
```

## Validator Bugs

None identified.

No validator-bug skip, copied filtered tests root, or filtered rerun was needed.

## Commands Run

```bash
cargo test --manifest-path safe/Cargo.toml \
  --workspace \
  --features host-video-tests \
  xvfb_backed_x11_window_smoke_replaces_manual_window_demos \
  -- \
  --test-threads=1 \
  --nocapture
```

```bash
cargo test --manifest-path safe/Cargo.toml \
  --workspace \
  --features host-video-tests \
  -- \
  --test-threads=1
```

```bash
cd safe
dpkg-buildpackage -us -uc -b
cd ..
```

```bash
rm -rf validator/artifacts/debs/local/libsdl
mkdir -p validator/artifacts/debs/local/libsdl
cp -v \
  libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb \
  libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb \
  libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb \
  validator/artifacts/debs/local/libsdl/
python3 - <<'PY'
from pathlib import Path
import hashlib
import subprocess

root = Path("validator/artifacts/debs/local/libsdl")
rows = []
for path in sorted(root.glob("*.deb")):
    pkg = subprocess.check_output(["dpkg-deb", "--field", str(path), "Package"], text=True).strip()
    ver = subprocess.check_output(["dpkg-deb", "--field", str(path), "Version"], text=True).strip()
    arch = subprocess.check_output(["dpkg-deb", "--field", str(path), "Architecture"], text=True).strip()
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    rows.append((path.name, pkg, ver, arch, digest))
packages = sorted(row[1] for row in rows)
assert packages == ["libsdl2-2.0-0", "libsdl2-dev", "libsdl2-tests"], packages
for row in rows:
    print("\t".join(row))
PY
```

```bash
cd validator
rm -rf artifacts/.workspace/libsdl-safe-phase06
bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/.workspace/libsdl-safe-phase06 \
  --mode original \
  --override-deb-root artifacts/debs/local \
  --library libsdl \
  --record-casts
cd ..
```

```bash
python3 - <<'PY'
from pathlib import Path
import json

root = Path("validator/artifacts/.workspace/libsdl-safe-phase06/results/libsdl")
summary = json.loads((root / "summary.json").read_text())
missing_override = []
not_passed = []
results = []
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
| `libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-2.0-0` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `84c6b5fc32190c363857dd087094538bb130d81f639bf1ffaabd9166df48f336` |
| `libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-dev` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `d8b9f7362c42bc257ca030fe40226d0968cf13a0191e4511d5cc78c595d6c933` |
| `libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-tests` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `6d9e7172e5c48d7a0f831aacf64b37dc61ef06eb78e13554c9cab5c520e5af66` |

## Spot-Checked Validator Cases

| Case ID | Status | Exit Code | Override Debs Installed |
| --- | --- | --- | --- |
| `installed-test-binary` | `passed` | `0` | `true` |
| `headless-event-timer` | `passed` | `0` | `true` |
| `dummy-audio-queue` | `passed` | `0` | `true` |
| `usage-python3-pygame-audio-dummy` | `passed` | `0` | `true` |
| `usage-python3-pygame-rect-inflate-ip` | `passed` | `0` | `true` |

## Preexisting Input Handling

The prepared source snapshots, generated ABI/install/dynapi/runtime contracts, CVE data, dependent inventories, original-test manifests, performance thresholds, dependent regression reports, unsafe audit report, existing tests, prior validator artifacts, and upstream test tree were consumed in place. I did not refetch, recollect, rediscover, regenerate, or update validator checkout content.
