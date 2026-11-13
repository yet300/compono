#!/bin/zsh
# Runs both sides on an iOS simulator and an Android device or emulator.
# Usage: run-devices.sh <simulator udid>. Needs GRADLE, ANDROID_NDK_HOME and ANDROID_SERIAL.
# With DECOMPOSE_ANDROID_ONLY=1, repeats only the last measurement.
set -u
cd "${0:A:h}"
udid=$1
adb=${ANDROID_HOME:-$HOME/Library/Android/sdk}/platform-tools/adb
limit() { perl -e 'alarm shift; exec @ARGV' "$@"; }

if [[ -z ${DECOMPOSE_ANDROID_ONLY:-} ]]; then
(cd rust && cargo build --release --target aarch64-apple-ios-sim 2>&1 | tail -1)
limit 300 xcrun simctl spawn $udid "$PWD/rust/target/aarch64-apple-ios-sim/release/compare" > out/compono-ios.txt 2>&1
echo "compono ios: $?"
limit 700 xcrun simctl spawn $udid "$PWD/kotlin/kmp/build/bin/iosSimulatorArm64/releaseExecutable/kmp.kexe" > out/decompose-ios.txt 2>&1
echo "decompose ios: $?"

(cd rust && cargo ndk -t arm64-v8a -P 26 build --release 2>&1 | tail -1)
$adb push rust/target/aarch64-linux-android/release/compare /data/local/tmp/compono-compare > /dev/null
limit 300 $adb shell /data/local/tmp/compono-compare > out/compono-android.txt 2>&1
echo "compono android: $?"
$adb shell rm /data/local/tmp/compono-compare

fi

(cd kotlin && limit 300 $GRADLE --no-daemon -q --console=plain :android:assembleRelease 2>&1 | grep -E '^e: |FAILURE')
$adb install -r kotlin/android/build/outputs/apk/release/android-release.apk > /dev/null
$adb shell cmd package compile -m speed -f compono.compare > /dev/null
$adb logcat -c
$adb shell am force-stop compono.compare
$adb shell am start -W -n compono.compare/.MainActivity > /dev/null
$adb logcat -v raw -s ComponoCompare:I AndroidRuntime:E > out/decompose-android.log &
logcat=$!
# Stops on the last line, on a crash, or after twelve minutes.
for _ in {1..720}; do
  grep -qE '^# done|FATAL' out/decompose-android.log && break
  sleep 1
done
kill $logcat
grep -vE '^---------' out/decompose-android.log > out/decompose-android.txt
rm out/decompose-android.log
$adb shell am force-stop compono.compare
$adb uninstall compono.compare > /dev/null
echo "decompose android: done"
