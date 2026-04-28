# Phase 3 Event and Timer Validator Report

Phase ID: `impl_phase_03_event_timer_fixes`

Date: 2026-04-28

## Outcome

- Baseline and phase-02 evidence had event/timer validator failures in pygame usage cases: custom event, event clear, event peek, event queue, key event, mouse event, and timer event all timed out. `headless-event-timer` already passed.
- Root cause: `SDL_PushEvent` invoked the global event filter and event watchers while holding the event queue mutex. Pygame event callbacks can re-enter SDL event APIs, which blocked on the same mutex and produced the validator timeouts.
- Fix: `safe/src/events/queue.rs` now snapshots filters/watchers, invokes callbacks outside the queue mutex, then re-locks only to enqueue and notify waiters. `SDL_FilterEvents` also runs caller callbacks outside the queue mutex while preserving queued event order around concurrent pushes.
- Added `safe/tests/validator_events_timers.rs` with targeted coverage for validator event/timer behavior under `SDL_INIT_VIDEO | SDL_INIT_TIMER | SDL_INIT_EVENTS` and `SDL_VIDEODRIVER=dummy`.
- Event/timer regression tests pass with `--test-threads=1`.
- Existing core and upstream core tests pass with `--test-threads=1`.
- Local safe package build: passed.
- Local override package verification: passed for exactly `libsdl2-2.0-0`, `libsdl2-dev`, and `libsdl2-tests`.
- Full phase-03 validator run completed with validator exit code `1`: `85` cases, `82` passed, `3` failed, `5` source cases passed, `80` usage cases, `85` casts.
- Event/timer outcome: no event/timer validator failures remain in phase-03 evidence.
- Override install verification: all `85` testcase JSON files have `override_debs_installed: true`.
- True validator bug: none identified for event/timer behavior in this phase.

The full validator run is not globally clean because three non-event pygame surface/transform failures remain: `usage-python3-pygame-alpha-blit`, `usage-python3-pygame-transform-scale`, and `usage-python3-pygame-transform-scale2x`. Those failures are outside the event/timer file-change scope for this phase.
The unrelated preexisting `original/src/joystick/__pycache__/` remains untouched and untracked.

## Commands Run

```bash
timeout 20s cargo test --manifest-path safe/Cargo.toml --test validator_events_timers event_filter_and_watch_callbacks_can_query_queue_without_deadlock -- --test-threads=1
```

Before the queue fix, this bounded Rust reproduction timed out with exit code `124`.

```bash
cargo fmt --manifest-path safe/Cargo.toml
```

```bash
cargo test --manifest-path safe/Cargo.toml --test validator_events_timers -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test original_apps_core -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --test upstream_port_core -- --test-threads=1
```

```bash
cargo test --manifest-path safe/Cargo.toml --features host-video-tests --test upstream_port_video_events -- --test-threads=1
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
cd validator
. .work/venv/bin/activate
rm -rf artifacts/.workspace/libsdl-safe-phase03
validator_status=0
bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/.workspace/libsdl-safe-phase03 \
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
| `libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-2.0-0` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `2a8d0e4dbb4fe96d7720aa295b412c2b2a5697a0abc1d60fac784df4eeb2243f` |
| `libsdl2-dev_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-dev` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `9a986f8fb1c8993dae4c469ec0e777de62b394c3024106356844486820555ce5` |
| `libsdl2-tests_2.30.0+dfsg-1ubuntu3.1+safelibs1_amd64.deb` | `libsdl2-tests` | `2.30.0+dfsg-1ubuntu3.1+safelibs1` | `amd64` | `6d9e7172e5c48d7a0f831aacf64b37dc61ef06eb78e13554c9cab5c520e5af66` |

## Event and Timer Results

All event/timer cases passed in `validator/artifacts/.workspace/libsdl-safe-phase03/results/libsdl/`.

| Case ID | Status | Exit Code | Override Debs Installed |
| --- | --- | --- | --- |
| `headless-event-timer` | `passed` | `0` | `true` |
| `usage-python3-pygame-custom-event` | `passed` | `0` | `true` |
| `usage-python3-pygame-event-clear` | `passed` | `0` | `true` |
| `usage-python3-pygame-event-name-keydown` | `passed` | `0` | `true` |
| `usage-python3-pygame-event-peek` | `passed` | `0` | `true` |
| `usage-python3-pygame-event-queue` | `passed` | `0` | `true` |
| `usage-python3-pygame-event-set-blocked` | `passed` | `0` | `true` |
| `usage-python3-pygame-key-event` | `passed` | `0` | `true` |
| `usage-python3-pygame-mouse-event` | `passed` | `0` | `true` |
| `usage-python3-pygame-time-delay` | `passed` | `0` | `true` |
| `usage-python3-pygame-time-wait` | `passed` | `0` | `true` |
| `usage-python3-pygame-timer-event` | `passed` | `0` | `true` |

## Remaining Non-Event Failures

| Case ID | Symptom |
| --- | --- |
| `usage-python3-pygame-alpha-blit` | Exit `139`; the pygame alpha-blit script still segfaults. |
| `usage-python3-pygame-transform-scale` | Exit `1`; traceback reports `ValueError: Source and destination surfaces need the same format.` |
| `usage-python3-pygame-transform-scale2x` | Exit `1`; traceback reports `ValueError: Source and destination surfaces need the same format.` |

## Raw Artifacts

- Results: `validator/artifacts/.workspace/libsdl-safe-phase03/results/libsdl/`
- Logs: `validator/artifacts/.workspace/libsdl-safe-phase03/logs/libsdl/`
- Casts: `validator/artifacts/.workspace/libsdl-safe-phase03/casts/libsdl/`
- Summary JSON: `validator/artifacts/.workspace/libsdl-safe-phase03/results/libsdl/summary.json`

## Preexisting Input Handling

The prepared source snapshots, generated contracts/manifests, CVE data, dependent inventories, performance evidence, dependent regression reports, unsafe audit report, existing integration tests, and upstream test tree were consumed in place. I did not refetch, recollect, rediscover, or regenerate those checked-in artifacts.
