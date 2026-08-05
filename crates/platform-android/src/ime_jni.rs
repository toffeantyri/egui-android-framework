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
use jni::JNIEnv;

/// Общий обработчик: положить команду в `PlatformState.ime_cmds`.
///
/// JNI-функции вызываются на главном Java-потоке. Если `PlatformState` ещё
/// не инициализирован (цикл не запущен) — команду просто игнорируем.
fn push_cmd(cmd: ImeCmd) {
    match GLOBAL_PLATFORM_STATE.get() {
        Some(state) => state.push_ime_cmd(cmd),
        None => log::warn!(
            "IME-JNI: PlatformState не инициализирован — команда отброшена: {:?}",
            cmd
        ),
    }
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

// ─── Rust → Kotlin: показ/скрытие клавиатуры через EguiImeView ───────────
//
// Главный цикл (`loop.rs`) вызывает эти функции вместо `backend.show_keyboard()`
// / `hide_keyboard()`, чтобы клавиатура фокусировала именно невидимый
// `EguiImeView` (через его InputConnection), а не штатный IME GameActivity.

/// Вызвать `EguiActivity.showSoftInputForIme()` на главном Java-потоке.
pub fn show_soft_input_jni(vm_ptr: *mut std::ffi::c_void, activity_ptr: *mut std::ffi::c_void) {
    call_activity_noarg_uithread(vm_ptr, activity_ptr, "showSoftInputForIme", "()V");
}

/// Вызвать `EguiActivity.hideSoftInputForIme()` на главном Java-потоке.
pub fn hide_soft_input_jni(vm_ptr: *mut std::ffi::c_void, activity_ptr: *mut std::ffi::c_void) {
    call_activity_noarg_uithread(vm_ptr, activity_ptr, "hideSoftInputForIme", "()V");
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
