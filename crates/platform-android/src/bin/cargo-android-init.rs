//! cargo-android-init — кроссплатформенный генератор Android-проекта.
//!
//! Скачивает android/ из репозитория egui-android-framework,
//! или копирует из локального пути к крейту (fallback для разработчика).
//!
//! Использование:
//!   cargo run --bin cargo-android-init
//!
//! Переменные окружения:
//!   APP_PACKAGE  — applicationId (по умолч. com.example.app)
//!   APP_LIB_NAME — имя .so (по умолч. app)
//!   APP_LABEL    — название приложения (по умолч. Egui App)

use std::fs;
use std::path::{Path, PathBuf};

const ANDROID_DIR_NAME: &str = "android";

/// URL для скачивания архива репозитория.
const REMOTE_URLS: &[&str] = &[
    "https://github.com/toffeantyri/egui-android-framework/archive/main.tar.gz",
    "https://gitverse.ru/Tofy3434/egui-android-framework/archive/main.tar.gz",
];

fn main() {
    let app_label = std::env::var("APP_LABEL").unwrap_or_else(|_| "Egui App".to_string());
    let app_lib_name = std::env::var("APP_LIB_NAME").unwrap_or_else(|_| "app".to_string());
    let app_package =
        std::env::var("APP_PACKAGE").unwrap_or_else(|_| "com.example.app".to_string());

    // Целевая директория — первый аргумент или CWD/android
    let target_dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(ANDROID_DIR_NAME));

    // Если android/ существует, но без gradlew — удаляем
    if target_dir.exists() && !target_dir.join("gradlew").exists() {
        eprintln!("android/ найден, но повреждён. Удаляем...");
        fs::remove_dir_all(&target_dir).ok();
    }

    if target_dir.exists() && target_dir.join("gradlew").exists() {
        eprintln!("android/ уже существует: {}", target_dir.display());
        // Всё равно генерируем манифест — он мог устареть
        generate_manifest(&target_dir, &app_label, &app_lib_name);
        return;
    }

    println!(
        "Создание android/ (label={app_label}, lib_name={app_lib_name}, package={app_package})"
    );

    // 1. Пытаемся скачать с удалённых репозиториев
    if let Ok(()) = download_android(&target_dir) {
        println!("  android/ — скачан с удалённого репозитория");
    }
    // 2. Fallback: ищем android/ рядом с крейтом (для разработчика)
    else if let Some(local_path) = find_local_android() {
        copy_android(&local_path, &target_dir);
        println!("  android/ — скопирован из {}", local_path.display());
    }
    // 3. Не удалось
    else {
        eprintln!("  Ошибка: не удалось найти android/.");
        eprintln!("  Убедитесь, что есть интернет, или скопируйте android/ вручную.");
        std::process::exit(1);
    }

    // Генерируем AndroidManifest.xml
    generate_manifest(&target_dir, &app_label, &app_lib_name);

    // Создаём local.properties из ANDROID_HOME/ANDROID_SDK_ROOT
    let sdk_path = std::env::var("ANDROID_HOME")
        .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
        .ok()
        .filter(|p| !p.is_empty())
        // Fallback: стандартные пути установки SDK
        .or_else(|| {
            let candidates = [
                "/usr/lib/android-sdk",
                "$HOME/Android/Sdk",
                "$HOME/.android/sdk",
            ];
            candidates
                .iter()
                .find(|p| {
                    let path = Path::new(p);
                    path.exists() && path.join("platforms").exists()
                })
                .map(|s| s.to_string())
        });

    if let Some(sdk) = sdk_path {
        // Разворачиваем $HOME вручную
        let sdk = sdk.replace("$HOME", &std::env::var("HOME").unwrap_or_default());
        let local_props = format!("sdk.dir={}\n", sdk);
        fs::write(target_dir.join("local.properties"), local_props).ok();
        println!("  local.properties — создан (sdk.dir={})", sdk);
    } else {
        println!("  local.properties — не создан (нет SDK)");
        println!("  Укажите ANDROID_HOME или ANDROID_SDK_ROOT");
    }

    println!("Готово.");
    println!("  APP_LIB_NAME={app_lib_name} APP_PACKAGE={app_package} ./gradlew assembleDebug");
}

// ─── Скачивание ────────────────────────────────────────────────

fn download_android(target: &Path) -> Result<(), ()> {
    // Создаём временный файл
    let tmp_path = Path::new("/tmp/egui-android-download.tar.gz");
    let _ = fs::remove_file(tmp_path);

    for url in REMOTE_URLS {
        println!("  Пробуем: {url}");
        let status = std::process::Command::new("curl")
            .args([
                "-sL",
                "--connect-timeout",
                "10",
                "-o",
                &tmp_path.to_string_lossy(),
                url,
            ])
            .status()
            .map_err(|_| ())?;

        if !status.success() {
            continue;
        }

        // Извлекаем android/ из архива
        let extract = std::process::Command::new("tar")
            .args([
                "-xzf",
                &tmp_path.to_string_lossy(),
                "--strip-components=1",
                "-C",
                &target.to_string_lossy(),
                "*/android/",
            ])
            .status()
            .map_err(|_| ())?;

        let _ = fs::remove_file(tmp_path);

        if extract.success() && target.join("gradlew").exists() {
            // Делаем gradlew исполняемым (Unix)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(target.join("gradlew"), fs::Permissions::from_mode(0o755)).ok();
            }
            return Ok(());
        }
    }

    let _ = fs::remove_file(tmp_path);
    Err(())
}

// ─── Локальный fallback ────────────────────────────────────────

/// Ищет android/ рядом с крейтом `egui-android-platform-android`.
fn find_local_android() -> Option<PathBuf> {
    // Пробуем найти через переменную CARGO_MANIFEST_DIR (доступна при cargo run)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = Path::new(&manifest_dir).join("../../android");
        if p.exists() && p.join("gradlew").exists() {
            return Some(p);
        }
    }

    // Пробуем найти относительно текущей директории
    let candidates = [
        Path::new("../android"),
        Path::new("../../android"),
        Path::new("../../../android"),
    ];
    for rel in &candidates {
        if rel.exists() && rel.join("gradlew").exists() {
            return Some(rel.to_path_buf());
        }
    }

    None
}

fn copy_android(src: &Path, dst: &Path) {
    fn copy_dir(src: &Path, dst: &Path) {
        fs::create_dir_all(dst).unwrap();
        for entry in fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let ty = entry.file_type().unwrap();
            let name = entry.file_name();
            let src_path = entry.path();
            let dst_path = dst.join(&name);

            if ty.is_dir() {
                copy_dir(&src_path, &dst_path);
            } else {
                fs::copy(&src_path, &dst_path).unwrap();
            }
        }
    }

    fs::create_dir_all(dst).unwrap();
    copy_dir(src, dst);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dst.join("gradlew"), fs::Permissions::from_mode(0o755)).ok();
    }
}

// ─── Генерация манифеста ───────────────────────────────────────

fn generate_manifest(target: &Path, label: &str, lib_name: &str) {
    let manifest_dir = target.join("app/src/main");
    fs::create_dir_all(&manifest_dir).unwrap();

    let manifest = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <application
        android:label="{label}"
        android:hasCode="true">

        <activity
            android:name="com.example.egui_android.EguiActivity"
            android:exported="true"
            android:label="{label}"
            android:theme="@style/AppTheme">
            <meta-data android:name="android.app.lib_name" android:value="{lib_name}" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>

</manifest>
"#
    );

    fs::write(manifest_dir.join("AndroidManifest.xml"), manifest).unwrap();
}
