#!/bin/zsh
# Runs Compono natively and Decompose on the JVM and Kotlin/Native on this machine.
# Needs GRADLE, and writes to out/. Each run is limited so a hang cannot go unnoticed.
set -u
cd "${0:A:h}"
limit() { perl -e 'alarm shift; exec @ARGV' "$@"; }
(cd rust && cargo build --release 2>&1 | tail -1)
limit 300 rust/target/release/compare > out/compono-host.txt; echo "compono host: $?"
cd kotlin
limit 500 $GRADLE --no-daemon -q --console=plain :kmp:linkReleaseExecutableMacosArm64 :kmp:linkReleaseExecutableIosSimulatorArm64 :kmp:jvmJar
echo "kotlin build: $?"
limit 600 $GRADLE --no-daemon -q --console=plain :kmp:jvmRun -DmainClass=compono.compare.MainKt > ../out/decompose-jvm.txt 2>&1; echo "decompose jvm: $?"
limit 600 kmp/build/bin/macosArm64/releaseExecutable/kmp.kexe > ../out/decompose-macos.txt 2>&1; echo "decompose macos: $?"
