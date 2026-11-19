#!/bin/sh
# stage.sh TARGET [STAGE_DIR]
set -eu
cd "$(dirname "$0")/.."
target=${1:?specify a Rust target}
[ "$target" != armhf ] || target=armv7-unknown-linux-musleabihf

stage=${2:-build/stage}
target_dir=${CARGO_TARGET_DIR:-target}
bin=$target_dir/$target/release/minesweeper-kindle
[ -x "$bin" ] || { echo "Build $bin before staging." >&2; exit 1; }
install -d "$stage/minesweeper/bin" "$stage/documents"
install -m 755 "$bin" "$stage/minesweeper/bin/minesweeper-kindle"
install -m 755 deploy/Minesweeper.sh "$stage/documents/Minesweeper.sh"
