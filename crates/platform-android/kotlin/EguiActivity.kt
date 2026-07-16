package com.example.egui_android_framework

import android.os.Bundle
import com.google.androidgamesdk.GameActivity

/**
 * Минимальная Activity для GL-режима.
 * Использует GameActivity из AGDK, который предоставляет:
 * - SurfaceView
 * - InputConnection (IME)
 * - WindowInsets
 * - DisplayMetrics
 *
 * Подключается к Rust через android-activity (game-activity feature).
 * lib_name указывается в AndroidManifest.xml каждого приложения.
 */
class EguiActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // SurfaceView создаётся автоматически GameActivity glue
    }
}
