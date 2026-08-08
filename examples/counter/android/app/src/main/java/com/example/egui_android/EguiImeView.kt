package com.example.egui_android

import android.content.Context
import android.graphics.Canvas
import android.text.InputType
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection

/**
 * Невидимый `View`, который существует только ради IME (InputConnection).
 *
 * GameActivity рендерит UI в Surface (egui/GL). Для работы нативной клавиатуры
 * с собственным вводом не используется штатный IME-протокол GameActivity
 * (`TextEvent`/`setTextInputState`). Вместо этого этот невидимый View:
 *
 *  1. При активации IME Android обращается к `onCreateInputConnection`.
 *  2. Мы возвращаем собственный `InputConnection`, который переопределяет
 *     `commitText`, `setComposingText`, `deleteSurroundingText`,
 *     `performEditorAction`, `finishComposingText`.
 *  3. Каждый такой метод вызывает JNI-функцию `nativeOn*...`, реализованную
 *     в Rust (`ime_jni.rs`). Rust кладёт команду в очередь `PlatformState.ime_cmds`,
 *     которую главный цикл преобразует в `egui::Event`.
 *
 * Никаких draw — View полностью невидим, рисует только рендер в Surface.
 */
class EguiImeView(context: Context) : View(context) {

    // Последние inputType/imeOptions, переданные из Rust через EguiActivity.setImeOptions.
    // Используются в onCreateInputConnection, чтобы после restartInput EditorInfo
    // отражал тип поля/действия текущего TextEdit (а не хардкод).
    private var currentInputType: Int = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
    private var currentImeOptions: Int = EditorInfo.IME_ACTION_NEXT or EditorInfo.IME_FLAG_NO_EXTRACT_UI

    // Фокусируемость в touch-режиме, чтобы IME мог активироваться по тапу.
    init {
        isFocusable = true
        isFocusableInTouchMode = true
    }

    /**
     * Обновить EditorInfo-настройки IME (переданы из Rust через JNI).
     */
    fun applyOptions(inputType: Int, imeOptions: Int) {
        currentInputType = inputType
        currentImeOptions = imeOptions
    }

    override fun onCheckIsTextEditor(): Boolean = true

    override fun onDraw(canvas: Canvas) {
        // Не рисуем ничего — невидимый слой для IME.
    }

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
        super.onCreateInputConnection(outAttrs)

        // Применяем inputType/imeOptions, переданные из Rust через setImeOptions.
        outAttrs.inputType = currentInputType
        outAttrs.imeOptions = currentImeOptions or EditorInfo.IME_FLAG_NO_EXTRACT_UI

        // Используем BaseInputConnection — он даёт корректное поведение для
        // редактирования (deleteSurroundingText, commitText) без собственной view.
        return EguiImeInputConnection(this)
    }

    /**
     * Собственный InputConnection, который пробрасывает IME-события в Rust JNI.
     */
    private inner class EguiImeInputConnection(targetView: View) :
        BaseInputConnection(targetView, true) {

        override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
            val t = text?.toString() ?: ""
            if (t.isNotEmpty()) {
                logIme("commitText", t)
                nativeOnCommitText(t, newCursorPosition)
            }
            // Не даём Android самому вставить текст — вставку делает egui из
            // события `Event::Text`, полученного из Rust после JNI.
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
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            logIme("deleteSurroundingText", "[$beforeLength, $afterLength]")
            nativeOnDeleteSurroundingText(beforeLength, afterLength)
            return true
        }

        // ─── Обратная связь: Rust владеет текстом поля, IME запрашивает его ───
        // Эти методы переопределяют BaseInputConnection, чтобы Kotlin возвращал
        // реальный текст/курсор фокусного поля, хранимый в Rust
        // (PlatformState.ime_editor_state), а не пустоту из невидимого View.

        override fun getTextBeforeCursor(length: Int, flags: Int): CharSequence? {
            return nativeGetTextBeforeCursor(length, flags)
        }

        override fun getTextAfterCursor(length: Int, flags: Int): CharSequence? {
            return nativeGetTextAfterCursor(length, flags)
        }

        override fun getSelectedText(flags: Int): CharSequence? {
            return nativeGetSelectedText(flags)
        }

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

        override fun getCursorCapsMode(reqModes: Int): Int {
            return nativeGetCursorCapsMode(reqModes)
        }

        override fun setSelection(start: Int, end: Int): Boolean {
            nativeSetSelection(start, end)
            return true
        }

        override fun setComposingRegion(start: Int, end: Int): Boolean {
            nativeSetComposingRegion(start, end)
            return true
        }

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

        override fun performPrivateCommand(action: String?, data: android.os.Bundle?): Boolean =
            super.performPrivateCommand(action, data)
    }

    private fun logIme(method: String, arg: String) {
        android.util.Log.i("EguiImeView", "$method: '$arg'")
    }

    // ─── JNI (реализованы в Rust: crates/platform-android/src/ime_jni.rs) ───

    private external fun nativeOnCommitText(text: String, newCursorPosition: Int)
    private external fun nativeOnComposingText(text: String, newCursorPosition: Int)
    private external fun nativeOnDeleteSurroundingText(beforeLength: Int, afterLength: Int)
    private external fun nativeOnImeActionNext()
    private external fun nativeOnImeActionDone()

    // Двусторонний InputConnection: чтение текста/курсора из Rust (editor_state).
    private external fun nativeGetTextBeforeCursor(length: Int, flags: Int): String?
    private external fun nativeGetTextAfterCursor(length: Int, flags: Int): String?
    private external fun nativeGetSelectedText(flags: Int): String?
    private external fun nativeGetFullText(): String?
    private external fun nativeGetCursorCapsMode(reqModes: Int): Int
    private external fun nativeGetSelectionStart(): Int
    private external fun nativeGetSelectionEnd(): Int
    private external fun nativeSetSelection(start: Int, end: Int)
    private external fun nativeSetComposingRegion(start: Int, end: Int)
}
