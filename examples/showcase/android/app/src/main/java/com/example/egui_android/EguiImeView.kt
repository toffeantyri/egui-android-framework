package com.example.egui_android

import android.content.Context
import android.graphics.Canvas
import android.text.InputType
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection

class EguiImeView(context: Context) : View(context) {

    private var currentInputType: Int = InputType.TYPE_CLASS_TEXT
    private var currentImeOptions: Int = EditorInfo.IME_ACTION_NEXT or EditorInfo.IME_FLAG_NO_EXTRACT_UI

    init {
        isFocusable = true
        isFocusableInTouchMode = true
    }

    fun applyOptions(inputType: Int, imeOptions: Int) {
        currentInputType = inputType
        currentImeOptions = imeOptions
    }

    override fun onCheckIsTextEditor(): Boolean = true
    override fun onDraw(canvas: Canvas) {}

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
        super.onCreateInputConnection(outAttrs)
        outAttrs.inputType = currentInputType
        outAttrs.imeOptions = currentImeOptions or EditorInfo.IME_FLAG_NO_EXTRACT_UI
        return EguiImeInputConnection(this)
    }

    private inner class EguiImeInputConnection(targetView: View) :
        BaseInputConnection(targetView, true) {

        override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
            val t = text?.toString() ?: ""
            if (t.isNotEmpty()) {
                logIme("commitText", t)
                nativeOnCommitText(t, newCursorPosition)
            }
            return true
        }

        override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
            val t = text?.toString() ?: ""
            logIme("setComposingText", t)
            nativeOnComposingText(t, newCursorPosition)
            return true
        }

        override fun finishComposingText(): Boolean {
            logIme("finishComposingText", "")
            nativeOnComposingText("", 0)
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            logIme("deleteSurroundingText", "[$beforeLength, $afterLength]")
            nativeOnDeleteSurroundingText(beforeLength, afterLength)
            return true
        }

        override fun sendKeyEvent(event: android.view.KeyEvent): Boolean {
            if (event.keyCode == android.view.KeyEvent.KEYCODE_DEL &&
                (event.action == android.view.KeyEvent.ACTION_DOWN ||
                    event.action == android.view.KeyEvent.ACTION_MULTIPLE)
            ) {
                logIme("sendKeyEvent", "DEL -> deleteSurroundingText(${event.repeatCount + 1},0)")
                deleteSurroundingText(event.repeatCount + 1, 0)
                return true
            }
            if (event.keyCode == android.view.KeyEvent.KEYCODE_FORWARD_DEL &&
                (event.action == android.view.KeyEvent.ACTION_DOWN ||
                    event.action == android.view.KeyEvent.ACTION_MULTIPLE)
            ) {
                logIme("sendKeyEvent", "FORWARD_DEL -> deleteSurroundingText(0,${event.repeatCount + 1})")
                deleteSurroundingText(0, event.repeatCount + 1)
                return true
            }
            return super.sendKeyEvent(event)
        }

        override fun deleteSurroundingTextInCodePoints(beforeLength: Int, afterLength: Int): Boolean {
            logIme("deleteSurroundingTextInCodePoints", "[$beforeLength, $afterLength]")
            nativeOnDeleteSurroundingText(beforeLength, afterLength)
            return true
        }

        override fun getTextBeforeCursor(length: Int, flags: Int): CharSequence? = nativeGetTextBeforeCursor(length, flags)
        override fun getTextAfterCursor(length: Int, flags: Int): CharSequence? = nativeGetTextAfterCursor(length, flags)
        override fun getSelectedText(flags: Int): CharSequence? = nativeGetSelectedText(flags)

        override fun getExtractedText(request: android.view.inputmethod.ExtractedTextRequest?, flags: Int): android.view.inputmethod.ExtractedText? {
            val text = nativeGetFullText() ?: return null
            val et = android.view.inputmethod.ExtractedText()
            et.text = text
            et.startOffset = 0
            et.selectionStart = nativeGetSelectionStart()
            et.selectionEnd = nativeGetSelectionEnd()
            et.flags = 0
            return et
        }

        override fun getCursorCapsMode(reqModes: Int): Int = nativeGetCursorCapsMode(reqModes)
        override fun setSelection(start: Int, end: Int): Boolean { nativeSetSelection(start, end); return true }
        override fun setComposingRegion(start: Int, end: Int): Boolean { nativeSetComposingRegion(start, end); return true }
        override fun beginBatchEdit(): Boolean { nativeBeginBatchEdit(); return true }
        override fun endBatchEdit(): Boolean { nativeEndBatchEdit(); return true }

        override fun performEditorAction(editorAction: Int): Boolean {
            logIme("performEditorAction", editorAction.toString())
            when (editorAction) {
                EditorInfo.IME_ACTION_NEXT -> nativeOnImeActionNext()
                EditorInfo.IME_ACTION_DONE,
                EditorInfo.IME_ACTION_SEARCH,
                EditorInfo.IME_ACTION_GO -> nativeOnImeActionDone()
                else -> nativeOnImeActionDone()
            }
            return true
        }

        override fun performPrivateCommand(action: String?, data: android.os.Bundle?): Boolean {
            logIme("performPrivateCommand", "action=$action")
            return false
        }
    }

    private fun logIme(method: String, arg: String) {
        android.util.Log.i("EguiImeView", "$method: '$arg' (thread=${Thread.currentThread().name})")
    }

    private external fun nativeOnCommitText(text: String, newCursorPosition: Int)
    private external fun nativeOnComposingText(text: String, newCursorPosition: Int)
    private external fun nativeOnDeleteSurroundingText(beforeLength: Int, afterLength: Int)
    private external fun nativeOnImeActionNext()
    private external fun nativeOnImeActionDone()

    private external fun nativeGetTextBeforeCursor(length: Int, flags: Int): String?
    private external fun nativeGetTextAfterCursor(length: Int, flags: Int): String?
    private external fun nativeGetSelectedText(flags: Int): String?
    private external fun nativeGetFullText(): String?
    private external fun nativeGetCursorCapsMode(reqModes: Int): Int
    private external fun nativeGetSelectionStart(): Int
    private external fun nativeGetSelectionEnd(): Int
    private external fun nativeSetSelection(start: Int, end: Int)
    private external fun nativeSetComposingRegion(start: Int, end: Int)
    private external fun nativeGetComposingStart(): Int
    private external fun nativeGetComposingEnd(): Int
    private external fun nativeBeginBatchEdit()
    private external fun nativeEndBatchEdit()

    fun getImeSelectionStart(): Int = nativeGetSelectionStart()
    fun getImeSelectionEnd(): Int = nativeGetSelectionEnd()
    fun getImeComposingStart(): Int = nativeGetComposingStart()
    fun getImeComposingEnd(): Int = nativeGetComposingEnd()
}
