#!/bin/zsh
# Builds Bench.swift against each binding and runs both on a booted iOS simulator.
# Usage: run-apple.sh <simulator udid>. Run after `boltffi pack apple --release` and
# the UniFFI steps in README.md.
set -eu
cd "${0:A:h}/.."
udid=$1
sdk=$(xcrun --sdk iphonesimulator --show-sdk-path)
common=(-O -wmo -parse-as-library -swift-version 5 -target arm64-apple-ios16.0-simulator -sdk $sdk -suppress-warnings)
xcrun swiftc $common -I bolt/dist/apple/FFI/include -Xcc -Ibolt/dist/apple/BoltBench.xcframework/ios-arm64_x86_64-simulator/Headers \
  harness/swift/Bench.swift bolt/dist/apple/Sources/BoltFFI/BoltBenchBoltFFI.swift \
  bolt/dist/apple/BoltBench.xcframework/ios-arm64_x86_64-simulator/libbolt-bench.a \
  -o out/bench-bolt-ios
xcrun swiftc $common -I uni/dist/swift -Xcc -fmodule-map-file=uni/dist/swift/uni_benchFFI.modulemap \
  harness/swift/Bench.swift uni/dist/swift/uni_bench.swift \
  target/aarch64-apple-ios-sim/release/libuni_bench.a \
  -o out/bench-uni-ios
for backend in bolt uni; do
  echo "== $backend ios-simulator"
  xcrun simctl spawn $udid "$PWD/out/bench-$backend-ios"
done
