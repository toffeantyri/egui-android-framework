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

    // Фокусируемость в touch-режиме, чтобы IME мог активироваться по тапу.
    init {
        isFocusable = true
        isFocusableInTouchMode = true
    }

    override fun onCheckIsTextEditor(): Boolean = true

    override fun onDraw(canvas: Canvas) {
        // Не рисуем ничего — невидимый слой для IME.
    }

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
        super.onCreateInputConnection(outAttrs)

        // Базовые настройки: обычный текст (однострочный), без extract UI.
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
        outAttrs.imeOptions = outAttrs.imeOptions or EditorInfo.IME_ACTION_NEXT

        // Запрещаем fullscreen-extract (всплывающее окно IME поверх Surface не нужно).
        outAttrs.imeOptions = outAttrs.imeOptions or EditorInfo.IME_FLAG_NO_EXTRACT_UI

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
}
