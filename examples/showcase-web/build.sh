#!/bin/sh
# Builds the web showcase into dist/. Needs wasm-bindgen-cli at the version in
# Cargo.lock: cargo install wasm-bindgen-cli --version 0.2.130 --locked
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
cargo build --manifest-path "$root/Cargo.toml" -p showcase-web --release --target wasm32-unknown-unknown
rm -rf "$here/dist"
mkdir -p "$here/dist"
"${WASM_BINDGEN:-wasm-bindgen}" --target web --no-typescript --out-dir "$here/dist" \
  "$root/target/wasm32-unknown-unknown/release/showcase_web.wasm"
cp "$here/www/index.html" "$here/dist/"
echo "built $here/dist; serve it with: python3 -m http.server -d $here/dist 8080"
