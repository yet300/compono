#!/bin/zsh
# Builds the Kotlin harness against each binding and runs both on a connected
# Android device or emulator. Run after `boltffi pack android --release` and the
# UniFFI steps in README.md.
set -eu
cd "${0:A:h}/.."
adb=${ANDROID_HOME:-$HOME/Library/Android/sdk}/platform-tools/adb
gradle=(${GRADLE:-gradle} --no-daemon)
app=harness/android/app/src/main
print "sdk.dir=${ANDROID_HOME:-$HOME/Library/Android/sdk}" > harness/android/local.properties

for backend in bolt uni; do
  rm -rf $app/java $app/jniLibs
  mkdir -p $app/java/compono/bench $app/jniLibs/arm64-v8a
  if [[ $backend == bolt ]]; then
    cp -R bolt/dist/android/kotlin/compono/bench/bolt $app/java/compono/bench/
    cp bolt/dist/android/jniLibs/arm64-v8a/*.so $app/jniLibs/arm64-v8a/
    bindings=compono.bench.bolt
    dirty=LongArray
  else
    mkdir -p $app/java/uniffi
    cp -R uni/dist/kotlin/uniffi/uni_bench $app/java/uniffi/
    cp target/aarch64-linux-android/release/libuni_bench.so $app/jniLibs/arm64-v8a/
    bindings=uniffi.uni_bench
    dirty='List<Long>'
  fi
  sed -e "s/@BINDINGS@/$bindings/" -e "s/@DIRTY@/$dirty/" harness/android/Bench.kt.template \
    > $app/java/compono/bench/Bench.kt
  (cd harness/android && $gradle -q --console=plain -Pbackend=$backend :app:assembleRelease)
  [[ -n ${BUILD_ONLY:-} ]] && continue
  $adb install -r harness/android/app/build/outputs/apk/release/app-release.apk > /dev/null
  $adb shell cmd package compile -m speed -f compono.bench > /dev/null
  $adb logcat -c
  $adb shell am force-stop compono.bench
  $adb shell am start -W -n compono.bench/.MainActivity > /dev/null
  print "== $backend android-emulator"
  log=out/android-$backend.log
  $adb logcat -v raw -s ComponoBench:I AndroidRuntime:E > $log &
  logcat=$!
  # Stops on the last line, on a crash, or after five minutes.
  for _ in {1..300}; do
    grep -qE '^# done|FATAL' $log && break
    sleep 1
  done
  kill $logcat
  cat $log
  $adb shell am force-stop compono.bench
done
$adb uninstall compono.bench > /dev/null
