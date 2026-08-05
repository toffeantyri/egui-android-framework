package com.example.egui_android

import android.os.Bundle
import android.view.View
import android.view.inputmethod.InputMethodManager
import android.content.Context
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import com.google.androidgamesdk.GameActivity

/**
 * Activity для GL-режима (GameActivity).
 *
 * Перехватывает системную кнопку Back — решение о завершении
 * принимается в Rust-коде (Application::on_back_pressed).
 *
 * # IME через собственный невидимый View
 *
 * Рендер — в Surface (egui/GL). Поверх него существует невидимый
 * [`EguiImeView`], который был добавлен для получения IME-событий через
 * собственный `InputConnection`. Rust показывает/скрывает клавиатуру,
 * вызывая публичные методы `showSoftInputForIme()` / `hideSoftInputForIme()`
 * через JNI.
 *
 * # JNI-мост для kill/restore процесса
 *
 * `nativeGetSavedState()` и `nativeSetSavedState()` — JNI-функции,
 * реализованные в Rust (`saved_state_jni.rs`).
 * Они передают сериализованный `Vec<u8>` (SavedStack<C>) между
 * Rust PlatformState и Android Bundle.
 */
class EguiActivity : GameActivity() {

    // Невидимый View для InputConnection (IME). Создаётся лениво при первом показе.
    private var imeView: EguiImeView? = null

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
                    // Главное — не вызывать super / finish() здесь.
                }
            }
        )

        // Невидимый IME-View добавляем в иерархию сразу — он не рисует ничего.
        ensureImeView()

        // Восстанавливаем состояние после kill/restore
        val savedBytes = savedInstanceState?.getByteArray(SAVED_STATE_KEY)
        nativeSetSavedState(savedBytes)

        logSavedState("onCreate", savedBytes)
    }

    private fun ensureImeView(): EguiImeView {
        imeView?.let { return it }

        val view = EguiImeView(this)
        imeView = view

        // Невидимый IME-View добавляем через addContentView с ненулевым размером
        // (1px), чтобы Android точно attach-нул его в окно и мог создать
        // InputConnection при showSoftInput. onDraw пуст — ничего не рисуется.
        addContentView(view, android.view.ViewGroup.LayoutParams(1, 1))
        view.isFocusableInTouchMode = true
        return view
    }

    fun showSoftInputForIme() {
        val view = ensureImeView()
        android.util.Log.i("egui-showcase", "Kotlin: showSoftInputForIme begin")
        view.post {
            val okFocus = view.requestFocus()
            view.isFocusableInTouchMode = true
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
            val okShow = imm.showSoftInput(view, InputMethodManager.SHOW_IMPLICIT)
            android.util.Log.i(
                "egui-showcase",
                "Kotlin: showSoftInput end okFocus=$okFocus okShow=$okShow"
            )
        }
        android.util.Log.i("egui-showcase", "Kotlin: showSoftInputForIme scheduled")
    }

    fun hideSoftInputForIme() {
        val view = imeView
        if (view == null) {
            android.util.Log.i("EguiActivity", "hideSoftInputForIme: нет IME-View")
            return
        }
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        imm.hideSoftInputFromWindow(view.windowToken, 0)
        android.util.Log.i("EguiActivity", "hideSoftInputForIme: скрыта")
    }

    fun setImeOptions(imeOptions: Int, inputType: Int) {
        imeView?.let { v ->
            v.post {
                val attrs = android.view.inputmethod.EditorInfo()
                attrs.imeOptions = imeOptions
                attrs.inputType = inputType
                val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
                imm.restartInput(v)
                android.util.Log.i(
                    "EguiActivity",
                    "setImeOptions: imeOptions=$imeOptions inputType=$inputType"
                )
            }
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)

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

    private external fun nativeGetSavedState(): ByteArray?
    private external fun nativeSetSavedState(bytes: ByteArray?)

    companion object {
        private const val SAVED_STATE_KEY = "egui_saved_state"
    }
}
