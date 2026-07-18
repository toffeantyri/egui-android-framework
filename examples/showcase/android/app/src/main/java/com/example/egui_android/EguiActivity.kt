package com.example.egui_android

import android.os.Bundle
import androidx.activity.OnBackPressedCallback
import com.google.androidgamesdk.GameActivity

/**
 * Activity для GL-режима (GameActivity).
 *
 * Перехватывает системную кнопку Back — решение о завершении
 * принимается в Rust-коде (Application::on_back_pressed).
 * Без этого перехвата GameActivity сама завершает Activity
 * при получении системного Back (жест/кнопка).
 */
class EguiActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Перехватываем системный Back — не даём GameActivity
        // завершить Activity. Rust-код сам решит, когда выйти.
        onBackPressedDispatcher.addCallback(
            this,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    // Back будет обработан в Rust через input events
                    // (AKEYCODE_BACK → InputStatus::Handled).
                    // Если Rust решит завершить — он сам вызовет finish()
                    // через JNI или дождётся Destroy от системы.
                    // Главное — не вызывать super / finish() здесь.
                }
            }
        )
    }
}
