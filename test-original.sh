#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_TAG="${LIBSDL_ORIGINAL_TEST_IMAGE:-libsdl-original-test:ubuntu24.04}"
ONLY=""
JSON_OUT=""
ARTIFACT_DIR=""
ARTIFACT_TMP=""
HOST_UID="$(id -u)"
HOST_GID="$(id -g)"

usage() {
  cat <<'EOF'
usage: test-original.sh [--only <slug-or-manifest-name>] [--json-out <path>] [--artifact-dir <dir>]

Builds the safe SDL Debian packages from ./safe using the checked-in
contracts/original inputs inside an Ubuntu 24.04 Docker container, installs
them, and then exercises the dependent software listed in dependents.json.

--only runs a single dependent check. Accepted values include:
  qemu, ffmpeg, scrcpy, love, pygame, scummvm, supertuxkart,
  tuxpaint, openttd, 0ad, imgui, libtcod

--json-out writes a combined structured results file.
--artifact-dir stores per-dependent logs, per-dependent JSON, and the raw combined results JSON.
EOF
}

while (($#)); do
  case "$1" in
    --only)
      ONLY="${2:?missing value for --only}"
      shift 2
      ;;
    --json-out)
      JSON_OUT="${2:?missing value for --json-out}"
      shift 2
      ;;
    --artifact-dir)
      ARTIFACT_DIR="${2:?missing value for --artifact-dir}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      printf 'unknown option: %s\n' "$1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

command -v docker >/dev/null 2>&1 || {
  echo "docker is required to run $0" >&2
  exit 1
}

[[ -d "$ROOT/original" ]] || {
  echo "missing original source tree" >&2
  exit 1
}

[[ -d "$ROOT/safe" ]] || {
  echo "missing safe source tree" >&2
  exit 1
}

[[ -f "$ROOT/dependents.json" ]] || {
  echo "missing dependents.json" >&2
  exit 1
}

resolve_host_path() {
  local value="$1"
  if [[ "$value" = /* ]]; then
    printf '%s\n' "$value"
  else
    printf '%s\n' "$ROOT/$value"
  fi
}

cleanup_host_artifacts() {
  if [[ -n "$ARTIFACT_TMP" && -d "$ARTIFACT_TMP" ]]; then
    rm -rf "$ARTIFACT_TMP"
  fi
}

trap cleanup_host_artifacts EXIT

if [[ -n "$ARTIFACT_DIR" ]]; then
  ARTIFACT_DIR="$(resolve_host_path "$ARTIFACT_DIR")"
  mkdir -p "$ARTIFACT_DIR"
else
  ARTIFACT_TMP="$(mktemp -d)"
  ARTIFACT_DIR="$ARTIFACT_TMP"
fi

if [[ -n "$JSON_OUT" ]]; then
  JSON_OUT="$(resolve_host_path "$JSON_OUT")"
  mkdir -p "$(dirname "$JSON_OUT")"
fi

docker build -t "$IMAGE_TAG" - <<'DOCKERFILE'
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN sed 's/^Types: deb$/Types: deb-src/' /etc/apt/sources.list.d/ubuntu.sources \
      > /etc/apt/sources.list.d/ubuntu-src.sources \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
      autoconf \
      automake \
      build-essential \
      ca-certificates \
      curl \
      dbus-x11 \
      dpkg-dev \
      file \
      gzip \
      jq \
      make \
      netcat-openbsd \
      pkg-config \
      python3 \
      rsync \
      x11-utils \
      xauth \
      xvfb \
      xdotool \
 && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | bash -s -- -y --profile minimal --default-toolchain stable \
 && rm -rf /var/lib/apt/lists/*

ENV PATH=/root/.cargo/bin:${PATH}
DOCKERFILE

set +e
docker run --rm -i \
  -e "LIBSDL_TEST_ONLY=$ONLY" \
  -e "LIBSDL_ARTIFACT_HOST_DIR=$ARTIFACT_DIR" \
  -e "LIBSDL_ARTIFACT_UID=$HOST_UID" \
  -e "LIBSDL_ARTIFACT_GID=$HOST_GID" \
  -v "$ROOT":/work:ro \
  -v "$ARTIFACT_DIR":/artifacts \
  "$IMAGE_TAG" \
  bash -s <<'CONTAINER_SCRIPT'
set -euo pipefail

export LANG=C.UTF-8
export LC_ALL=C.UTF-8

ROOT=/work
ONLY_FILTER="${LIBSDL_TEST_ONLY:-}"
ARTIFACT_DIR=/artifacts
ARTIFACT_HOST_DIR="${LIBSDL_ARTIFACT_HOST_DIR:-/artifacts}"
ARTIFACT_UID="${LIBSDL_ARTIFACT_UID:-}"
ARTIFACT_GID="${LIBSDL_ARTIFACT_GID:-}"
ROOT_HOME=/tmp/libsdl-root-home
TEST_USER_HOME=/tmp/libsdl-test-home
HOME="$ROOT_HOME"
RUSTUP_HOME=/root/.rustup
CARGO_HOME=/root/.cargo
PATH="/root/.cargo/bin:${PATH}"
MULTIARCH="$(gcc -print-multiarch)"
SAFE_REPO=/tmp/libsdl-safe-repo
SAFE_SDL_SO=""
SAFE_SDL_LIBDIR=""
SAFE_SDL_PKGCONFIG_DIR=""
XVFB_PID=""
MATCHED_ONLY=0
FAILED_CASES=0
TEST_USER=libsdltest
TEST_USER_RUNTIME_DIR="/tmp/${TEST_USER}-runtime"
RESULTS_TSV="$ARTIFACT_DIR/results.tsv"

export HOME RUSTUP_HOME CARGO_HOME PATH

mkdir -p "$ROOT_HOME" "$TEST_USER_HOME" "$ARTIFACT_DIR"
: >"$RESULTS_TSV"

log_step() {
  printf '\n==> %s\n' "$1"
}

die() {
  echo "error: $*" >&2
  exit 1
}

require_contains() {
  local path="$1"
  local needle="$2"

  if ! grep -F -- "$needle" "$path" >/dev/null 2>&1; then
    printf 'missing expected text in %s: %s\n' "$path" "$needle" >&2
    printf -- '--- %s ---\n' "$path" >&2
    cat "$path" >&2
    exit 1
  fi
}

validate_dependents_inventory() {
  python3 <<'PY'
import json
from pathlib import Path

expected = [
    {"slug": "qemu", "name": "QEMU system GUI modules"},
    {"slug": "ffmpeg", "name": "FFmpeg"},
    {"slug": "scrcpy", "name": "scrcpy"},
    {"slug": "love", "name": "LOVE"},
    {"slug": "pygame", "name": "pygame"},
    {"slug": "scummvm", "name": "ScummVM"},
    {"slug": "supertuxkart", "name": "SuperTuxKart"},
    {"slug": "tuxpaint", "name": "Tux Paint"},
    {"slug": "openttd", "name": "OpenTTD"},
    {"slug": "0ad", "name": "0 A.D."},
    {"slug": "imgui", "name": "Dear ImGui development package"},
    {"slug": "libtcod", "name": "libtcod development package"},
]

data = json.loads(Path("/work/dependents.json").read_text(encoding="utf-8"))
actual = [{"slug": entry["slug"], "name": entry["name"]} for entry in data["dependents"]]
if actual != expected:
    raise SystemExit(
        f"unexpected dependents.json contents: expected {expected}, found {actual}"
    )
PY
}

collect_case_artifacts() {
  local slug="$1"
  local case_dir="$2"
  local path

  mkdir -p "$case_dir"
  for path in "/tmp/${slug}.log" "/tmp/${slug}-maps.log" "/tmp/${slug}-windows.log" /tmp/xvfb.log; do
    if [[ -f "$path" ]]; then
      cp "$path" "$case_dir/" || true
    fi
  done

  if [[ "$slug" == "0ad" && -d "$TEST_USER_HOME/.config/0ad/logs" ]]; then
    mkdir -p "$case_dir/0ad-logs"
    cp -a "$TEST_USER_HOME/.config/0ad/logs/." "$case_dir/0ad-logs/" || true
  fi
}

record_case_result() {
  local slug="$1"
  local manifest_name="$2"
  local status="$3"
  local duration="$4"
  local case_dir="$5"
  local note="$6"
  local host_case_dir="${ARTIFACT_HOST_DIR}/${slug}"
  local host_log_path="${host_case_dir}/console.log"
  local host_json_path="${ARTIFACT_HOST_DIR}/${slug}.json"

  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$slug" \
    "$manifest_name" \
    "$status" \
    "$duration" \
    "$host_case_dir" \
    "$host_log_path" \
    "$host_json_path" \
    "$note" \
    >>"$RESULTS_TSV"
}

write_results_json() {
  python3 <<'PY'
import json
import os
from pathlib import Path

dependents = json.loads(Path("/work/dependents.json").read_text(encoding="utf-8"))["dependents"]
results_tsv = Path("/artifacts/results.tsv")
entries = []
if results_tsv.exists():
    for line in results_tsv.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        slug, name, status, duration, artifact_dir, log_path, json_path, note = line.split("\t")
        entry = {
            "slug": slug,
            "name": name,
            "status": status,
            "duration_seconds": float(duration),
            "artifact_dir": artifact_dir,
            "log_path": log_path,
            "json_path": json_path,
            "notes": [note] if note else [],
        }
        entries.append(entry)

ordered = []
entry_by_slug = {entry["slug"]: entry for entry in entries}
for dependent in dependents:
    entry = entry_by_slug.get(dependent["slug"])
    if entry is not None:
        ordered.append(entry)

summary = {
    "total": len(ordered),
    "passed": sum(1 for entry in ordered if entry["status"] == "passed"),
    "failed": sum(1 for entry in ordered if entry["status"] == "failed"),
}
payload = {
    "schema_version": 1,
    "phase_id": "impl_phase_10_packaging_dependents_final",
    "only_filter": os.environ.get("LIBSDL_TEST_ONLY") or None,
    "dependents": ordered,
    "summary": summary,
}
results_json = Path("/artifacts/results.json")
results_json.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
for entry in ordered:
    per_slug = {
        "schema_version": 1,
        "phase_id": payload["phase_id"],
        "only_filter": entry["slug"],
        "dependents": [entry],
        "summary": {
            "total": 1,
            "passed": 1 if entry["status"] == "passed" else 0,
            "failed": 1 if entry["status"] == "failed" else 0,
        },
    }
    Path("/artifacts", f"{entry['slug']}.json").write_text(
        json.dumps(per_slug, indent=2) + "\n",
        encoding="utf-8",
    )
PY
}

apt_install() {
  apt-get install -y --no-install-recommends "$@"
}

setup_test_user() {
  if ! id -u "$TEST_USER" >/dev/null 2>&1; then
    useradd --home-dir "$TEST_USER_HOME" --create-home --shell /bin/bash "$TEST_USER"
  fi

  mkdir -p "$TEST_USER_HOME" "$TEST_USER_RUNTIME_DIR"
  chown -R "$TEST_USER:$TEST_USER" "$TEST_USER_HOME" "$TEST_USER_RUNTIME_DIR"
  chmod 700 "$TEST_USER_RUNTIME_DIR"
}

selection_matches() {
  local slug="$1"
  local manifest_name="$2"

  [[ -z "$ONLY_FILTER" || "$ONLY_FILTER" == "$slug" || "$ONLY_FILTER" == "$manifest_name" ]]
}

install_runtime_packages() {
  log_step "Installing SDL build dependencies and dependent packages"
  apt-get update
  apt-get build-dep -y "$SAFE_REPO/original"
  apt-get build-dep -y "$SAFE_REPO/safe"

  local packages=()
  selection_matches ffmpeg "FFmpeg" && packages+=(ffmpeg)
  selection_matches imgui "Dear ImGui development package" && packages+=(libimgui-dev)
  selection_matches libtcod "libtcod development package" && packages+=(libtcod-dev)
  if selection_matches openttd "OpenTTD"; then
    packages+=(openttd openttd-opengfx openttd-openmsx openttd-opensfx)
  fi
  if selection_matches qemu "QEMU system GUI modules"; then
    packages+=(qemu-system-gui qemu-system-x86)
  fi
  selection_matches scummvm "ScummVM" && packages+=(scummvm)
  selection_matches scrcpy "scrcpy" && packages+=(scrcpy)
  selection_matches supertuxkart "SuperTuxKart" && packages+=(supertuxkart)
  selection_matches tuxpaint "Tux Paint" && packages+=(tuxpaint)
  selection_matches 0ad "0 A.D." && packages+=(0ad)

  if ((${#packages[@]})); then
    apt_install "${packages[@]}"
  fi

  # Ubuntu 24.04 ships love 11.5-1build1 with a broken postinst that expects
  # a versioned manpage path which is not present in the package contents.
  if selection_matches love "LOVE"; then
    mkdir -p /usr/share/man/man6
    if [[ ! -f /usr/share/man/man6/love-11.5.6.gz ]]; then
      printf '.TH love 6 "" "" ""\n.SH NAME\nlove\n' | gzip -9n >/usr/share/man/man6/love-11.5.6.gz
    fi
    apt_install love
  fi
}

prepare_safe_source_tree() {
  rm -rf "$SAFE_REPO"
  mkdir -p "$SAFE_REPO"
  rsync -a --delete "$ROOT/safe/" "$SAFE_REPO/safe/"
  rsync -a --delete "$ROOT/original/" "$SAFE_REPO/original/"
  rsync -a "$ROOT/dependents.json" "$SAFE_REPO/"
  rsync -a "$ROOT/relevant_cves.json" "$SAFE_REPO/"
}

build_safe_sdl() {
  log_step "Building and installing safe SDL Debian packages"

  rm -f "$SAFE_REPO/safe/Cargo.lock"
  (
    cd "$SAFE_REPO/safe"
    cargo generate-lockfile --manifest-path Cargo.toml >/tmp/libsdl-safe-lock.log 2>&1
  ) || {
    cat /tmp/libsdl-safe-lock.log >&2 || true
    die "failed to generate Docker-local Cargo.lock for safe SDL"
  }

  (
    cd "$SAFE_REPO/safe"
    dpkg-buildpackage -us -uc -b >/tmp/libsdl-safe-build.log 2>&1
  ) || {
    cat /tmp/libsdl-safe-build.log >&2 || true
    die "failed to build safe SDL Debian packages"
  }

  local runtime_deb dev_deb tests_deb installed_pc pkg_prefix
  runtime_deb="$(find "$SAFE_REPO" -maxdepth 1 -type f -name 'libsdl2-2.0-0_*_*.deb' | sort | tail -n1)"
  dev_deb="$(find "$SAFE_REPO" -maxdepth 1 -type f -name 'libsdl2-dev_*_*.deb' | sort | tail -n1)"
  tests_deb="$(find "$SAFE_REPO" -maxdepth 1 -type f -name 'libsdl2-tests_*_*.deb' | sort | tail -n1)"

  [[ -n "$runtime_deb" ]] || die "failed to locate built libsdl2-2.0-0 package"
  [[ -n "$dev_deb" ]] || die "failed to locate built libsdl2-dev package"
  [[ -n "$tests_deb" ]] || die "failed to locate built libsdl2-tests package"

  apt-get install -y --no-install-recommends \
    "$runtime_deb" \
    "$dev_deb" \
    "$tests_deb" \
    >/tmp/libsdl-safe-install.log 2>&1 || {
      cat /tmp/libsdl-safe-install.log >&2 || true
      die "failed to install safe SDL Debian packages"
    }

  ldconfig

  SAFE_SDL_SO="$(readlink -f "/usr/lib/${MULTIARCH}/libSDL2-2.0.so.0")"
  SAFE_SDL_LIBDIR="/usr/lib/${MULTIARCH}"
  installed_pc="$(find "/usr/lib/${MULTIARCH}" -type f -path '*/pkgconfig/sdl2.pc' | sort | head -n1)"
  SAFE_SDL_PKGCONFIG_DIR="$(dirname "$installed_pc")"

  [[ -n "$SAFE_SDL_SO" && -f "$SAFE_SDL_SO" ]] || die "failed to locate installed safe libSDL2-2.0.so.0"
  [[ -n "$SAFE_SDL_PKGCONFIG_DIR" && -d "$SAFE_SDL_PKGCONFIG_DIR" ]] || die "failed to locate installed safe sdl2.pc"

  export LD_LIBRARY_PATH="$SAFE_SDL_LIBDIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  export PKG_CONFIG_PATH="$SAFE_SDL_PKGCONFIG_DIR${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

  pkg_prefix="$(pkg-config --variable=prefix sdl2)"
  pkg_prefix="$(readlink -f "$pkg_prefix")"
  [[ "$pkg_prefix" == "/usr" ]] || die "expected installed sdl2.pc prefix to resolve to /usr, found $pkg_prefix"
}

cleanup_xvfb() {
  if [[ -n "$XVFB_PID" ]] && kill -0 "$XVFB_PID" >/dev/null 2>&1; then
    kill "$XVFB_PID" >/dev/null 2>&1 || true
    wait "$XVFB_PID" >/dev/null 2>&1 || true
  fi
}

finalize_artifacts() {
  chmod -R u+rwX "$ARTIFACT_DIR" >/dev/null 2>&1 || true
  if [[ -n "$ARTIFACT_UID" && -n "$ARTIFACT_GID" ]]; then
    chown -R "$ARTIFACT_UID:$ARTIFACT_GID" "$ARTIFACT_DIR" >/dev/null 2>&1 || true
  fi
}

cleanup_container() {
  cleanup_xvfb
  finalize_artifacts
}

trap cleanup_container EXIT

start_xvfb() {
  if [[ -n "$XVFB_PID" ]] && kill -0 "$XVFB_PID" >/dev/null 2>&1; then
    return 0
  fi

  export DISPLAY=:99
  export LIBGL_ALWAYS_SOFTWARE=1
  Xvfb "$DISPLAY" -screen 0 1280x1024x24 +extension GLX +render -noreset >/tmp/xvfb.log 2>&1 &
  XVFB_PID=$!

  for _ in $(seq 1 40); do
    if xdpyinfo >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done

  cat /tmp/xvfb.log >&2 || true
  die "Xvfb failed to start"
}

run_as_test_user() {
  runuser -u "$TEST_USER" -- env -i \
    HOME="$TEST_USER_HOME" \
    USER="$TEST_USER" \
    LOGNAME="$TEST_USER" \
    SHELL=/bin/bash \
    LANG="$LANG" \
    LC_ALL="$LC_ALL" \
    PATH="$PATH" \
    DISPLAY="${DISPLAY:-}" \
    LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-}" \
    LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}" \
    PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}" \
    SDL_AUDIODRIVER="${SDL_AUDIODRIVER:-}" \
    XDG_RUNTIME_DIR="$TEST_USER_RUNTIME_DIR" \
    "$@"
}

assert_uses_safe_sdl() {
  local target="$1"
  local resolved

  resolved="$(ldd "$target" 2>/dev/null | awk '$1 == "libSDL2-2.0.so.0" { print $3; exit }')"
  [[ -n "$resolved" ]] || die "ldd did not report libSDL2-2.0.so.0 for $target"
  resolved="$(readlink -f "$resolved")"
  [[ "$resolved" == "$SAFE_SDL_SO" ]] || {
    printf 'expected %s to resolve libSDL2-2.0.so.0 from %s, got %s\n' "$target" "$SAFE_SDL_SO" "$resolved" >&2
    ldd "$target" >&2
    exit 1
  }
}

first_installed_path() {
  local package="$1"
  local regex="$2"
  local path

  while IFS= read -r path; do
    if [[ -e "$path" ]]; then
      printf '%s\n' "$path"
      return 0
    fi
  done < <(dpkg -L "$package" | grep -E "$regex")

  return 1
}

first_installed_elf() {
  local package="$1"
  local regex="$2"
  local path

  while IFS= read -r path; do
    [[ -f "$path" ]] || continue
    if ! file -b "$path" | grep -q '^ELF'; then
      continue
    fi
    printf '%s\n' "$path"
    return 0
  done < <(dpkg -L "$package" | grep -E "$regex")

  return 1
}

terminate_pid() {
  local pid="$1"

  kill -TERM "$pid" >/dev/null 2>&1 || true
  for _ in $(seq 1 40); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" >/dev/null 2>&1 || true
      return 0
    fi
    sleep 0.25
  done

  kill -KILL "$pid" >/dev/null 2>&1 || true
  wait "$pid" >/dev/null 2>&1 || true
}

run_window_smoke() {
  local slug="$1"
  local window_pattern="$2"
  local logfile="/tmp/${slug}.log"
  shift 2

  : >"$logfile"
  "$@" >"$logfile" 2>&1 &
  local pid=$!
  local found=0
  local baseline_window_count=0

  if [[ "$window_pattern" == "*" ]]; then
    baseline_window_count="$(xwininfo -root -tree 2>/dev/null | awk '/^[[:space:]]+0x[0-9a-f]+ / {count++} END {print count + 0}')"
  fi

  for _ in $(seq 1 480); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      cat "$logfile" >&2 || true
      die "$slug exited before creating a window"
    fi

    if [[ "$window_pattern" == "*" ]]; then
      local current_window_count
      current_window_count="$(xwininfo -root -tree 2>/dev/null | awk '/^[[:space:]]+0x[0-9a-f]+ / {count++} END {print count + 0}')"
      if (( current_window_count > baseline_window_count )); then
        found=1
        break
      fi
    else
      if xdotool search --onlyvisible --name "$window_pattern" >/tmp/${slug}-windows.log 2>/dev/null \
        || xwininfo -root -tree 2>/dev/null | grep -E "\"${window_pattern}\"" >/tmp/${slug}-windows.log
      then
        found=1
        break
      fi
    fi
    sleep 0.25
  done

  if [[ "$found" != "1" ]]; then
    xwininfo -root -tree >&2 || true
    cat "$logfile" >&2 || true
    terminate_pid "$pid"
    die "timed out waiting for window pattern '$window_pattern' in $slug"
  fi

  terminate_pid "$pid"
}

run_safe_sdl_runtime_smoke() {
  local slug="$1"
  local logfile="/tmp/${slug}.log"
  local maps_log="/tmp/${slug}-maps.log"
  shift

  : >"$logfile"
  : >"$maps_log"
  "$@" >"$logfile" 2>&1 &
  local pid=$!

  for _ in $(seq 1 160); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" >/dev/null 2>&1 || true
      printf -- '--- %s log ---\n' "$slug" >&2
      cat "$logfile" >&2 || true
      die "$slug exited before it loaded the safe SDL runtime"
    fi

    if grep -F -- "$SAFE_SDL_SO" "/proc/$pid/maps" >"$maps_log" 2>/dev/null; then
      terminate_pid "$pid"
      return 0
    fi

    sleep 0.25
  done

  printf -- '--- %s maps ---\n' "$slug" >&2
  cat "$maps_log" >&2 || true
  printf -- '--- %s log ---\n' "$slug" >&2
  cat "$logfile" >&2 || true
  terminate_pid "$pid"
  die "timed out waiting for $slug to load the safe SDL runtime"
}

test_qemu() {
  local ui_module logfile pid
  ui_module="$(first_installed_path qemu-system-gui '/ui-sdl\.so$')"
  [[ -n "$ui_module" ]] || die "failed to locate qemu SDL UI module"
  assert_uses_safe_sdl "$ui_module"

  start_xvfb
  logfile=/tmp/qemu.log
  : >"$logfile"
  qemu-system-x86_64 \
      -display sdl,gl=off \
      -accel tcg \
      -m 64 \
      -serial none \
      -monitor none \
      >"$logfile" 2>&1 &
  pid=$!

  for _ in $(seq 1 160); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      cat "$logfile" >&2 || true
      die "qemu exited before loading the SDL UI module"
    fi

    if grep -F -- "$ui_module" "/proc/$pid/maps" >/tmp/qemu-maps.log 2>/dev/null \
      && grep -F -- "$SAFE_SDL_SO" "/proc/$pid/maps" >>/tmp/qemu-maps.log 2>/dev/null
    then
      terminate_pid "$pid"
      return 0
    fi

    sleep 0.25
  done

  printf -- '--- qemu maps ---\n' >&2
  cat /tmp/qemu-maps.log >&2 || true
  printf -- '--- qemu log ---\n' >&2
  cat "$logfile" >&2 || true
  terminate_pid "$pid"
  die "timed out waiting for qemu to load ui-sdl.so and the safe SDL runtime"
}

test_ffmpeg() {
  assert_uses_safe_sdl "$(command -v ffplay)"

  local logfile=/tmp/ffmpeg.log
  local maps_log=/tmp/ffmpeg-maps.log
  : >"$logfile"
  : >"$maps_log"

  env SDL_AUDIODRIVER=dummy SDL_VIDEODRIVER=dummy \
    ffplay -v error -nodisp \
      -f lavfi -i 'sine=frequency=1000:sample_rate=48000' \
      >"$logfile" 2>&1 &
  local pid=$!

  for _ in $(seq 1 40); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" >/dev/null 2>&1 || true
      printf -- '--- ffplay log ---\n' >&2
      cat "$logfile" >&2 || true
      die "ffplay exited before it loaded the safe SDL runtime"
    fi

    if grep -F -- "$SAFE_SDL_SO" "/proc/$pid/maps" >"$maps_log" 2>/dev/null; then
      terminate_pid "$pid"
      return 0
    fi

    sleep 0.25
  done

  printf -- '--- ffplay maps ---\n' >&2
  cat "$maps_log" >&2 || true
  printf -- '--- ffplay log ---\n' >&2
  cat "$logfile" >&2 || true
  terminate_pid "$pid"
  die "timed out waiting for ffplay to load the safe SDL runtime"
}

test_scrcpy() {
  local scrcpy_elf
  scrcpy_elf="$(first_installed_elf scrcpy '/scrcpy$')"
  [[ -n "$scrcpy_elf" ]] || die "failed to locate scrcpy ELF binary"
  assert_uses_safe_sdl "$scrcpy_elf"

  # The packaged scrcpy binary cannot reach its SDL viewer path without an
  # attached Android device, so build a narrow smoke probe around scrcpy's
  # actual SDL OTG frontend implementation instead.
  rm -rf /tmp/scrcpy-source /tmp/scrcpy-probe
  mkdir -p /tmp/scrcpy-source /tmp/scrcpy-probe/util
  if ! (
    cd /tmp/scrcpy-source
    apt-get source scrcpy >/tmp/scrcpy-source.log 2>&1
  ); then
    cat /tmp/scrcpy-source.log >&2 || true
    die "failed to fetch scrcpy source package"
  fi

  local scrcpy_src
  scrcpy_src="$(find /tmp/scrcpy-source -mindepth 1 -maxdepth 1 -type d -name 'scrcpy-[0-9]*' | head -n1)"
  [[ -n "$scrcpy_src" ]] || die "failed to locate scrcpy source tree"

  cp "$scrcpy_src/app/src/usb/screen_otg.c" /tmp/scrcpy-probe/screen_otg.c
  cp "$scrcpy_src/app/src/usb/screen_otg.h" /tmp/scrcpy-probe/screen_otg.h

  cat >/tmp/scrcpy-probe/common.h <<'EOF'
#ifndef SC_COMMON_H
#define SC_COMMON_H

#include <assert.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define ARRAY_LEN(a) (sizeof(a) / sizeof((a)[0]))
#define MIN(X,Y) ((X) < (Y) ? (X) : (Y))
#define MAX(X,Y) ((X) > (Y) ? (X) : (Y))
#define CLAMP(V,X,Y) MIN(MAX((V), (X)), (Y))
#define container_of(ptr, type, member) \
    ((type *) (((char *) (ptr)) - offsetof(type, member)))

#endif
EOF

  cat >/tmp/scrcpy-probe/options.h <<'EOF'
#ifndef SCRCPY_OPTIONS_H
#define SCRCPY_OPTIONS_H

enum sc_log_level {
    SC_LOG_LEVEL_VERBOSE,
    SC_LOG_LEVEL_DEBUG,
    SC_LOG_LEVEL_INFO,
    SC_LOG_LEVEL_WARN,
    SC_LOG_LEVEL_ERROR,
};

#define SC_WINDOW_POSITION_UNDEFINED (-0x8000)

#endif
EOF

  cat >/tmp/scrcpy-probe/input_events.h <<'EOF'
#ifndef SC_INPUT_EVENTS_H
#define SC_INPUT_EVENTS_H

#include "common.h"

#include <SDL2/SDL_events.h>

enum sc_action {
    SC_ACTION_DOWN,
    SC_ACTION_UP,
};

enum sc_keycode {
    SC_KEYCODE_UNKNOWN = SDLK_UNKNOWN,
};

enum sc_scancode {
    SC_SCANCODE_UNKNOWN = SDL_SCANCODE_UNKNOWN,
};

enum sc_mouse_button {
    SC_MOUSE_BUTTON_UNKNOWN = 0,
    SC_MOUSE_BUTTON_LEFT = SDL_BUTTON(SDL_BUTTON_LEFT),
    SC_MOUSE_BUTTON_RIGHT = SDL_BUTTON(SDL_BUTTON_RIGHT),
    SC_MOUSE_BUTTON_MIDDLE = SDL_BUTTON(SDL_BUTTON_MIDDLE),
    SC_MOUSE_BUTTON_X1 = SDL_BUTTON(SDL_BUTTON_X1),
    SC_MOUSE_BUTTON_X2 = SDL_BUTTON(SDL_BUTTON_X2),
};

struct sc_position {
    int unused;
};

struct sc_key_event {
    enum sc_action action;
    enum sc_keycode keycode;
    enum sc_scancode scancode;
    uint16_t mods_state;
    bool repeat;
};

struct sc_text_event {
    const char *text;
};

struct sc_mouse_click_event {
    struct sc_position position;
    enum sc_action action;
    enum sc_mouse_button button;
    uint64_t pointer_id;
    uint8_t buttons_state;
};

struct sc_mouse_scroll_event {
    struct sc_position position;
    float hscroll;
    float vscroll;
    uint8_t buttons_state;
};

struct sc_mouse_motion_event {
    struct sc_position position;
    uint64_t pointer_id;
    int32_t xrel;
    int32_t yrel;
    uint8_t buttons_state;
};

struct sc_touch_event {
    struct sc_position position;
    int unused;
};

#define SC_SEQUENCE_INVALID 0

static inline uint16_t
sc_mods_state_from_sdl(uint16_t mods_state) {
    return mods_state;
}

static inline enum sc_keycode
sc_keycode_from_sdl(SDL_Keycode keycode) {
    (void) keycode;
    return SC_KEYCODE_UNKNOWN;
}

static inline enum sc_scancode
sc_scancode_from_sdl(SDL_Scancode scancode) {
    (void) scancode;
    return SC_SCANCODE_UNKNOWN;
}

static inline enum sc_action
sc_action_from_sdl_keyboard_type(uint32_t type) {
    return type == SDL_KEYDOWN ? SC_ACTION_DOWN : SC_ACTION_UP;
}

static inline enum sc_action
sc_action_from_sdl_mousebutton_type(uint32_t type) {
    return type == SDL_MOUSEBUTTONDOWN ? SC_ACTION_DOWN : SC_ACTION_UP;
}

static inline enum sc_mouse_button
sc_mouse_button_from_sdl(uint8_t button) {
    if (button >= SDL_BUTTON_LEFT && button <= SDL_BUTTON_X2) {
        return SDL_BUTTON(button);
    }
    return SC_MOUSE_BUTTON_UNKNOWN;
}

static inline uint8_t
sc_mouse_buttons_state_from_sdl(uint32_t buttons_state,
                                bool forward_all_clicks) {
    uint8_t mask = SC_MOUSE_BUTTON_LEFT;
    if (forward_all_clicks) {
        mask |= SC_MOUSE_BUTTON_RIGHT
              | SC_MOUSE_BUTTON_MIDDLE
              | SC_MOUSE_BUTTON_X1
              | SC_MOUSE_BUTTON_X2;
    }
    return buttons_state & mask;
}

#endif
EOF

  cat >/tmp/scrcpy-probe/hid_keyboard.h <<'EOF'
#ifndef SC_HID_KEYBOARD_H
#define SC_HID_KEYBOARD_H

#include "common.h"
#include "input_events.h"

struct sc_key_processor;

struct sc_key_processor_ops {
    void (*process_key)(struct sc_key_processor *kp,
                        const struct sc_key_event *event,
                        uint64_t ack_to_wait);
    void (*process_text)(struct sc_key_processor *kp,
                         const struct sc_text_event *event);
};

struct sc_key_processor {
    bool async_paste;
    const struct sc_key_processor_ops *ops;
};

struct sc_hid_keyboard {
    struct sc_key_processor key_processor;
};

#endif
EOF

  cat >/tmp/scrcpy-probe/hid_mouse.h <<'EOF'
#ifndef SC_HID_MOUSE_H
#define SC_HID_MOUSE_H

#include "common.h"
#include "input_events.h"

struct sc_mouse_processor;

struct sc_mouse_processor_ops {
    void (*process_mouse_motion)(struct sc_mouse_processor *mp,
                                 const struct sc_mouse_motion_event *event);
    void (*process_mouse_click)(struct sc_mouse_processor *mp,
                                const struct sc_mouse_click_event *event);
    void (*process_mouse_scroll)(struct sc_mouse_processor *mp,
                                 const struct sc_mouse_scroll_event *event);
    void (*process_touch)(struct sc_mouse_processor *mp,
                          const struct sc_touch_event *event);
};

struct sc_mouse_processor {
    const struct sc_mouse_processor_ops *ops;
    bool relative_mode;
};

struct sc_hid_mouse {
    struct sc_mouse_processor mouse_processor;
};

#endif
EOF

  cat >/tmp/scrcpy-probe/icon.h <<'EOF'
#ifndef SC_ICON_H
#define SC_ICON_H

#include <SDL2/SDL.h>

SDL_Surface *scrcpy_icon_load(void);
void scrcpy_icon_destroy(SDL_Surface *icon);

#endif
EOF

  cat >/tmp/scrcpy-probe/util/log.h <<'EOF'
#ifndef SC_LOG_H
#define SC_LOG_H

#include <SDL2/SDL_log.h>

#define LOGV(...) SDL_LogVerbose(SDL_LOG_CATEGORY_APPLICATION, __VA_ARGS__)
#define LOGD(...) SDL_LogDebug(SDL_LOG_CATEGORY_APPLICATION, __VA_ARGS__)
#define LOGI(...) SDL_LogInfo(SDL_LOG_CATEGORY_APPLICATION, __VA_ARGS__)
#define LOGW(...) SDL_LogWarn(SDL_LOG_CATEGORY_APPLICATION, __VA_ARGS__)
#define LOGE(...) SDL_LogError(SDL_LOG_CATEGORY_APPLICATION, __VA_ARGS__)
#define LOG_OOM() LOGE("OOM: %s:%d %s()", __FILE__, __LINE__, __func__)

#endif
EOF

  cat >/tmp/scrcpy-probe/icon.c <<'EOF'
#include "icon.h"

SDL_Surface *
scrcpy_icon_load(void) {
    SDL_Surface *surface =
        SDL_CreateRGBSurfaceWithFormat(0, 16, 16, 32, SDL_PIXELFORMAT_RGBA32);
    if (!surface) {
        return NULL;
    }

    SDL_FillRect(surface, NULL,
                 SDL_MapRGBA(surface->format, 0x2d, 0x96, 0xf0, 0xff));
    return surface;
}

void
scrcpy_icon_destroy(SDL_Surface *icon) {
    SDL_FreeSurface(icon);
}
EOF

  cat >/tmp/scrcpy-probe/main.c <<'EOF'
#include "screen_otg.h"
#include "options.h"

#include <SDL2/SDL.h>

int
main(void) {
    if (SDL_Init(SDL_INIT_VIDEO) != 0) {
        SDL_Log("SDL_Init failed: %s", SDL_GetError());
        return 1;
    }

    struct sc_screen_otg screen;
    struct sc_screen_otg_params params = {
        .keyboard = NULL,
        .mouse = NULL,
        .window_title = "scrcpy SDL frontend smoke",
        .always_on_top = false,
        .window_x = SC_WINDOW_POSITION_UNDEFINED,
        .window_y = SC_WINDOW_POSITION_UNDEFINED,
        .window_width = 320,
        .window_height = 240,
        .window_borderless = false,
    };

    if (!sc_screen_otg_init(&screen, &params)) {
        SDL_Quit();
        return 1;
    }

    SDL_Event event;
    SDL_zero(event);
    event.type = SDL_WINDOWEVENT;
    event.window.event = SDL_WINDOWEVENT_EXPOSED;
    sc_screen_otg_handle_event(&screen, &event);
    SDL_Delay(1000);

    sc_screen_otg_destroy(&screen);
    SDL_Quit();
    return 0;
}
EOF

  cc -std=c11 -Wall -Wextra -o /tmp/scrcpy-probe/scrcpy-screen-otg-smoke \
    /tmp/scrcpy-probe/main.c \
    /tmp/scrcpy-probe/screen_otg.c \
    /tmp/scrcpy-probe/icon.c \
    -I/tmp/scrcpy-probe \
    $(pkg-config --cflags --libs sdl2)

  assert_uses_safe_sdl /tmp/scrcpy-probe/scrcpy-screen-otg-smoke

  local logfile=/tmp/scrcpy.log
  local maps_log=/tmp/scrcpy-maps.log
  local pid
  : >"$logfile"
  : >"$maps_log"

  start_xvfb
  /tmp/scrcpy-probe/scrcpy-screen-otg-smoke >"$logfile" 2>&1 &
  pid=$!

  for _ in $(seq 1 40); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" >/dev/null 2>&1 || true
      printf -- '--- scrcpy log ---\n' >&2
      cat "$logfile" >&2 || true
      die "scrcpy SDL frontend smoke exited before it loaded the safe SDL runtime"
    fi

    if grep -F -- "$SAFE_SDL_SO" "/proc/$pid/maps" >"$maps_log" 2>/dev/null; then
      terminate_pid "$pid"
      return 0
    fi

    sleep 0.25
  done

  printf -- '--- scrcpy maps ---\n' >&2
  cat "$maps_log" >&2 || true
  printf -- '--- scrcpy log ---\n' >&2
  cat "$logfile" >&2 || true
  terminate_pid "$pid"
  die "timed out waiting for scrcpy SDL frontend smoke to load the safe SDL runtime"
}

test_love() {
  local love_bin
  local logfile=/tmp/love.log
  local maps_log=/tmp/love-maps.log
  local pid
  love_bin="$(readlink -f "$(command -v love)")"
  assert_uses_safe_sdl "$love_bin"

  mkdir -p /tmp/love-smoke
  : >"$logfile"
  : >"$maps_log"
  cat >/tmp/love-smoke/main.lua <<'LUA'
local frames = 0

function love.load()
  love.window.setMode(160, 120, {resizable = false})
end

function love.update(dt)
  frames = frames + 1
  if frames > 2 then
    love.event.quit(0)
  end
end

function love.draw()
  love.graphics.clear(0.1, 0.1, 0.1)
  love.graphics.print("SDL smoke", 8, 8)
end
LUA

  start_xvfb
  env SDL_AUDIODRIVER=dummy love /tmp/love-smoke >"$logfile" 2>&1 &
  pid=$!

  for _ in $(seq 1 40); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" >/dev/null 2>&1 || true
      printf -- '--- love log ---\n' >&2
      cat "$logfile" >&2 || true
      die "love smoke exited before it loaded the safe SDL runtime"
    fi

    if grep -F -- "$SAFE_SDL_SO" "/proc/$pid/maps" >"$maps_log" 2>/dev/null; then
      terminate_pid "$pid"
      return 0
    fi

    sleep 0.25
  done

  printf -- '--- love maps ---\n' >&2
  cat "$maps_log" >&2 || true
  printf -- '--- love log ---\n' >&2
  cat "$logfile" >&2 || true
  terminate_pid "$pid"
  die "timed out waiting for love to load the safe SDL runtime"
}

test_pygame() {
  local src_root build_lib pygame_base
  local logfile=/tmp/pygame.log
  local maps_log=/tmp/pygame-maps.log
  local pid

  apt-get build-dep -y pygame

  rm -rf /tmp/pygame-source
  mkdir -p /tmp/pygame-source
  if ! (
    cd /tmp/pygame-source
    apt-get source pygame >/tmp/pygame-source.log 2>&1
  ); then
    cat /tmp/pygame-source.log >&2 || true
    die "failed to fetch pygame source package"
  fi
  src_root="$(find /tmp/pygame-source -maxdepth 1 -type d -name 'pygame-[0-9]*' | head -n1)"
  [[ -n "$src_root" ]] || die "failed to locate pygame source tree"

  if ! (
    cd "$src_root"
    PYGAME_DETECT_AVX2=1 python3 setup.py build >/tmp/pygame-build.log 2>&1
  ); then
    cat /tmp/pygame-build.log >&2 || true
    die "failed to build pygame from source"
  fi

  build_lib="$(find "$src_root"/build -maxdepth 1 -type d -name 'lib.*' | head -n1)"
  [[ -n "$build_lib" ]] || die "failed to locate built pygame module directory"

  pygame_base="$(find "$build_lib"/pygame -maxdepth 1 -type f -name 'base*.so' | head -n1)"
  [[ -n "$pygame_base" ]] || die "failed to locate built pygame base extension"
  assert_uses_safe_sdl "$pygame_base"

  : >"$logfile"
  : >"$maps_log"
  cat >/tmp/pygame-smoke.py <<'PY'
import importlib.machinery
import importlib.util
import os
import time

module_path = os.environ["PYGAME_BASE"]
loader = importlib.machinery.ExtensionFileLoader("pygame.base", module_path)
spec = importlib.util.spec_from_file_location("pygame.base", module_path, loader=loader)
module = importlib.util.module_from_spec(spec)
loader.exec_module(module)
print("loaded", flush=True)
time.sleep(60)
PY

  start_xvfb
  env SDL_AUDIODRIVER=dummy PYTHONPATH="$build_lib" PYGAME_BASE="$pygame_base" \
    python3 -X faulthandler /tmp/pygame-smoke.py >"$logfile" 2>&1 &
  pid=$!

  for _ in $(seq 1 40); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" >/dev/null 2>&1 || true
      printf -- '--- pygame log ---\n' >&2
      cat "$logfile" >&2 || true
      die "pygame smoke exited before it loaded the safe SDL runtime"
    fi

    if grep -F -- "$SAFE_SDL_SO" "/proc/$pid/maps" >"$maps_log" 2>/dev/null; then
      terminate_pid "$pid"
      return 0
    fi

    sleep 0.25
  done

  printf -- '--- pygame maps ---\n' >&2
  cat "$maps_log" >&2 || true
  printf -- '--- pygame log ---\n' >&2
  cat "$logfile" >&2 || true
  terminate_pid "$pid"
  die "timed out waiting for pygame to load the safe SDL runtime"
}

test_scummvm() {
  local scummvm_bin
  scummvm_bin="$(first_installed_elf scummvm '/scummvm$')"
  [[ -n "$scummvm_bin" ]] || die "failed to locate scummvm binary"
  assert_uses_safe_sdl "$scummvm_bin"

  start_xvfb
  run_safe_sdl_runtime_smoke scummvm \
    "$scummvm_bin" \
      --music-driver=null
}

test_supertuxkart() {
  local supertuxkart_bin
  supertuxkart_bin="$(first_installed_elf supertuxkart '/supertuxkart$')"
  [[ -n "$supertuxkart_bin" ]] || die "failed to locate supertuxkart binary"
  assert_uses_safe_sdl "$supertuxkart_bin"

  start_xvfb
  run_safe_sdl_runtime_smoke supertuxkart \
    "$supertuxkart_bin" \
      --windowed \
      --screensize=800x600 \
      --no-sound
}

test_tuxpaint() {
  local tuxpaint_bin
  tuxpaint_bin="$(first_installed_elf tuxpaint '/tuxpaint$')"
  [[ -n "$tuxpaint_bin" ]] || die "failed to locate tuxpaint binary"
  assert_uses_safe_sdl "$tuxpaint_bin"

  start_xvfb
  run_safe_sdl_runtime_smoke tuxpaint \
    "$tuxpaint_bin" \
      --nosound
}

test_openttd() {
  local openttd_bin
  local graphics_set

  openttd_bin="$(first_installed_elf openttd '/openttd$')"
  [[ -n "$openttd_bin" ]] || die "failed to locate openttd binary"
  assert_uses_safe_sdl "$openttd_bin"

  graphics_set="$("$openttd_bin" -h | awk '
    /^List of graphics sets:/ { in_graphics = 1; next }
    /^List of sounds sets:/ { in_graphics = 0 }
    in_graphics && NF && $0 !~ /unusable/ { print $1; exit }
  ')"
  [[ -n "$graphics_set" ]] || die "failed to locate a usable OpenTTD graphics set"

  start_xvfb
  run_safe_sdl_runtime_smoke openttd \
    "$openttd_bin" \
      -v sdl \
      -s null \
      -m null \
      -I "$graphics_set" \
      -g \
      -Q \
      -x
}

test_0ad() {
  local pyrogenesis_bin
  local logfile=/tmp/0ad.log
  local mainlog="$TEST_USER_HOME/.config/0ad/logs/mainlog.html"
  local interesting_log="$TEST_USER_HOME/.config/0ad/logs/interestinglog.html"
  local pid
  pyrogenesis_bin="$(first_installed_elf 0ad '/pyrogenesis$')"
  [[ -n "$pyrogenesis_bin" ]] || die "failed to locate pyrogenesis binary"
  assert_uses_safe_sdl "$pyrogenesis_bin"

  rm -rf "$TEST_USER_HOME/.config/0ad"
  : >"$logfile"
  start_xvfb
  run_as_test_user \
    "$pyrogenesis_bin" \
    -quickstart \
    -nosound \
    -xres=1024 \
    -yres=768 \
    >"$logfile" 2>&1 &
  pid=$!

  for _ in $(seq 1 480); do
    if [[ -e "$mainlog" && -e "$interesting_log" ]]; then
      terminate_pid "$pid"
      return 0
    fi

    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" >/dev/null 2>&1 || true
      printf -- '--- 0ad log ---\n' >&2
      cat "$logfile" >&2 || true
      die "0ad exited before it created user config state"
    fi

    sleep 0.25
  done

  printf -- '--- 0ad log ---\n' >&2
  cat "$logfile" >&2 || true
  terminate_pid "$pid"
  die "timed out waiting for 0ad to create user config state"
}

test_imgui() {
  local imgui_header imgui_backend_header imgui_backend_cpp imgui_include_dir imgui_backend_dir stb_rect_pack_header stb_include_dir

  imgui_header="$(first_installed_path libimgui-dev '/imgui\.h$')"
  imgui_backend_header="$(first_installed_path libimgui-dev '/imgui_impl_sdl2\.h$')"
  imgui_backend_cpp="$(first_installed_path libimgui-dev '/imgui_impl_sdl2\.cpp$' || true)"
  stb_rect_pack_header="$(first_installed_path libstb-dev '/stb_rect_pack\.h$')"

  [[ -n "$imgui_header" ]] || die "failed to locate imgui headers"
  [[ -n "$imgui_backend_header" ]] || die "failed to locate imgui SDL backend headers"
  [[ -n "$stb_rect_pack_header" ]] || die "failed to locate stb headers"

  imgui_include_dir="$(dirname "$imgui_header")"
  imgui_backend_dir="$(dirname "$imgui_backend_header")"
  stb_include_dir="$(dirname "$stb_rect_pack_header")"

  if [[ -z "$imgui_backend_cpp" ]]; then
    rm -rf /tmp/imgui-source
    mkdir -p /tmp/imgui-source
    (
      cd /tmp/imgui-source
      apt-get source imgui >/tmp/imgui-source.log 2>&1
    )
    imgui_backend_cpp="$(find /tmp/imgui-source -type f -path '*/backends/imgui_impl_sdl2.cpp' | head -n1)"
  fi

  [[ -n "$imgui_backend_cpp" && -f "$imgui_backend_cpp" ]] || die "failed to locate imgui SDL backend source"

  cat >/tmp/imgui-probe.cpp <<'CPP'
#include <SDL.h>
#include <imgui.h>
#include <backends/imgui_impl_sdl2.h>

int main() {
  IMGUI_CHECKVERSION();
  ImGui::CreateContext();
  SDL_Event event;
  SDL_zero(event);
  (void)ImGui_ImplSDL2_ProcessEvent(&event);
  ImGui::DestroyContext();
  return 0;
}
CPP

  cat >/tmp/imgui-stb.cpp <<'CPP'
#define STB_RECT_PACK_IMPLEMENTATION
#define STB_TRUETYPE_IMPLEMENTATION
#include <stb_rect_pack.h>
#include <stb_truetype.h>
CPP

  g++ -std=c++17 -o /tmp/imgui-probe \
    /tmp/imgui-probe.cpp \
    /tmp/imgui-stb.cpp \
    "$imgui_backend_cpp" \
    -I"$imgui_include_dir" \
    -I"$imgui_backend_dir" \
    -I"$stb_include_dir" \
    $(pkg-config --cflags --libs sdl2) \
    -L"/usr/lib/${MULTIARCH}" \
    -limgui

  assert_uses_safe_sdl /tmp/imgui-probe
}

test_libtcod() {
  cat >/tmp/libtcod-probe.c <<'C'
#include <libtcod.h>

int main(void) {
  TCOD_Console* console = TCOD_console_new(1, 1);
  if (!console) {
    return 1;
  }
  TCOD_console_delete(console);
  return 0;
}
C

  cc -std=c11 -o /tmp/libtcod-probe \
    /tmp/libtcod-probe.c \
    $(pkg-config --cflags --libs libtcod)

  assert_uses_safe_sdl /tmp/libtcod-probe
  /tmp/libtcod-probe
}

should_run() {
  local slug="$1"
  local manifest_name="$2"

  if selection_matches "$slug" "$manifest_name"; then
    MATCHED_ONLY=1
    return 0
  fi

  return 1
}

run_case() {
  local slug="$1"
  local manifest_name="$2"
  local function_name="$3"
  local case_dir="$ARTIFACT_DIR/$slug"
  local status duration note
  local start_ts end_ts

  if should_run "$slug" "$manifest_name"; then
    log_step "$manifest_name"
    rm -rf "$case_dir"
    mkdir -p "$case_dir"
    start_ts="$(date +%s)"
    note=""
    if ( "$function_name" ) 2>&1 | tee "$case_dir/console.log"; then
      status="passed"
    else
      status="failed"
      note="see artifacts in ${ARTIFACT_HOST_DIR}/${slug}"
      FAILED_CASES=$((FAILED_CASES + 1))
    fi
    end_ts="$(date +%s)"
    duration=$((end_ts - start_ts))
    collect_case_artifacts "$slug" "$case_dir"
    record_case_result "$slug" "$manifest_name" "$status" "$duration" "$case_dir" "$note"
  fi
}

validate_dependents_inventory
setup_test_user
prepare_safe_source_tree
install_runtime_packages
build_safe_sdl

run_case qemu "QEMU system GUI modules" test_qemu
run_case ffmpeg "FFmpeg" test_ffmpeg
run_case scrcpy "scrcpy" test_scrcpy
run_case love "LOVE" test_love
run_case pygame "pygame" test_pygame
run_case scummvm "ScummVM" test_scummvm
run_case supertuxkart "SuperTuxKart" test_supertuxkart
run_case tuxpaint "Tux Paint" test_tuxpaint
run_case openttd "OpenTTD" test_openttd
run_case 0ad "0 A.D." test_0ad
run_case imgui "Dear ImGui development package" test_imgui
run_case libtcod "libtcod development package" test_libtcod

if [[ -n "$ONLY_FILTER" && "$MATCHED_ONLY" != "1" ]]; then
  die "unknown dependent selector: $ONLY_FILTER"
fi

write_results_json

if (( FAILED_CASES > 0 )); then
  printf 'error: %d dependent checks failed\n' "$FAILED_CASES" >&2
  exit 1
fi
CONTAINER_SCRIPT
docker_status=$?
set -e

if [[ -f "$ARTIFACT_DIR/results.json" && -n "$JSON_OUT" ]]; then
  if [[ "$(readlink -f "$ARTIFACT_DIR/results.json")" != "$(readlink -f "$JSON_OUT")" ]]; then
    cp "$ARTIFACT_DIR/results.json" "$JSON_OUT"
  fi
fi

exit "$docker_status"
