#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# build_android.sh — сборка .so + APK для GL-режима (GameActivity)
#
# Пайплайн:
#   1. cargo ndk — сборка Rust как cdylib (.so) для arm64-v8a
#   2. ./gradlew — сборка APK с Kotlin + GameActivity AAR
#   3. adb install — опциональная установка
#
# Использование:
#   ./scripts/build_android.sh              # сборка + APK
#   ./scripts/build_android.sh --install    # сборка + APK + установка
#   ./scripts/build_android.sh --release    # release-сборка
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Параметры
INSTALL=false
BUILD_TYPE="debug"
CARGO_PROFILE="debug"
GRADLE_TASK="assembleDebug"
JNI_DIR="$PROJECT_ROOT/android/app/src/main/jniLibs/arm64-v8a"
LIB_NAME="libegui_android.so"

# Разбор аргументов
for arg in "$@"; do
    case "$arg" in
        --install)
            INSTALL=true
            ;;
        --release)
            BUILD_TYPE="release"
            CARGO_PROFILE="release"
            GRADLE_TASK="assembleRelease"
            ;;
    esac
done

echo "=== 1. Сборка Rust .so ($BUILD_TYPE) ==="
cd "$PROJECT_ROOT"

# Создаём каталог для .so
mkdir -p "$JNI_DIR"

# Сборка через cargo-ndk
# NOTE: нужно установить: cargo install cargo-ndk
if command -v cargo-ndk &>/dev/null; then
    if [ "$CARGO_PROFILE" = "release" ]; then
        cargo ndk -t arm64-v8a -o "$JNI_DIR" build --release -p egui-android
    else
        cargo ndk -t arm64-v8a -o "$JNI_DIR" build -p egui-android
    fi
    echo "  .so собран: $JNI_DIR/$LIB_NAME"
else
    echo "  [WARN] cargo-ndk не найден. Установи: cargo install cargo-ndk"
    echo "  Пробую прямой вызов cargo..."
    rustup target add aarch64-linux-android
    export CC_aarch64_linux_android="aarch64-linux-android21-clang"
    export AR_aarch64_linux_android="llvm-ar"
    if [ "$CARGO_PROFILE" = "release" ]; then
        cargo build --release --target aarch64-linux-android -p egui-android
    else
        cargo build --target aarch64-linux-android -p egui-android
    fi
    TARGET_DIR="$PROJECT_ROOT/target/aarch64-linux-android/$CARGO_PROFILE"
    cp "$TARGET_DIR/$LIB_NAME" "$JNI_DIR/"
    echo "  .so скопирован: $JNI_DIR/$LIB_NAME"
fi

echo ""
echo "=== 2. Сборка APK ($BUILD_TYPE) ==="
cd "$PROJECT_ROOT/android"

if [ ! -f "$PROJECT_ROOT/android/gradlew" ]; then
    echo "  Инициализация Gradle wrapper..."
    gradle wrapper --gradle-version 8.5
fi

./gradlew "$GRADLE_TASK"

APK_PATH="app/build/outputs/apk/$BUILD_TYPE/app-$BUILD_TYPE.apk"
echo ""
echo "=== APK готов: $APK_PATH ==="

if [ "$INSTALL" = true ]; then
    echo ""
    echo "=== 3. Установка на устройство ==="
    adb install -r "$APK_PATH"
    echo "  Установлено."
fi
