package com.example.egui_android

import android.os.Bundle
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import com.google.androidgamesdk.GameActivity

/**
 * Activity для GL-режима (GameActivity).
 *
 * Перехватывает системную кнопку Back — решение о завершении
 * принимается в Rust-коде (Application::on_back_pressed).
 * Без этого перехвата GameActivity сама завершает Activity
 * при получении системного Back (жест/кнопка).
 *
 * # JNI-мост для kill/restore процесса
 *
 * `nativeGetSavedState()` и `nativeSetSavedState()` — JNI-функции,
 * реализованные в Rust (`saved_state_jni.rs`).
 * Они передают сериализованный `Vec<u8>` (SavedStack<C>) между
 * Rust PlatformState и Android Bundle.
 *
 * Поток данных:
 * ```
 * onSaveInstanceState:
 *   nativeGetSavedState() → ByteArray → Bundle.putByteArray()
 *
 * onCreate:
 *   Bundle.getByteArray() → nativeSetSavedState(bytes)
 * ```
 */
class EguiActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Android 16: обязательный edge-to-edge режим
        enableEdgeToEdge()

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

        // Восстанавливаем состояние после kill/restore
        // savedInstanceState содержит Bundle из предыдущего процесса
        val savedBytes = savedInstanceState?.getByteArray(SAVED_STATE_KEY)
        nativeSetSavedState(savedBytes)

        logSavedState("onCreate", savedBytes)
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)

        // Сохраняем состояние для следующего запуска после kill
        val bytes = nativeGetSavedState()
        if (bytes != null) {
            outState.putByteArray(SAVED_STATE_KEY, bytes)
        }

        logSavedState("onSaveInstanceState", bytes)
    }

    private fun logSavedState(method: String, bytes: ByteArray?) {
        if (bytes != null) {
            android.util.Log.i(
                "EguiActivity",
                "$method: saved_state = ${bytes.size} байт"
            )
        } else {
            android.util.Log.i("EguiActivity", "$method: saved_state = null")
        }
    }

    // ─── JNI-методы (реализованы в Rust) ─────────────────────────

    /**
     * Получить сериализованное состояние из Rust PlatformState.
     * Возвращает null, если буфер пуст (состояние не сохранялось).
     */
    private external fun nativeGetSavedState(): ByteArray?

    /**
     * Передать сериализованное состояние в Rust PlatformState.
     * bytes — данные из Bundle (null при первом запуске).
     */
    private external fun nativeSetSavedState(bytes: ByteArray?)

    companion object {
        private const val SAVED_STATE_KEY = "egui_saved_state"
    }
}
