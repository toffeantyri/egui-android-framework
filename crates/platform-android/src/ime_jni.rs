//! JNI-мост InputConnection → IME-команды → главный цикл.
//!
//! Kotlin-класс `EguiImeView` (невидимый `View` поверх GameActivity) реализует
//! собственный `InputConnection` и вызывает эти JNI-функции при поступлении
//! IME-событий: `commitText`, `setComposingText`, `performEditorAction`,
//! `deleteSurroundingText`.
//!
//! Каждая функция кладёт [`ImeCmd`](crate::event::ImeCmd) в потокобезопасную
//! очередь `PlatformState.ime_cmds`. Главный цикл (`loop.rs`) забирает очередь
//! каждый кадр и конвертирует команды в `egui::Event`.
//!
//! Эти JNI-функции **не используют** `InputEvent::TextEvent`, `setTextInputState`
//! или `textInputState()` — весь ввод идёт только через InputConnection → JNI.

#![cfg(target_os = "android")]

use crate::event::ImeCmd;
use crate::saved_state_jni::GLOBAL_PLATFORM_STATE;
use jni::objects::JClass;
use jni::sys::jint;
use jni::JNIEnv;

/// Общий обработчик: положить команду в `PlatformState.ime_cmds`.
///
/// JNI-функции вызываются на главном Java-потоке. Если `PlatformState` ещё
/// не инициализирован (цикл не запущен) — команду просто игнорируем.
fn push_cmd(cmd: ImeCmd) {
    log::info!("IME-JNI: push_cmd enter (thread={})", thread_name());
    match GLOBAL_PLATFORM_STATE.get() {
        Some(state) => state.push_ime_cmd(cmd),
        None => log::warn!(
            "IME-JNI: PlatformState не инициализирован — команда отброшена: {:?}",
            cmd
        ),
    }
    log::info!("IME-JNI: push_cmd exit (thread={})", thread_name());
}

/// Прочитать `JNIString` из JNIEnv (JString → String), безопасно.
///
/// JNI-строки в Android — UTF-8 модифицированная. `get_string` у JNI String
/// в jni 0.21 выдаст `JNIString`, которую конвертируем в обычный String
/// через `to_string_lossy`.
fn jstring_to_string<'local>(
    env: &mut JNIEnv<'local>,
    jstr: jni::objects::JString<'local>,
) -> String {
    match env.get_string(&jstr) {
        Ok(jni_string) => jni_string.to_string_lossy().into_owned(),
        Err(e) => {
            log::error!("IME-JNI: не удалось прочитать JString: {:?}", e);
            String::new()
        }
    }
}

/// `commitText(CharSequence text, int newCursorPosition)` → `ImeCmd::Commit(text)`.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeOnCommitText<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    text: jni::objects::JString<'a>,
    _new_cursor_position: i32,
) {
    let text = jstring_to_string(&mut env, text);
    if text.is_empty() {
        return;
    }
    log::info!("IME-JNI: commitText -> {:?}", text);
    push_cmd(ImeCmd::Commit(text));
}

/// `setComposingText(CharSequence text, int newCursorPosition)` → `ImeCmd::Composing(text)`.
///
/// `new_cursor_position` — позиция курсора (1 = в конце). Диапазон composition
/// берётся из `EditorInfo`/строки на стороне Kotlin; здесь передаём только текст.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeOnComposingText<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    text: jni::objects::JString<'a>,
    _new_cursor_position: i32,
) {
    let text = jstring_to_string(&mut env, text);
    log::info!("IME-JNI: setComposingText -> {:?}", text);
    push_cmd(ImeCmd::Composing(text));
}

/// `performEditorAction(EditorInfo.IME_ACTION_NEXT)` → `ImeCmd::Next`.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeOnImeActionNext(
    _env: JNIEnv,
    _class: JClass,
) {
    log::info!("IME-JNI: IME_ACTION_NEXT");
    push_cmd(ImeCmd::Next);
}

/// `performEditorAction(EditorInfo.IME_ACTION_DONE / SEARCH / GO)` → `ImeCmd::Done`.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeOnImeActionDone(
    _env: JNIEnv,
    _class: JClass,
) {
    log::info!("IME-JNI: IME_ACTION_DONE");
    push_cmd(ImeCmd::Done);
}

/// `deleteSurroundingText(int beforeLength, int afterLength)` → `ImeCmd::DeleteSurrounding`.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeOnDeleteSurroundingText(
    _env: JNIEnv,
    _class: JClass,
    before: i32,
    after: i32,
) {
    log::info!(
        "IME-JNI: deleteSurroundingText before={} after={}",
        before,
        after
    );
    push_cmd(ImeCmd::DeleteSurrounding { before, after });
}

/// `setComposingText(text)` с позицией (start, end) — расширенный вариант
/// для поддержки диапазона composition при preedit.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeOnComposingRange<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    text: jni::objects::JString<'a>,
    start: i32,
    end: i32,
) {
    let text = jstring_to_string(&mut env, text);
    log::info!(
        "IME-JNI: setComposingText range {:?} ({start}..{end})",
        text
    );
    push_cmd(ImeCmd::ComposingRange { text, start, end });
}

/// `performPrivateCommand(action, data)` — приватная команда IME (Gboard, Samsung и т.д.).
///
/// Не обрабатывается функционально (egui не имеет API для private-команд), но
/// передаётся в Rust через `ImeCmd::PrivateCommand`, где логируется для
/// диагностики проблем с клавиатурой (см. ветку в `process_ime_cmd`).
/// При необходимости в будущем здесь можно добавить обработку конкретных
/// команд (например, emoji-panel от Gboard).
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeOnPrivateCommand<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    action: jni::objects::JString<'a>,
) {
    let action = jstring_to_string(&mut env, action);
    log::info!("IME-JNI: performPrivateCommand action={:?}", action);
    push_cmd(ImeCmd::PrivateCommand(action));
}

// ─── Что читают JNI-функции: слот состояния редактирования ────────────────
//
// ui-слой (`TextEdit`) пишет `ImeEditorState` (текст + курсор в UTF-16) в
// `PlatformState.ime_editor_state` каждый кадр, пока поле в фокусе.
// Kotlin `EguiImeView.EguiImeInputConnection` запрашивает эти данные через
// JNI для полноценного `InputConnection` (getTextBeforeCursor и т.п.).

/// Текущее состояние редактирования активного поля (копия).
fn current_editor_state() -> Option<egui_android_runtime::ImeEditorState> {
    log::info!(
        "IME-EDIT: current_editor_state enter (thread={})",
        thread_name()
    );
    let r = GLOBAL_PLATFORM_STATE
        .get()
        .and_then(|ps| ps.ime_editor_state());
    log::info!("IME-EDIT: current_editor_state exit -> {}", r.is_some());
    r
}

/// Имя текущего потока (для диагностики deadlock между Rust-циклом и JNI).
fn thread_name() -> String {
    std::thread::current()
        .name()
        .map(|s| s.to_owned())
        .unwrap_or_else(|| "<no-name>".to_owned())
}

/// Лог входа в JNI-функцию с именем потока.
fn thread_id_log(fn_name: &str) {
    log::info!("IME-JNI: {} enter (thread={})", fn_name, thread_name());
}

/// Вырезать UTF-16-подстроку из текста в диапазоне [start, end).
fn utf16_substr(text: &str, start: usize, end: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let start = start.min(units.len());
    let end = end.min(units.len()).max(start);
    String::from_utf16_lossy(&units[start..end])
}

/// `getTextBeforeCursor(n, flags)` — текст из `n` UTF-16 units до курсора.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetTextBeforeCursor<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    length: i32,
    _flags: i32,
) -> jni::objects::JString<'a> {
    thread_id_log("nativeGetTextBeforeCursor");
    let s = current_editor_state();
    let out = match s {
        Some(st) => {
            let len = length as usize;
            let from = st.selection_start.saturating_sub(len);
            utf16_substr(&st.text, from, st.selection_start)
        }
        None => String::new(),
    };
    string_to_jstring(&mut env, &out)
}

/// `getTextAfterCursor(n, flags)` — текст из `n` UTF-16 units после курсора.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetTextAfterCursor<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    length: i32,
    _flags: i32,
) -> jni::objects::JString<'a> {
    let s = current_editor_state();
    let out = match s {
        Some(st) => {
            let len = length as usize;
            utf16_substr(
                &st.text,
                st.selection_end,
                st.selection_end.saturating_add(len),
            )
        }
        None => String::new(),
    };
    string_to_jstring(&mut env, &out)
}

/// `getSelectedText(flags)` — выделенный текст поля.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetSelectedText<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    _flags: i32,
) -> jni::objects::JString<'a> {
    let s = current_editor_state();
    let out = match s {
        Some(st) => utf16_substr(&st.text, st.selection_start, st.selection_end),
        None => String::new(),
    };
    string_to_jstring(&mut env, &out)
}

/// `getTextLength()` — длина текста в UTF-16 units.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetTextLength(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    current_editor_state().map_or(0, |st| st.text_len as jint)
}

/// Преобразовать Rust String в JNI строку.
fn string_to_jstring<'a>(env: &mut JNIEnv<'a>, s: &str) -> jni::objects::JString<'a> {
    match env.new_string(s) {
        Ok(js) => js,
        Err(e) => {
            log::error!("IME-JNI: не удалось создать JString: {:?}", e);
            jni::objects::JString::from(jni::objects::JObject::null())
        }
    }
}

// ─── Ещё read-функции InputConnection + setSelection ─────────────────────

/// `getFullText()` — весь текст активного поля (для `getExtractedText`).
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetFullText<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
) -> jni::objects::JString<'a> {
    let out = current_editor_state().map_or(String::new(), |st| st.text.clone());
    string_to_jstring(&mut env, &out)
}

/// `getCursorCapsMode(reqModes)` — всегда 0 (поле не знает о регистре; IME
/// сам решает по inputType).
/// `getCursorCapsMode(reqModes)` — возвращает битовую маску режимов
/// капитализации: после точки/!/? — CAP_MODE_SENTENCES, после пробела —
/// CAP_MODE_WORDS, в начале — все три.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetCursorCapsMode(
    _env: JNIEnv,
    _class: JClass,
    _req_modes: i32,
) -> jint {
    /// `CAP_MODE_CHARACTERS` — следующий символ должен быть заглавным.
    const CAP_MODE_CHARACTERS: i32 = 0x0001;
    /// `CAP_MODE_WORDS` — начало нового слова.
    const CAP_MODE_WORDS: i32 = 0x0002;
    /// `CAP_MODE_SENTENCES` — начало нового предложения.
    const CAP_MODE_SENTENCES: i32 = 0x0004;

    let state = current_editor_state();
    let Some(st) = state else {
        return 0;
    };

    if st.selection_start == 0 {
        return CAP_MODE_CHARACTERS | CAP_MODE_WORDS | CAP_MODE_SENTENCES;
    }

    // Берём символ перед курсором (по UTF-16 индексу).
    let before = utf16_substr(&st.text, 0, st.selection_start);
    let last_char = before.chars().last();

    match last_char {
        Some('.') | Some('!') | Some('?') => CAP_MODE_CHARACTERS | CAP_MODE_SENTENCES,
        Some(' ') => CAP_MODE_WORDS,
        _ => 0,
    }
}

/// `getSelectionStart()` — позиция начала выделения (UTF-16), для getExtractedText.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetSelectionStart(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    current_editor_state().map_or(0, |st| st.selection_start as jint)
}

/// `getSelectionEnd()` — позиция конца выделения (UTF-16).
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetSelectionEnd(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    current_editor_state().map_or(0, |st| st.selection_end as jint)
}

/// `setSelection(start, end)` — IME просит передвинуть курсор/выделение.
/// Пушим `ImeCmd::SetSelection`; применение к egui-курсору на текущий момент
/// no-op (логируется) — курсором управляет сам egui (тапы/стрелки).
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeSetSelection(
    _env: JNIEnv,
    _class: JClass,
    start: i32,
    end: i32,
) {
    log::info!("IME-JNI: setSelection {start}..{end}");
    push_cmd(ImeCmd::SetSelection { start, end });
}

/// `setComposingRegion(start, end)` — IME помечает диапазон композиции.
/// Читает текст из `ImeEditorState` и отправляет `ImeCmd::ComposingRange`
/// с вырезанным текстом и границами.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeSetComposingRegion(
    _env: JNIEnv,
    _class: JClass,
    start: i32,
    end: i32,
) {
    log::info!("IME-JNI: setComposingRegion {start}..{end}");
    let state = current_editor_state();
    if let Some(st) = state {
        let text = utf16_substr(&st.text, start as usize, end as usize);
        log::info!("IME-JNI: setComposingRegion text={:?}", text);
        push_cmd(ImeCmd::ComposingRange { text, start, end });
    }
}

/// Возвращает composing_start из текущего `ImeEditorState`. Если composing
/// не активен — `-1` (Android-конвенция).
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetComposingStart(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    current_editor_state()
        .and_then(|st| st.composing_start)
        .map_or(-1, |v| v as jint)
}

/// Возвращает composing_end из текущего `ImeEditorState`. Если composing
/// не активен — `-1` (Android-конвенция).
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeGetComposingEnd(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    current_editor_state()
        .and_then(|st| st.composing_end)
        .map_or(-1, |v| v as jint)
}

/// `beginBatchEdit()` — начало пакетной операции IME.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeBeginBatchEdit(
    _env: JNIEnv,
    _class: JClass,
) {
    push_cmd(ImeCmd::BeginBatchEdit);
}

/// `endBatchEdit()` — завершение пакетной операции IME.
#[no_mangle]
pub extern "system" fn Java_com_example_egui_1android_EguiImeView_nativeEndBatchEdit(
    _env: JNIEnv,
    _class: JClass,
) {
    push_cmd(ImeCmd::EndBatchEdit);
}

// ─── Rust → Kotlin: показ/скрытие клавиатуры через EguiImeView ───────────
//
// Главный цикл (`loop.rs`) вызывает эти функции вместо `backend.show_keyboard()`
// / `hide_keyboard()`, чтобы клавиатура фокусировала именно невидимый
// `EguiImeView` (через его InputConnection), а не штатный IME GameActivity.

/// Вызвать `EguiActivity.showSoftInputForIme()` на главном Java-потоке.
pub fn show_soft_input_jni(vm_ptr: *mut std::ffi::c_void, activity_ptr: *mut std::ffi::c_void) {
    log::info!("LOOP-JNI: show_soft_input enter");
    call_activity_noarg_uithread(vm_ptr, activity_ptr, "showSoftInputForIme", "()V");
    log::info!("LOOP-JNI: show_soft_input exit");
}

/// Вызвать `EguiActivity.hideSoftInputForIme()` на главном Java-потоке.
pub fn hide_soft_input_jni(vm_ptr: *mut std::ffi::c_void, activity_ptr: *mut std::ffi::c_void) {
    log::info!("LOOP-JNI: hide_soft_input enter");
    call_activity_noarg_uithread(vm_ptr, activity_ptr, "hideSoftInputForIme", "()V");
    log::info!("LOOP-JNI: hide_soft_input exit");
}

/// Обновить imeOptions/inputType текущего IME-View.
///
/// `ime_options` — битовая маска `android.view.inputmethod.EditorInfo.IME_ACTION_*`;
/// `input_type` — `android.text.InputType.TYPE_*`. Вызывается при смене активного
/// TextEdit, когда нужен другой тип клавиатуры/действия.
pub fn set_ime_options_jni(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
    ime_options: i32,
    input_type: i32,
) {
    log::info!("LOOP-JNI: set_ime_options enter ({input_type:#x}, {ime_options:#x})");
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return;
    }
    unsafe {
        let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
            Ok(j) => j,
            Err(_) => return,
        };
        let mut env = match jvm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return,
        };
        let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);
        let _ = env.call_method(
            &activity,
            "setImeOptions",
            "(II)V",
            &[
                jni::objects::JValue::Int(ime_options),
                jni::objects::JValue::Int(input_type),
            ],
        );
    }
    log::info!("LOOP-JNI: set_ime_options exit");
}

/// Передать прямоугольник курсор (экранные px) в Kotlin для candidate window
/// и позиционирования кандидатов IME. Вызывается главным циклом каждый кадр,
/// пока IME активна (cursor_rect из `platform_output.ime`).
///
/// `left/top/right/bottom` — пиксели экрана (px), вертикально ориентированы
/// на физический размер окна (уже умножены на `pixels_per_point` в loop).
pub fn update_cursor_rect_jni(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) {
    log::info!("LOOP-JNI: update_cursor_rect enter [{left},{top},{right},{bottom}]");
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return;
    }
    unsafe {
        let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
            Ok(j) => j,
            Err(_) => return,
        };
        let mut env = match jvm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return,
        };
        let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);
        let _ = env.call_method(
            &activity,
            "updateCursorRect",
            "(IIII)V",
            &[
                jni::objects::JValue::Int(left),
                jni::objects::JValue::Int(top),
                jni::objects::JValue::Int(right),
                jni::objects::JValue::Int(bottom),
            ],
        );
    }
    log::info!("LOOP-JNI: update_cursor_rect exit");
}

/// Вспомогательный вызов метода Activity без аргументов на главном Java-потоке.
///
/// Метод должен сам планировать работу через `view.post` (см. Kotlin), потому
/// что прямой JNI `call_method` с UI-ищем из произвольного потока небезопасен
/// для `requestFocus`/`showSoftInput`. Для простоты выполняем вызов сразу
/// (Kotlin-метод использует `post` для перевода на UI-поток).
fn call_activity_noarg_uithread(
    vm_ptr: *mut std::ffi::c_void,
    activity_ptr: *mut std::ffi::c_void,
    method: &str,
    signature: &str,
) {
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return;
    }
    unsafe {
        let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
            Ok(j) => j,
            Err(e) => {
                log::warn!("ime_jni: JavaVM::from_raw: {e:?}");
                return;
            }
        };
        let mut env = match jvm.attach_current_thread() {
            Ok(e) => e,
            Err(e) => {
                log::warn!("ime_jni: attach_current_thread: {e:?}");
                return;
            }
        };
        let activity = jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject);
        match env.call_method(&activity, method, signature, &[]) {
            Ok(_) => {}
            Err(e) => log::warn!("ime_jni: {method}: {e:?}"),
        }
    }
}
