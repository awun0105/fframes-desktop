#!/usr/bin/env bash
# Installs everything an fframes video project needs to build on macOS or Linux: the system
# packages (compiler, nasm, pkg-config, codec libraries) and Rust (rustup, minimal profile).
# Steps already satisfied are skipped, so running it again is safe.
#
#   curl -fsSL https://raw.githubusercontent.com/khanhthanhdev/fframes-desktop/main/scripts/install-deps.sh | bash
#
# On Windows use scripts/install-deps.ps1. fframes Studio users do not need this: Studio ships
# Rust and FFmpeg in its managed SDK.
set -euo pipefail

say() { printf '\033[1m%s\033[0m\n' "$*" >&2; }
fail() { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
sudo_if_needed() { if [[ $(id -u) -eq 0 ]]; then "$@"; else sudo "$@"; fi; }

case "$(uname -s)" in
  Darwin)
    if ! xcode-select -p >/dev/null 2>&1; then
      say "Installing the Xcode Command Line Tools; finish the dialog, then run this script again"
      xcode-select --install
      exit 1
    fi
    command -v brew >/dev/null 2>&1 || fail "Homebrew is required: https://brew.sh"
    say "Installing system packages with Homebrew"
    brew install pkg-config ffmpeg x264 x265 opus nasm ninja
    ;;
  Linux)
    if command -v apt-get >/dev/null 2>&1; then
      say "Installing system packages with apt"
      sudo_if_needed apt-get update
      sudo_if_needed apt-get install -y curl build-essential pkg-config yasm nasm ffmpeg libx264-dev libx265-dev \
        libopus-dev libclang-dev clang ninja-build libvpx-dev libasound2-dev libfontconfig1-dev
    elif command -v pacman >/dev/null 2>&1; then
      say "Installing system packages with pacman"
      sudo_if_needed pacman -S --needed --noconfirm curl base-devel pkgconf ninja yasm nasm ffmpeg x264 x265 opus clang libvpx alsa-lib fontconfig
    else
      fail "No apt-get or pacman found; install the packages listed under Requirements in README.md, or use nix-shell"
    fi
    ;;
  *) fail "Unsupported system $(uname -s); on Windows use scripts/install-deps.ps1" ;;
esac

if ! command -v cargo >/dev/null 2>&1 && [[ ! -x "$HOME/.cargo/bin/cargo" ]]; then
  say "Installing Rust (rustup, minimal profile)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
fi
# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
say "Rust: $(cargo --version)"

say "Done. Open a new terminal (or run: source \"\$HOME/.cargo/env\"), then:"
say "  cargo install --locked cargo-fframes"
say "  cargo fframes new my-video"
