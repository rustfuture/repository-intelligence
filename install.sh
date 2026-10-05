#!/bin/sh
# Installer for the `repository-intelligence` CLI (https://github.com/rustfuture/repository-intelligence).
#
#   curl -fsSL https://raw.githubusercontent.com/rustfuture/repository-intelligence/main/install.sh | sh
#
# Downloads the prebuilt release archive for this machine, checks its SHA-256
# checksum and copies `repository-intelligence` into an install directory. It never uses sudo
# and never edits shell startup files.
#
# Environment variables (all optional):
#   RI_VERSION        Version to install, e.g. 0.3.0 or v0.3.0 (default: latest release)
#   RI_INSTALL_DIR    Where to put the binary (default: $HOME/.local/bin)
#   RI_DOWNLOAD_BASE  Base URL (or file:// URL) holding the archive and its
#                         .sha256 file, instead of the GitHub release. Needs
#                         RI_VERSION. Meant for mirrors and testing.
#   RI_REPO           GitHub repository (default: rustfuture/repository-intelligence)

set -eu

REPO="${RI_REPO:-rustfuture/repository-intelligence}"
FROM_SOURCE="cargo install --git https://github.com/${REPO} repository-intelligence"

say() { printf '%s\n' "$*"; }
err() { printf 'repository-intelligence-install: error: %s\n' "$*" >&2; }
die() { err "$*"; exit 1; }

have() { command -v "$1" >/dev/null 2>&1; }

unsupported() {
  err "$1"
  err "No prebuilt binary is available for this platform. If you have Rust (1.85 or newer), build from source:"
  err "  ${FROM_SOURCE}"
  exit 1
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$arch" in
    x86_64 | amd64) arch="x86_64" ;;
    aarch64 | arm64) arch="aarch64" ;;
    *) unsupported "unsupported CPU architecture: ${arch}" ;;
  esac

  case "$os" in
    Linux)
      # The Linux binaries are linked against glibc; musl (e.g. Alpine) cannot run them.
      if have ldd && ldd --version 2>&1 | grep -qi musl; then
        unsupported "this Linux uses musl libc, but the prebuilt binaries need glibc"
      fi
      TARGET="${arch}-unknown-linux-gnu"
      ;;
    Darwin)
      # A shell running under Rosetta reports x86_64 on an Apple Silicon Mac.
      if [ "$arch" = "x86_64" ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = "1" ]; then
        arch="aarch64"
      fi
      TARGET="${arch}-apple-darwin"
      ;;
    MINGW* | MSYS* | CYGWIN*)
      die "this looks like Windows. Use the PowerShell installer instead: irm https://raw.githubusercontent.com/${REPO}/main/install.ps1 | iex"
      ;;
    *)
      unsupported "unsupported operating system: ${os}"
      ;;
  esac
}

# fetch URL DEST
fetch() {
  if have curl; then
    curl -fsSL --retry 3 -o "$2" "$1" || return 1
  elif have wget; then
    wget -q -O "$2" "$1" || return 1
  else
    die "need curl or wget to download files"
  fi
}

# fetch_stdout URL
fetch_stdout() {
  if have curl; then
    curl -fsSL --retry 3 "$1"
  elif have wget; then
    wget -q -O - "$1"
  else
    die "need curl or wget to download files"
  fi
}

resolve_version() {
  if [ -n "${RI_VERSION:-}" ]; then
    VERSION="${RI_VERSION#v}"
  elif [ -n "${RI_DOWNLOAD_BASE:-}" ]; then
    die "RI_DOWNLOAD_BASE is set, so also set RI_VERSION (for example RI_VERSION=0.3.0)"
  else
    say "Looking up the latest release..."
    json="$(fetch_stdout "https://api.github.com/repos/${REPO}/releases/latest")" ||
      die "could not look up the latest release (no release published yet, no network, or the GitHub API rate limit). Set RI_VERSION, e.g. RI_VERSION=0.3.0"
    tag="$(printf '%s\n' "$json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
    [ -n "$tag" ] || die "could not read the latest release tag. Set RI_VERSION, e.g. RI_VERSION=0.3.0"
    VERSION="${tag#v}"
  fi
  case "$VERSION" in
    '' | *[!0-9A-Za-z.+-]*) die "invalid version: ${VERSION}" ;;
  esac
}

sha256_of() {
  if have sha256sum; then
    sha256sum "$1" | awk '{print $1}'
  elif have shasum; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "need sha256sum or shasum to verify the download"
  fi
}

main() {
  have tar || die "need tar to unpack the download"
  have curl || have wget || die "need curl or wget to download files"

  detect_target
  resolve_version

  INSTALL_DIR="${RI_INSTALL_DIR:-${HOME:?HOME is not set}/.local/bin}"
  NAME="repository-intelligence-${VERSION}-${TARGET}"
  ARCHIVE="${NAME}.tar.gz"
  BASE="${RI_DOWNLOAD_BASE:-https://github.com/${REPO}/releases/download/v${VERSION}}"
  BASE="${BASE%/}"

  TMP="$(mktemp -d 2>/dev/null || mktemp -d -t repository-intelligence-install)"
  trap 'rm -rf "$TMP"' EXIT INT TERM HUP

  say "Installing repository-intelligence ${VERSION} for ${TARGET}"
  fetch "${BASE}/${ARCHIVE}" "${TMP}/${ARCHIVE}" ||
    die "could not download ${BASE}/${ARCHIVE} (is ${VERSION} a published version for ${TARGET}?)"
  fetch "${BASE}/${ARCHIVE}.sha256" "${TMP}/${ARCHIVE}.sha256" ||
    die "could not download ${BASE}/${ARCHIVE}.sha256"

  expected="$(awk 'NR == 1 {print tolower($1)}' "${TMP}/${ARCHIVE}.sha256")"
  actual="$(sha256_of "${TMP}/${ARCHIVE}")"
  case "$expected" in
    '' | *[!0-9a-f]*) die "the checksum file ${ARCHIVE}.sha256 is not valid" ;;
  esac
  if [ "${#expected}" -ne 64 ]; then
    die "the checksum file ${ARCHIVE}.sha256 is not valid"
  fi
  if [ "$expected" != "$actual" ]; then
    err "checksum mismatch for ${ARCHIVE}"
    err "  expected: ${expected}"
    err "  actual:   ${actual}"
    die "refusing to install a file that does not match its checksum"
  fi
  say "Checksum OK"

  tar -xzf "${TMP}/${ARCHIVE}" -C "$TMP" || die "could not unpack ${ARCHIVE}"
  [ -f "${TMP}/${NAME}/repository-intelligence" ] || die "the archive does not contain ${NAME}/repository-intelligence"

  mkdir -p "$INSTALL_DIR" || die "could not create ${INSTALL_DIR}; set RI_INSTALL_DIR to a writable directory"
  # Copy under a temporary name and rename, so a running repository-intelligence is replaced safely.
  cp "${TMP}/${NAME}/repository-intelligence" "${INSTALL_DIR}/.repository-intelligence.new.$$" ||
    die "could not write to ${INSTALL_DIR}; set RI_INSTALL_DIR to a writable directory"
  chmod 755 "${INSTALL_DIR}/.repository-intelligence.new.$$"
  mv -f "${INSTALL_DIR}/.repository-intelligence.new.$$" "${INSTALL_DIR}/repository-intelligence"

  say "Installed: ${INSTALL_DIR}/repository-intelligence"
  "${INSTALL_DIR}/repository-intelligence" --version ||
    die "the installed binary did not run; it may not suit this system. Build from source instead: ${FROM_SOURCE}"

  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      say ""
      say "${INSTALL_DIR} is not on your PATH. Add it, for example:"
      say "  export PATH=\"${INSTALL_DIR}:\$PATH\""
      say "(put that line in your shell startup file to keep it; this installer does not edit it)"
      ;;
  esac

  say ""
  say "Next: run 'repository-intelligence --help' to get started."
}

main "$@"
