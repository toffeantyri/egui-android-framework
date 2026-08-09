package com.example.egui_android

import android.os.Bundle
import android.view.inputmethod.InputMethodManager
import android.content.Context
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import com.google.androidgamesdk.GameActivity

class EguiActivity : GameActivity() {

    private var imeView: EguiImeView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {}
        })
        ensureImeView()
        val savedBytes = savedInstanceState?.getByteArray(SAVED_STATE_KEY)
        nativeSetSavedState(savedBytes)
        logSavedState("onCreate", savedBytes)
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

    companion object {
        private const val SAVED_STATE_KEY = "egui_saved_state"
    }
}
