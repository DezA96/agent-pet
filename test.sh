#!/bin/bash
# Runs both test suites: the Swift placement arithmetic and the Rust core.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# swift-testing ships with the Command Line Tools, but off the default search
# paths — SwiftPM finds neither the framework nor the interop dylib it loads at
# run time. XCTest is not an option here: it ships inside Xcode, which this
# project deliberately does not require. These four flags are the whole reason
# this script exists rather than a bare `swift test`.
DEV="$(xcode-select -p)"
FRAMEWORKS="$DEV/Library/Developer/Frameworks"
LIBS="$DEV/Library/Developer/usr/lib"

if [ ! -d "$FRAMEWORKS/Testing.framework" ]; then
  echo "error: Testing.framework not found under $FRAMEWORKS" >&2
  echo "       the Command Line Tools may be too old for swift-testing" >&2
  exit 1
fi

echo "==> swift: placement arithmetic"
(cd "$ROOT/macos" && swift test \
  -Xswiftc -F -Xswiftc "$FRAMEWORKS" \
  -Xlinker -F -Xlinker "$FRAMEWORKS" \
  -Xlinker -rpath -Xlinker "$FRAMEWORKS" \
  -Xlinker -rpath -Xlinker "$LIBS")

echo "==> rust: observation core"
cargo test --manifest-path "$ROOT/core/Cargo.toml"
