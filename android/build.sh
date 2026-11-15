#!/usr/bin/env bash
# Builds the Android APKs: cargo-ndk compiles the library, Gradle packages it.
#
# Usage: android/build.sh [--release] [--quick] [--abi <abi>]
#
#   --release   release build (the default is a debug build)
#   --quick     build the library with the `quick` profile: the release
#               optimizations without the fat LTO, so the link is minutes
#               instead of quarters. The APK type still follows --release.
#   --abi       one ABI to build, e.g. arm64-v8a. The default builds both
#               arm64-v8a and x86_64, which is what the release publishes.
#
# One package per ABI comes out: a universal one would carry the libraries of
# every architecture, so it downloads half of them unused.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

PROFILE=debug
CARGO_PROFILE=debug
GRADLE_TASK=assembleDebug
ABIS=(arm64-v8a x86_64)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)
            PROFILE=release
            CARGO_PROFILE=release
            GRADLE_TASK=assembleRelease
            shift
            ;;
        --quick)
            CARGO_PROFILE=quick
            shift
            ;;
        --abi)
            ABIS=("${2:?--abi needs a name}")
            shift 2
            ;;
        *)
            echo "unknown option: $1" >&2
            exit 1
            ;;
    esac
done

SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
export ANDROID_HOME="$SDK"
export ANDROID_SDK_ROOT="$SDK"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$(ls -d "$SDK"/ndk/* | sort -V | tail -1)}"

# Gradle rejects JDK 27; pick an installed 21 or 17 when the caller did not.
if [[ -z "${JAVA_HOME:-}" ]]; then
    for candidate in "$HOME"/.jdks/jdk-21* "$HOME"/.jdks/jdk-17*; do
        [[ -d "$candidate" ]] && export JAVA_HOME="$candidate" && break
    done
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cache/jondo-android}"
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"

# A library left in jniLibs from an earlier build still lands in the APK, so
# the ABIs this build does not want have to go before Gradle packages it.
JNI_LIBS=android/app/src/main/jniLibs
mkdir -p "$JNI_LIBS"
for dir in "$JNI_LIBS"/*/; do
    [[ -d "$dir" ]] || continue
    abi="$(basename "$dir")"
    if [[ " ${ABIS[*]} " != *" $abi "* ]]; then
        echo "==> dropping stale $abi library"
        rm -rf "$dir"
    fi
done

# cargo-ndk only copies the library when the whole ABI directory is gone, so
# a build with another profile would keep the previous one and the APK would
# ship it. Taking the directory away first makes the copy unconditional.
for abi in "${ABIS[@]}"; do
    rm -rf "$JNI_LIBS/$abi"
done

CARGO_ARGS=(build "--profile=$CARGO_PROFILE")
if [[ "$CARGO_PROFILE" == "quick" ]]; then
    # The quick profile keeps the symbols for the desktop smoke test; an APK
    # that size is not worth installing.
    CARGO_ARGS+=(--config 'profile.quick.strip="symbols"')
fi

NDK_ARGS=()
for abi in "${ABIS[@]}"; do
    NDK_ARGS+=(-t "$abi")
done

echo "==> compiling the library (${CARGO_PROFILE}, ${ABIS[*]})"
cargo ndk "${NDK_ARGS[@]}" -P 26 -o "$JNI_LIBS" "${CARGO_ARGS[@]}"

echo "==> packaging ${VERSION}"
./android/gradlew -p android "$GRADLE_TASK" -Pversion="$VERSION" -Pabis="$(IFS=,; echo "${ABIS[*]}")"

APK_DIR="android/app/build/outputs/apk/${PROFILE}"
for abi in "${ABIS[@]}"; do
    cp "${APK_DIR}/app-${abi}-${PROFILE}.apk" "${APK_DIR}/jondo-${abi}.apk"
done
echo "==> ${APK_DIR}/jondo-*.apk"
