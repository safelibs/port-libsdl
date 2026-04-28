# Phase 5 Audio and Runtime Validator Report

Phase ID: `impl_phase_05_audio_runtime_fixes`

Date: 2026-04-28

## Outcome

- Consumed the baseline and prior phase validator evidence in place. Phase 04 already had a clean libsdl run, and the audio/runtime-relevant cases were passing there.
- Rebuilt the local safe override packages and reran the full libsdl validator suite into `validator/artifacts/.workspace/libsdl-safe-phase05/`.
- Full phase-05 validator run completed cleanly with validator exit code `0`: `85` cases, `85` passed, `0` failed, `5` source cases passed, `80` usage cases, `85` casts.
- Override install verification: all `85` testcase JSON files have `override_debs_installed: true`.
- Audio/runtime outcome: no audio, mixer, queued-audio, dummy-driver, installed-test, init/quit, or runtime validator failures exist in `validator/artifacts/.workspace/libsdl-safe-phase05/results/libsdl/`.
- The focused audio and WAV regression tests passed locally. Existing coverage already exercises dummy audio driver selection, queue playback/capture sizing, pause/unpause status, callback and push paths, audio conversion/streaming, and malformed WAV rejection, so no new `safe/tests/validator_audio_runtime.rs` file was created.
- No audio source, runtime init/quit, installed-test staging, or validator testcase source changes were required.
- True validator bug: none identified for audio/runtime behavior in this phase.
- The unrelated preexisting `original/src/joystick/__pycache__/` remains untouched and untracked.

## Commands Run

```bash
cargo test --manifest-path safe/Cargo.toml --test upstream_port_audio -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test original_apps_audio -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test security_wave_adpcm -- --test-threads=1
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
rm -rf artifacts/.workspace/libsdl-safe-phase05
validator_status=0
bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/.workspace/libsdl-safe-phase05 \
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

root = Path("validator/artifacts/.workspace/libsdl-safe-phase05/results/libsdl")
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
for case in ["dummy-audio-queue", "usage-python3-pygame-audio-dummy", "installed-test-binary", "headless-event-timer"]:
    data = json.loads((root / f"{case}.json").read_text())
    print(f"{case}\t{data.get('status')}\texit={data.get('exit_code')}\toverride={data.get('override_debs_installed')}")
print("summary", summary)
PY
```

## Local Override Debs

Artifact directory: `validator/artifacts/debs/local/libsdl/`

| File | Package | Version | Architecture | SHA256 |
| --- | --- | --- | --- | --- |
| `libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-2.0-0` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `48bda642be7d4bd70cfae450c2db3d3ebfc0dd33e11e2a416de067a884db965b` |
| `libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-dev` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `1c35bf70b2cb508afc6cefebbfdc063b4879643476cfcc5540c22583a3fb47ad` |
| `libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-tests` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `6d9e7172e5c48d7a0f831aacf64b37dc61ef06eb78e13554c9cab5c520e5af66` |

## Audio and Runtime Results

Audio/runtime-relevant validator cases all passed in the phase-05 run.

| Case ID | Status | Exit Code | Override Debs Installed |
| --- | --- | --- | --- |
| `dummy-audio-queue` | `passed` | `0` | `true` |
| `usage-python3-pygame-audio-dummy` | `passed` | `0` | `true` |
| `installed-test-binary` | `passed` | `0` | `true` |
| `headless-event-timer` | `passed` | `0` | `true` |

No other audio, mixer, queued-audio, dummy-driver, installed-test, init/quit, or runtime validator case failed in the phase-05 run.

## Raw Artifacts

- Results: `validator/artifacts/.workspace/libsdl-safe-phase05/results/libsdl/`
- Logs: `validator/artifacts/.workspace/libsdl-safe-phase05/logs/libsdl/`
- Casts: `validator/artifacts/.workspace/libsdl-safe-phase05/casts/libsdl/`
- Summary JSON: `validator/artifacts/.workspace/libsdl-safe-phase05/results/libsdl/summary.json`

## Preexisting Input Handling

The prepared source snapshots, generated contracts/manifests, CVE data, dependent inventories, performance evidence, dependent regression reports, unsafe audit report, existing integration tests, prior validator artifacts, and upstream test tree were consumed in place. I did not refetch, recollect, rediscover, or regenerate those checked-in artifacts.
