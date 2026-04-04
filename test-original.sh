#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_TAG="${LIBSDL_ORIGINAL_TEST_IMAGE:-libsdl-original-test:ubuntu24.04}"
ONLY=""

usage() {
  cat <<'EOF'
usage: test-original.sh [--only <slug-or-manifest-name>]

Builds the upstream SDL source from ./original inside an Ubuntu 24.04 Docker
container, installs it into /usr/local, and then exercises the dependent
software listed in dependents.json.

--only runs a single dependent check. Accepted values include:
  qemu, ffmpeg, scrcpy, love, pygame, scummvm, supertuxkart,
  tuxpaint, openttd, 0ad, imgui, libtcod
EOF
}

while (($#)); do
  case "$1" in
    --only)
      ONLY="${2:?missing value for --only}"
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

[[ -f "$ROOT/dependents.json" ]] || {
  echo "missing dependents.json" >&2
  exit 1
}

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
 && rm -rf /var/lib/apt/lists/*
DOCKERFILE

docker run --rm -i \
  -e "LIBSDL_TEST_ONLY=$ONLY" \
  -v "$ROOT":/work:ro \
  "$IMAGE_TAG" \
  bash -s <<'CONTAINER_SCRIPT'
set -euo pipefail

export LANG=C.UTF-8
export LC_ALL=C.UTF-8

ROOT=/work
ONLY_FILTER="${LIBSDL_TEST_ONLY:-}"
HOME=/tmp/libsdl-home
MULTIARCH="$(gcc -print-multiarch)"
ORIGINAL_SDL_SO=""
ORIGINAL_SDL_LIBDIR=""
ORIGINAL_SDL_PKGCONFIG_DIR=""
XVFB_PID=""
MATCHED_ONLY=0
TEST_USER=libsdltest
TEST_USER_RUNTIME_DIR="/tmp/${TEST_USER}-runtime"

mkdir -p "$HOME"

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
    "QEMU system GUI modules",
    "FFmpeg",
    "scrcpy",
    "LOVE",
    "pygame",
    "ScummVM",
    "SuperTuxKart",
    "Tux Paint",
    "OpenTTD",
    "0 A.D.",
    "Dear ImGui development package",
    "libtcod development package",
]

data = json.loads(Path("/work/dependents.json").read_text(encoding="utf-8"))
actual = [entry["name"] for entry in data["dependents"]]
if actual != expected:
    raise SystemExit(
        f"unexpected dependents.json contents: expected {expected}, found {actual}"
    )
PY
}

apt_install() {
  apt-get install -y --no-install-recommends "$@"
}

setup_test_user() {
  if ! id -u "$TEST_USER" >/dev/null 2>&1; then
    useradd --home-dir "$HOME" --create-home --shell /bin/bash "$TEST_USER"
  fi

  mkdir -p "$HOME" "$TEST_USER_RUNTIME_DIR"
  chown -R "$TEST_USER:$TEST_USER" "$HOME" "$TEST_USER_RUNTIME_DIR"
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
  apt-get build-dep -y libsdl2

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
  selection_matches tuxpaint "Tux Paint" && packages+=(tuxpaint)

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

build_dummy_package() {
  local package_name="$1"
  local version="$2"
  local staging_root="/tmp/dummy-${package_name}"
  local output="/tmp/${package_name}_${version}_all.deb"

  rm -rf "$staging_root"
  mkdir -p "$staging_root/DEBIAN"
  cat >"$staging_root/DEBIAN/control" <<EOF
Package: ${package_name}
Version: ${version}
Section: misc
Priority: optional
Architecture: all
Maintainer: SDL smoke tests <noreply@example.com>
Description: Minimal dummy asset package for SDL dependent smoke tests.
EOF

  dpkg-deb --build "$staging_root" "$output" >/dev/null
  printf '%s\n' "$output"
}

install_heavy_packages_without_assets() {
  local need_0ad=0
  local need_supertuxkart=0

  selection_matches 0ad "0 A.D." && need_0ad=1
  selection_matches supertuxkart "SuperTuxKart" && need_supertuxkart=1

  if [[ "$need_0ad" != "1" && "$need_supertuxkart" != "1" ]]; then
    return 0
  fi

  log_step "Installing heavy dependents with dummy asset packages"

  local dummy_packages=()
  local runtime_packages=()

  if [[ "$need_0ad" == "1" ]]; then
    dummy_packages+=(
      "$(build_dummy_package 0ad-data 0.0.26)"
      "$(build_dummy_package 0ad-data-common 0.0.26)"
    )
    runtime_packages+=(0ad)
  fi

  if [[ "$need_supertuxkart" == "1" ]]; then
    dummy_packages+=(
      "$(build_dummy_package supertuxkart-data 1.4+dfsg-3ubuntu1)"
    )
    runtime_packages+=(supertuxkart)
  fi

  apt_install "${dummy_packages[@]}"
  apt-mark hold 0ad-data 0ad-data-common supertuxkart-data >/dev/null 2>&1 || true
  apt_install "${runtime_packages[@]}"
}

build_original_sdl() {
  log_step "Building original SDL"

  rm -rf /tmp/libsdl-original
  rsync -a --delete "$ROOT/original/" /tmp/libsdl-original/

  (
    cd /tmp/libsdl-original
    ./configure --prefix=/usr/local --disable-static >/tmp/libsdl-configure.log 2>&1
    make -j"$(nproc)" >/tmp/libsdl-make.log 2>&1
    make install >/tmp/libsdl-install.log 2>&1
  )

  local installed_sdl installed_pc
  installed_sdl="$(find /usr/local \( -type f -o -type l \) -name 'libSDL2-2.0.so.0*' | sort | head -n1)"
  installed_pc="$(find /usr/local -type f -path '*/pkgconfig/sdl2.pc' | sort | head -n1)"

  [[ -n "$installed_sdl" ]] || die "failed to locate installed libSDL2-2.0.so.0"
  [[ -n "$installed_pc" ]] || die "failed to locate installed sdl2.pc"

  ORIGINAL_SDL_SO="$(readlink -f "$installed_sdl")"
  ORIGINAL_SDL_LIBDIR="$(dirname "$ORIGINAL_SDL_SO")"
  ORIGINAL_SDL_PKGCONFIG_DIR="$(dirname "$installed_pc")"

  [[ -n "$ORIGINAL_SDL_SO" && -f "$ORIGINAL_SDL_SO" ]] || die "failed to locate installed libSDL2-2.0.so.0"
  [[ -n "$ORIGINAL_SDL_PKGCONFIG_DIR" && -d "$ORIGINAL_SDL_PKGCONFIG_DIR" ]] || die "failed to locate installed sdl2.pc"

  printf '%s\n' "$ORIGINAL_SDL_LIBDIR" >/etc/ld.so.conf.d/zz-libsdl-local.conf
  ldconfig

  export PATH="/usr/local/bin:$PATH"
  export LD_LIBRARY_PATH="$ORIGINAL_SDL_LIBDIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  export PKG_CONFIG_PATH="$ORIGINAL_SDL_PKGCONFIG_DIR${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

  pkg-config --variable=prefix sdl2 | grep -Fx '/usr/local' >/dev/null
}

cleanup_xvfb() {
  if [[ -n "$XVFB_PID" ]] && kill -0 "$XVFB_PID" >/dev/null 2>&1; then
    kill "$XVFB_PID" >/dev/null 2>&1 || true
    wait "$XVFB_PID" >/dev/null 2>&1 || true
  fi
}

trap cleanup_xvfb EXIT

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
    HOME="$HOME" \
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

assert_uses_original_sdl() {
  local target="$1"
  local resolved

  resolved="$(ldd "$target" 2>/dev/null | awk '$1 == "libSDL2-2.0.so.0" { print $3; exit }')"
  [[ -n "$resolved" ]] || die "ldd did not report libSDL2-2.0.so.0 for $target"
  resolved="$(readlink -f "$resolved")"
  [[ "$resolved" == "$ORIGINAL_SDL_SO" ]] || {
    printf 'expected %s to resolve libSDL2-2.0.so.0 from %s, got %s\n' "$target" "$ORIGINAL_SDL_SO" "$resolved" >&2
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

  for _ in $(seq 1 80); do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      cat "$logfile" >&2 || true
      die "$slug exited before creating a window"
    fi

    if xdotool search --onlyvisible --name "$window_pattern" >/tmp/${slug}-windows.log 2>/dev/null \
      || xwininfo -root -tree 2>/dev/null | grep -F "\"$window_pattern\"" >/tmp/${slug}-windows.log
    then
      found=1
      break
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

test_qemu() {
  local ui_module
  ui_module="$(first_installed_path qemu-system-gui '/ui-sdl\.so$')"
  [[ -n "$ui_module" ]] || die "failed to locate qemu SDL UI module"
  assert_uses_original_sdl "$ui_module"

  start_xvfb
  run_window_smoke qemu 'QEMU' \
    qemu-system-x86_64 \
      -display sdl,gl=off \
      -accel tcg \
      -m 64 \
      -serial none \
      -monitor none
}

test_ffmpeg() {
  assert_uses_original_sdl "$(command -v ffplay)"

  start_xvfb
  timeout 30 env SDL_AUDIODRIVER=dummy \
    ffplay -v error -autoexit -f lavfi -i 'testsrc=size=128x96:rate=1:duration=1' \
    >/tmp/ffmpeg.log 2>&1
}

test_scrcpy() {
  local scrcpy_elf
  scrcpy_elf="$(first_installed_elf scrcpy '/scrcpy$')"
  [[ -n "$scrcpy_elf" ]] || die "failed to locate scrcpy ELF binary"
  assert_uses_original_sdl "$scrcpy_elf"

  scrcpy --version >/tmp/scrcpy.log 2>&1
  require_contains /tmp/scrcpy.log "scrcpy "
}

test_love() {
  local love_bin
  love_bin="$(readlink -f "$(command -v love)")"
  assert_uses_original_sdl "$love_bin"

  mkdir -p /tmp/love-smoke
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
  timeout 30 env SDL_AUDIODRIVER=dummy love /tmp/love-smoke >/tmp/love.log 2>&1
}

test_pygame() {
  local src_root build_lib pygame_base

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
  assert_uses_original_sdl "$pygame_base"

  start_xvfb
  if ! env SDL_AUDIODRIVER=dummy PYTHONPATH="$build_lib" python3 -X faulthandler <<'PY' >/tmp/pygame.log 2>&1
import pygame

print("imported", flush=True)
pygame.init()
print("init", flush=True)
pygame.mixer.init()
print("mixer", flush=True)
screen = pygame.display.set_mode((160, 120))
print("display", flush=True)
pygame.display.set_caption("pygame smoke")
screen.fill((17, 34, 51))
pygame.display.flip()
print(screen.get_size(), flush=True)
pygame.time.wait(100)
pygame.quit()
print("quit", flush=True)
PY
  then
    cat /tmp/pygame.log >&2 || true
    die "pygame smoke script failed"
  fi
  require_contains /tmp/pygame.log "(160, 120)"
}

test_scummvm() {
  local scummvm_bin
  scummvm_bin="$(first_installed_elf scummvm '/scummvm$')"
  [[ -n "$scummvm_bin" ]] || die "failed to locate scummvm binary"
  assert_uses_original_sdl "$scummvm_bin"

  start_xvfb
  run_window_smoke scummvm 'ScummVM' \
    "$scummvm_bin" \
      --music-driver=null
}

test_supertuxkart() {
  local supertuxkart_bin
  supertuxkart_bin="$(first_installed_elf supertuxkart '/supertuxkart$')"
  [[ -n "$supertuxkart_bin" ]] || die "failed to locate supertuxkart binary"
  assert_uses_original_sdl "$supertuxkart_bin"

  "$supertuxkart_bin" --help >/tmp/supertuxkart.log 2>&1
  if ! grep -E 'SuperTuxKart|supertuxkart' /tmp/supertuxkart.log >/dev/null 2>&1; then
    cat /tmp/supertuxkart.log >&2
    die "supertuxkart help output did not contain an expected identifier"
  fi
}

test_tuxpaint() {
  local tuxpaint_bin
  tuxpaint_bin="$(first_installed_elf tuxpaint '/tuxpaint$')"
  [[ -n "$tuxpaint_bin" ]] || die "failed to locate tuxpaint binary"
  assert_uses_original_sdl "$tuxpaint_bin"

  start_xvfb
  run_window_smoke tuxpaint 'Tux Paint' \
    "$tuxpaint_bin" \
      --nosound
}

test_openttd() {
  local openttd_bin
  local graphics_set

  openttd_bin="$(first_installed_elf openttd '/openttd$')"
  [[ -n "$openttd_bin" ]] || die "failed to locate openttd binary"
  assert_uses_original_sdl "$openttd_bin"

  graphics_set="$("$openttd_bin" -h | awk '
    /^List of graphics sets:/ { in_graphics = 1; next }
    /^List of sounds sets:/ { in_graphics = 0 }
    in_graphics && NF && $0 !~ /unusable/ { print $1; exit }
  ')"
  [[ -n "$graphics_set" ]] || die "failed to locate a usable OpenTTD graphics set"

  start_xvfb
  run_window_smoke openttd 'OpenTTD' \
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
  pyrogenesis_bin="$(first_installed_elf 0ad '/pyrogenesis$')"
  [[ -n "$pyrogenesis_bin" ]] || die "failed to locate pyrogenesis binary"
  assert_uses_original_sdl "$pyrogenesis_bin"

  if ! run_as_test_user "$pyrogenesis_bin" --version >/tmp/0ad.log 2>&1; then
    if ! run_as_test_user "$pyrogenesis_bin" -version >/tmp/0ad.log 2>&1; then
      run_as_test_user "$pyrogenesis_bin" -help >/tmp/0ad.log 2>&1 || true
    fi
  fi

  if ! grep -Ei '0 A\.D\.|pyrogenesis|Usage' /tmp/0ad.log >/dev/null 2>&1; then
    cat /tmp/0ad.log >&2
    die "0ad probe output did not contain an expected identifier"
  fi
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

  assert_uses_original_sdl /tmp/imgui-probe
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

  assert_uses_original_sdl /tmp/libtcod-probe
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

  if should_run "$slug" "$manifest_name"; then
    log_step "$manifest_name"
    "$function_name"
  fi
}

validate_dependents_inventory
setup_test_user
install_runtime_packages
install_heavy_packages_without_assets
build_original_sdl

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
CONTAINER_SCRIPT
