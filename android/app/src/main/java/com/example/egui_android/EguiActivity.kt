package com.example.egui_android

import android.os.Build
import android.os.Bundle
import android.view.View
import android.view.WindowInsets
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
    // Была ли IME видна на прошлой доставке инсетов. Для детекции перехода
    // «клавиатура была открыта → скрыта» при любом пути закрытия (системный Back,
    // Done, тап мимо, системный жест).
    private var imeVisible = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Android 16: обязательный edge-to-edge режим
        enableEdgeToEdge()
        setupImeVisibilityDetection()

        // Перехватываем системный Back — не даём GameActivity
        // завершить Activity. Rust-код сам решит, когда выйти.
        onBackPressedDispatcher.addCallback(
            this,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    // Системный Back: сначала закрываем клавиатуру (IME), если она
                    // открыта, и уведомляем Rust (nativeOnSystemBackPressed), чтобы тот
                    // сбросил владельца/фокус поля — иначе повторный тап не откроет
                    // клавиатуру заново. Навигацию оставляем на усмотрение Rust.
                    hideSoftInputForIme()
                    nativeOnSystemBackPressed()
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

    /**
     * Отслеживаем видимость IME по WindowInsets и уведомляем Rust, когда
     * клавиатура только что скрылась.
     *
     * Проблема: первый системный Back при открытой клавиатуре обрабатывает само
     * IME-окно, и до `onBackPressedDispatcher`/Rust он НЕ доходит. Значит сброс
     * владельца/фокуса поля нельзя вешать только на Back. Здесь ловим сам факт
     * скрытия IME по инсетам — он срабатывает при любом пути закрытия.
     */
    private fun setupImeVisibilityDetection() {
        val decor = window.decorView
        decor.setOnApplyWindowInsetsListener { _, insets ->
            val imeHeight = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                insets.getInsets(WindowInsets.Type.ime()).bottom
            } else {
                @Suppress("DEPRECATION")
                insets.systemWindowInsetBottom
            }
            val nowVisible = imeHeight > 0
            if (imeVisible && !nowVisible) {
                nativeOnSystemBackPressed()
            }
            imeVisible = nowVisible
            insets
        }
        // Гарантируем доставку текущих insets после attach окна.
        decor.requestApplyInsets()
    }

    /**
     * Создать (лениво) невидимый IME-View и добавить его поверх контента.
     * View не рисует ничего (`onDraw` пуст), поэтому не мешает GL-рендеру в Surface.
     */
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

    /**
     * Показать клавиатуру: дать фокус невидимому IME-View и вызвать
     * `showSoftInput`. Вызывается из Rust (через JNI) при активации TextEdit.
     */
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

    /**
     * Скрыть клавиатуру. Вызывается из Rust (через JNI) при завершении
     * редактирования (Done) или потере фокуса TextEdit.
     */
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

    /**
     * Обновить imeOptions (Next/Done) и inputType для текущего TextEdit.
     * Вызывается из Rust при смене активного поля.
     */
    fun setImeOptions(imeOptions: Int, inputType: Int) {
        imeView?.let { v ->
            v.post {
                // Сохраняем настройки в EguiImeView, чтобы onCreateInputConnection
                // (вызываемый restartInput) применил их, а не хардкод.
                v.applyOptions(inputType, imeOptions)
                val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
                imm.restartInput(v)
                android.util.Log.i(
                    "EguiActivity",
                    "setImeOptions: imeOptions=$imeOptions inputType=$inputType"
                )
            }
        }
    }

    /**
     * Передать позицию курсора в IME для позиционирования candidate window.
     * Вызывается из Rust каждый кадр, пока IME активна.
     *
     * Координаты (left/top/right/bottom) — пиксельные, для отладки.
     * Реальное позиционирование кандидат-окна — через `updateSelection`
     * с данными из `ImeEditorState` (selection + composing range).
     */
    fun updateCursorRect(left: Int, top: Int, right: Int, bottom: Int) {
        android.util.Log.i(
            "EguiActivity",
            "updateCursorRect: [$left, $top, $right, $bottom]"
        )
        imeView?.post {
            val view = imeView ?: return@post
            val selStart = view.getImeSelectionStart()
            val selEnd = view.getImeSelectionEnd()
            val compStart = view.getImeComposingStart()
            val compEnd = view.getImeComposingEnd()
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
            imm.updateSelection(view, selStart, selEnd, compStart, compEnd)
        }
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

    private external fun nativeGetSavedState(): ByteArray?
    private external fun nativeSetSavedState(bytes: ByteArray?)

    // Уведомляет Rust, что системный Back закрыл IME (сброс владельца/фокуса поля).
    private external fun nativeOnSystemBackPressed()

    companion object {
        private const val SAVED_STATE_KEY = "egui_saved_state"
    }
}
