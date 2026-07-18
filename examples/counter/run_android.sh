#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# run_android.sh — сборка + установка + логи для counter
#
# Использование:
#   cd examples/counter && ./run_android.sh
#   cd examples/counter && ./run_android.sh --release
#   cd examples/counter && ./run_android.sh --install  (только установка)
#   cd examples/counter && ./run_android.sh --run      (сборка + установка + логи)
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

CRATE="egui-android-counter"
LIB_NAME="lib${CRATE//-/_}.so"
LOG_TAG="egui-counter"
JNI_DIR="$PROJECT_ROOT/android/app/src/main/jniLibs"

CARGO_PROFILE="debug"
GRADLE_TASK="assembleDebug"
INSTALL=false
SHOW_LOGS=false

for arg in "$@"; do
    case "$arg" in
        --release)
            CARGO_PROFILE="release"
            GRADLE_TASK="assembleRelease"
            ;;
        --install) INSTALL=true ;;
        --log) SHOW_LOGS=true ;;
        --run)
            INSTALL=true
            SHOW_LOGS=true
            ;;
    esac
done

# Устанавливаем переменную для build.gradle
export APP_LIB_NAME="${CRATE//-/_}"

echo "=== 1/3: Сборка .so ($CARGO_PROFILE) ==="
cd "$PROJECT_ROOT"

# Чистим jniLibs перед сборкой — в APK попадёт только нужная .so
rm -rf "$JNI_DIR"
mkdir -p "$JNI_DIR"

ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-/usr/lib/android-sdk/ndk/27.3.13750724}" \
    cargo ndk -t arm64-v8a -o "$JNI_DIR" build $([ "$CARGO_PROFILE" = "release" ] && echo "--release") -p "$CRATE"
echo "  $JNI_DIR/arm64-v8a/$LIB_NAME"

echo ""
echo "=== 2/3: Сборка APK ==="

cd "$PROJECT_ROOT/android"
./gradlew "$GRADLE_TASK"

APK_PATH="app/build/outputs/apk/$CARGO_PROFILE/app-$CARGO_PROFILE.apk"
echo ""
echo "=== APK: $APK_PATH ==="

if [ "$INSTALL" = true ]; then
    echo ""
    echo "=== 3/3: Установка и запуск ==="
    adb install -r "$APK_PATH"
    echo "  Установлено."
    adb shell am start -n com.example.egui_android/.EguiActivity
fi

if [ "$SHOW_LOGS" = true ]; then
    echo ""
    echo "=== Логи (Ctrl+C для выхода) ==="
    adb logcat -v brief | grep "$LOG_TAG"
fi
