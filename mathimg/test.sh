#!/usr/bin/env bash

mkdir -p svgs

# -exec cargo run -- -f "/Library/Fonts/Microsoft/Cambria Math.ttf" {} svgs/{}.svg \;
cargo build
find -E ../tests/testfiles -regex .*.xml -print0 | xargs -t -P8 -I{} -0 target/debug/mathimg -f "/Library/Fonts/Microsoft/Cambria Math.ttf" --show-ink-bounds --show-logical-bounds {} svgs
#find -E ../tests/testfiles -regex .*.xml -print0 | xargs -t -P8 -I{} -0 target/debug/mathimg -f ~/Library/Fonts/latinmodern-math.otf {} svgs
