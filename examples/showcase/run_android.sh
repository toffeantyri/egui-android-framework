#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# run_android.sh — сборка + установка + логи
#
# Скопируйте этот скрипт в корень своего Rust-проекта.
# Заполните настройки ниже — и запускайте.
#
# Скрипт:
#   1. Скачивает android/ (Gradle-проект) — автоматически
#   2. Собирает .so через cargo ndk
#   3. Собирает APK через ./gradlew
#   4. Устанавливает и запускает на устройстве
#
# Использование:
#   ./run_android.sh                  # сборка
#   ./run_android.sh --run           # сборка + установка + логи
#   ./run_android.sh --release       # release-сборка
# ============================================================

# ═══════════════════════════════════════════════════════════════
# >>> НАСТРОЙКИ ПРИЛОЖЕНИЯ <<<
# ═══════════════════════════════════════════════════════════════

APP_LABEL="Showcase"                    # Название (в списке приложений)
APP_PACKAGE="com.example.egui_showcase" # applicationId
APP_LIB_NAME="egui_showcase"            # Имя .so (lib${APP_LIB_NAME}.so)
CRATE="egui-showcase"                   # Крейт с cdylib для cargo ndk
LOG_TAG="egui-showcase"                 # Фильтр логов (adb logcat | grep)

# Путь к workspace Cargo.toml (если скрипт в корне проекта — не трогайте)
PROJECT_CARGO="Cargo.toml"

# ═══════════════════════════════════════════════════════════════
# >>> НАСТРОЙКИ СБОРКИ <<<
# ═══════════════════════════════════════════════════════════════

# Путь к NDK (можно через ANDROID_NDK_HOME)
NDK_PATH="${ANDROID_NDK_HOME:-/usr/lib/android-sdk/ndk/27.3.13750724}"

# Целевая архитектура (arm64-v8a, armeabi-v7a, x86_64)
TARGET="arm64-v8a"

# ═══════════════════════════════════════════════════════════════
# >>> ДАЛЬШЕ МЕНЯТЬ НЕ НУЖНО <<<
# ═══════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ANDROID_DIR="$SCRIPT_DIR/android"
JNI_DIR="$ANDROID_DIR/app/src/main/jniLibs"
LIB_NAME="lib${APP_LIB_NAME}.so"

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

# ─── 1. Подготовка android/ ──────────────────────────────────
if [ ! -d "$ANDROID_DIR" ] || [ ! -f "$ANDROID_DIR/gradlew" ]; then
    echo "=== 1/4: Подготовка android/ ==="
    cd "$SCRIPT_DIR"
    APP_LABEL="$APP_LABEL" APP_LIB_NAME="$APP_LIB_NAME" APP_PACKAGE="$APP_PACKAGE" \
        cargo run --manifest-path "$PROJECT_CARGO" --bin cargo-android-init -- "$ANDROID_DIR" 2>/dev/null \
        || cargo run --manifest-path "../$PROJECT_CARGO" --bin cargo-android-init -- "$ANDROID_DIR" 2>/dev/null \
        || cargo run --manifest-path "../../$PROJECT_CARGO" --bin cargo-android-init -- "$ANDROID_DIR"
    echo ""
fi

# Всегда генерируем AndroidManifest.xml (может устареть после cargo-android-init)
mkdir -p "$ANDROID_DIR/app/src/main"
cat > "$ANDROID_DIR/app/src/main/AndroidManifest.xml" << EOF
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <application
        android:label="${APP_LABEL}"
        android:hasCode="true">

        <activity
            android:name="com.example.egui_android.EguiActivity"
            android:exported="true"
            android:label="${APP_LABEL}"
            android:theme="@style/AppTheme">
            <meta-data android:name="android.app.lib_name" android:value="${APP_LIB_NAME}" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>

</manifest>
EOF

# ─── 2. Сборка .so ───────────────────────────────────────────
echo "=== 2/4: Сборка .so ($CARGO_PROFILE) ==="
cd "$SCRIPT_DIR"
rm -rf "$JNI_DIR"
mkdir -p "$JNI_DIR"

ANDROID_NDK_HOME="$NDK_PATH" \
    cargo ndk -t "$TARGET" -o "$JNI_DIR" build $([ "$CARGO_PROFILE" = "release" ] && echo "--release") -p "$CRATE"
echo "  $JNI_DIR/$TARGET/$LIB_NAME"

echo ""
echo "=== 3/4: Сборка APK ==="
cd "$ANDROID_DIR"
APP_LIB_NAME="$APP_LIB_NAME" APP_PACKAGE="$APP_PACKAGE" ./gradlew "$GRADLE_TASK"

APK_PATH="app/build/outputs/apk/$CARGO_PROFILE/app-$CARGO_PROFILE.apk"
echo ""
echo "=== APK: $APK_PATH ==="

if [ "$INSTALL" = true ]; then
    echo ""
    echo "=== 4/4: Установка и запуск ==="
    adb install -r "$APK_PATH"
    echo "  Установлено."
    adb shell am start -n "$APP_PACKAGE/com.example.egui_android.EguiActivity"
fi

if [ "$SHOW_LOGS" = true ]; then
    echo ""
    echo "=== Логи (Ctrl+C для выхода) ==="
    echo "    Фильтр: $LOG_TAG"
    adb logcat -v brief | grep "$LOG_TAG"
fi
