#!/usr/bin/env bash

mkdir -p svgs

# -exec cargo run -- -f "/Library/Fonts/Microsoft/Cambria Math.ttf" {} svgs/{}.svg \;
cargo build
#find -E ../tests/testfiles -regex .*.xml -print0 | xargs -t -P8 -I{} -0 ../target/debug/mathimg -f "/Library/Fonts/Microsoft/Cambria Math.ttf" {} svgs
find -E ../tests/testfiles -regex .*.xml -print0 | xargs -t -P8 -I{} -0 ../target/debug/mathimg --show-ink-bounds --show-logical-bounds -f ~/Library/Fonts/latinmodern-math.otf {} svgs
#find -E ../tests/testfiles -regex .*.xml -print0 | xargs -t -P8 -I{} -0 ../target/debug/mathimg -f "/Users/mr/Library/Fonts/texgyredejavu-math.otf" {} svgs
#find -E ../tests/testfiles -regex .*.xml -print0 | xargs -t -P8 -I{} -0 ../target/debug/mathimg -f "/Users/mr/Library/Fonts/STIX2Math.otf" {} svgs
#find -E ../tests/testfiles -regex .*.xml -print0 | xargs -t -P8 -I{} -0 ../target/debug/mathimg -f "/Users/mr/Library/Fonts/texgyretermes-math.otf" {} svgs
#find -E ../tests/testfiles -regex .*.xml -print0 | xargs -t -P8 -I{} -0 ../target/debug/mathimg -f "/Users/mr/Library/Fonts/texgyreschola-math.otf" {} svgs