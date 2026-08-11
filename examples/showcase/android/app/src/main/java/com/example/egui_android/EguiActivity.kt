package com.example.egui_android

import android.os.Build
import android.os.Bundle
import android.view.WindowInsets
import android.view.inputmethod.InputMethodManager
import android.content.Context
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import com.google.androidgamesdk.GameActivity

class EguiActivity : GameActivity() {

    private var imeView: EguiImeView? = null
    // Была ли IME видна на прошлой доставке инсетов. Служит для детекции перехода
    // «клавиатура была открыта → скрыта» независимо от того, кто её скрыл
    // (системный Back, Done, тап мимо, системный жест).
    private var imeVisible = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setupImeVisibilityDetection()
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                // Системный Back: сначала закрываем клавиатуру (IME), если она
                // открыта, и уведомляем Rust (nativeOnSystemBackPressed), чтобы тот
                // сбросил владельца/фокус поля — иначе повторный тап не откроет
                // клавиатуру заново. Навигацию оставляем на усмотрение Rust.
                hideSoftInputForIme()
                nativeOnSystemBackPressed()
            }
        })
        ensureImeView()
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

    private fun ensureImeView(): EguiImeView {
        imeView?.let { return it }
        val view = EguiImeView(this)
        imeView = view
        addContentView(view, android.view.ViewGroup.LayoutParams(1, 1))
        view.isFocusableInTouchMode = true
        return view
    }

    fun showSoftInputForIme() {
        val view = ensureImeView()
        view.post {
            val okFocus = view.requestFocus()
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
            val okShow = imm.showSoftInput(view, InputMethodManager.SHOW_IMPLICIT)
            android.util.Log.i("egui-showcase", "Kotlin: showSoftInput end okFocus=$okFocus okShow=$okShow")
        }
    }

    fun hideSoftInputForIme() {
        val view = imeView ?: return
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        imm.hideSoftInputFromWindow(view.windowToken, 0)
    }

    fun setImeOptions(imeOptions: Int, inputType: Int) {
        imeView?.let { v ->
            v.post {
                v.applyOptions(inputType, imeOptions)
                val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
                imm.restartInput(v)
            }
        }
    }

    fun updateCursorRect(left: Int, top: Int, right: Int, bottom: Int) {
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
        val bytes = nativeGetSavedState()
        if (bytes != null) outState.putByteArray(SAVED_STATE_KEY, bytes)
    }

    private fun logSavedState(method: String, bytes: ByteArray?) {
        android.util.Log.i("EguiActivity", "$method: saved_state = ${bytes?.size ?: "null"} байт")
    }

    private external fun nativeGetSavedState(): ByteArray?
    private external fun nativeSetSavedState(bytes: ByteArray?)

    // Уведомляет Rust, что системный Back закрыл IME (сброс владельца/фокуса поля).
    private external fun nativeOnSystemBackPressed()

    companion object {
        private const val SAVED_STATE_KEY = "egui_saved_state"
    }
}
