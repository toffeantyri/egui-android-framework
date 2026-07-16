package com.example.egui_android

import android.os.Bundle
import com.google.androidgamesdk.GameActivity

/**
 * Минимальная Activity для GL-режима.
 *
 * Наследует GameActivity из Android Game Development Kit.
 * GameActivity предоставляет:
 * - SurfaceView для EGL-рендеринга
 * - InputConnection для IME (клавиатурный ввод без JNI)
 * - WindowInsets API
 * - DisplayMetrics для DPI
 *
 * Rust-код подключается через android-activity (feature "game-activity").
 * Имя .so библиотеки указывается в AndroidManifest.xml через meta-data
 * android.app.lib_name.
 */
class EguiActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // SurfaceView создаётся автоматически GameActivity glue
        // Весь жизненный цикл и рендеринг управляются из Rust через android_main()
    }
}
