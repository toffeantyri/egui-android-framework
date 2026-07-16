#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# build_android.sh — сборка .so + APK одной командой
#
# Использование:
#   ./scripts/build_android.sh                           # сборка + APK
#   ./scripts/build_android.sh --install                 # сборка + APK + установка
#   ./scripts/build_android.sh --install --log           # установка + логи в реальном времени
#   ./scripts/build_android.sh --run                     # сборка + установка + логи (как x run)
#   ./scripts/build_android.sh --release                 # release-сборка
#
# Зависимости:
#   - cargo-ndk (cargo install cargo-ndk)
#   - Android SDK + NDK + Gradle
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Параметры
INSTALL=false
SHOW_LOGS=false
CARGO_PROFILE="debug"
GRADLE_TASK="assembleDebug"
# Крейт с android_main (cdylib)
CRATE="egui-gl-app"
# Куда класть .so
JNI_DIR="$PROJECT_ROOT/android/app/src/main/jniLibs/arm64-v8a"
LIB_NAME="lib${CRATE//-/_}.so"
# NDK
NDK_HOME="${ANDROID_NDK_HOME:-/usr/lib/android-sdk/ndk/27.3.13750724}"
# Фильтр логов egui-gl-app
LOG_TAG="egui-gl-app"

# Разбор аргументов
for arg in "$@"; do
    case "$arg" in
        --install) INSTALL=true ;;
        --log) SHOW_LOGS=true ;;
        --run)
            INSTALL=true
            SHOW_LOGS=true
            ;;
        --release)
            CARGO_PROFILE="release"
            GRADLE_TASK="assembleRelease"
            ;;
    esac
done

echo "=== 1/3: Сборка Rust .so ($CARGO_PROFILE) ==="
cd "$PROJECT_ROOT"
mkdir -p "$JNI_DIR"
ANDROID_NDK_HOME="$NDK_HOME" \
    cargo ndk -t arm64-v8a -o "$JNI_DIR" build $([ "$CARGO_PROFILE" = "release" ] && echo "--release") -p "$CRATE"
echo "  $JNI_DIR/$LIB_NAME"

echo ""
echo "=== 2/3: Сборка APK ($CARGO_PROFILE) ==="
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
    echo "=== Логи в реальном времени (Ctrl+C для выхода) ==="
    echo "    Фильтр: $LOG_TAG"
    echo ""
    adb logcat -v brief | grep "$LOG_TAG"
fi
