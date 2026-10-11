#!/bin/bash
# Bundle id of every extension built by build-extensions.sh.
cd "$(dirname "$0")/.."
for proj in src/ios/*/*.xcodeproj/project.pbxproj; do
    sed -n 's/.*PRODUCT_BUNDLE_IDENTIFIER = "*\([^";]*\)"*;/\1/p' "$proj" | head -1
done
