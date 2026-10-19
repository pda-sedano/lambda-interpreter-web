#!/bin/sh
# Rebuild ulc/pkg/ without embedding local folder paths in the .wasm.
# Usage (from anywhere): sh ulc/build.sh
set -eu
cd "$(dirname "$0")"

# Rust records source paths of dependencies and the standard library
# (e.g. ~/.cargo/registry/..., ~/.rustup/...) for panic messages.
# Rewrite the home-folder prefix to "~" so it never ends up in pkg/.
export RUSTFLAGS="--remap-path-prefix=$HOME=~ ${RUSTFLAGS:-}"

wasm-pack build --release --target web
rm -f pkg/.gitignore   # wasm-pack writes "*", which would keep pkg/ out of git

if grep -rqF "$HOME" pkg/; then
  echo "error: pkg/ still contains your home folder path; not safe to commit" >&2
  exit 1
fi
echo "pkg/ rebuilt; no home-folder paths found."
