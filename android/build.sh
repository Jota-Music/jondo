#!/usr/bin/env bash
# Builds the Android APK: cargo-ndk compiles the library, Gradle packages it.
#
# Usage: android/build.sh [--release]
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

PROFILE=debug
CARGO_ARGS=()
GRADLE_TASK=assembleDebug
if [[ "${1:-}" == "--release" ]]; then
    PROFILE=release
    CARGO_ARGS+=(--release)
    GRADLE_TASK=assembleRelease
fi

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

echo "==> compiling the library (${PROFILE})"
cargo ndk -t arm64-v8a -t x86_64 -P 26 -o android/app/src/main/jniLibs build "${CARGO_ARGS[@]}"

echo "==> packaging ${VERSION}"
./android/gradlew -p android "$GRADLE_TASK" -Pversion="$VERSION"

echo "==> android/app/build/outputs/apk/${PROFILE}/"
