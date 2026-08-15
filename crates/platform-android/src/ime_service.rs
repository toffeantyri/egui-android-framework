//! Изолированный сервис ввода: единый источник состояния композиции и
//! трансляция команд из "внешнего мира" (Ime/InputConnection и UI) в
//! элементарные буферные операции для UI (egui-поле).
//!
//! # Мотивация
//!
//! Раньше состояние композиции было размазано по `ime_logic.rs`
//! (ImeBuffer/last_preedit/ime_replace_range), `input_processing.rs` и
//! двухкадровой доставке в `loop.rs`, а также дублировалось в egui-patch
//! (`replace_range`). Это приводило к рассинхронизации (потеря букв /
//! "каша") при непрерывном вводе, где Gboard чередует посимвольный,
//! нарастающий и перекомпозицию через `setComposingRegion`.
//!
//! Этот модуль инкапсулирует ввод по схеме, похожей на MVI, но изолированно
//! от системы MVI фреймворка: **команды** (от IME и UI) → редьюсер →
//! **буферные операции** (для UI). Модуль не зависит от Android/JNI и не
//! использует egui-Preedit (Gboard сам показывает композицию; буфер просто
//! получает итоговые Insert/Replace/Delete), поэтому тестируется на хосте.
//!
//! # Владение текстом
//!
//! Сервис НЕ хранит само поле текста (буфером владеет UI/egui). Он держит
//! только: активный регион композиции (для корректной замены при наращивании
//! и перекомпозиции), курсор и глубину batch. UI синхронизирует снапшот текста
//! через [`UiCmd::SyncText`], чтобы сервис мог отвечать InputConnection
//! (`getTextBeforeCursor`) и вычислять регионы относительно актуального буфера.

/// Команда ввода от внешней стороны (IME/InputConnection) или от UI.
#[derive(Debug, Clone, PartialEq)]
pub enum ImeCommand {
    /// Источник — Android InputConnection (Gboard и др.).
    Ime(ImeCmd),
    /// Источник — UI (управление фокусом/текстом/синхронизация).
    Ui(UiCmd),
}

/// Команда от InputConnection (контракт приведён к char-индексам, без UTF-16).
#[derive(Debug, Clone, PartialEq)]
pub enum ImeCmd {
    /// `commitText(text)` — финальное подтверждение композиции.
    Commit(String),
    /// `setComposingText(text)` — промежуточный preedit. Может быть пустым
    /// (завершение композиции).
    Composing(String),
    /// `setComposingRegion(start, end)` — Gboard перестраивает существующий
    /// текст в char-диапазоне `start..end` (перекомпозиция/замена).
    Region { start: usize, end: usize },
    /// `deleteSurroundingText(before, after)` — удалить вокруг курсора.
    DeleteSurrounding { before: usize, after: usize },
    /// `beginBatchEdit()` / `endBatchEdit()` — группировка ивентов.
    Batch(bool), // true = begin, false = end
    /// `performEditorAction`: перейти к следующему полю.
    Next,
    /// `performEditorAction`: завершить редактирование.
    Done,
}

/// Команда от UI.
#[derive(Debug, Clone, PartialEq)]
pub enum UiCmd {
    /// Поле получило фокус (сброс композиции).
    Focus,
    /// Поле потеряло фокус (сброс композиции).
    Blur,
    /// UI прислал актуальный текст поля (синхронизация снапшота для
    /// InputConnection-ответов и вычисления регионов).
    SyncText(String),
    /// Переместить курсор (char-индекс).
    MoveCursor(usize),
}

/// Результат применения команды: элементарные буферные операции, которые UI
/// должен применить к своему полю (без знаний об Android/egui-предикте).
#[derive(Debug, Clone, PartialEq)]
pub enum ImeEvent {
    /// Вставить `text` в позицию курсора (обычный ввод/commit).
    Insert(String),
    /// Заменить char-диапазон `start..end` буфера на `replacement`
    /// (наращивание композиции или перекомпозиция через `setComposingRegion`).
    Replace {
        start: usize,
        end: usize,
        replacement: String,
    },
    /// Удалить `before` символов до курсора и/или `after` после.
    Delete { before: usize, after: usize },
    /// Переместить курсор в char-индекс `index`.
    Cursor(usize),
    /// Управляющее действие (не текст) для главного цикла.
    Action(ImeActionEvent),
}

/// Управляющее изменение (не текст) для главного цикла.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeActionEvent {
    /// Перевести фокус на следующее поле.
    Next,
    /// Завершить редактирование, скрыть клавиатуру.
    Done,
}

/// Единое состояние композиции ввода.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImeServiceState {
    /// Активный регион композиции в буфере (char-индексы). `None` — нет живой
    /// композиции, последующее `Insert` не заменяет, а добавляет.
    pub composition_range: Option<std::ops::Range<usize>>,
    /// Позиция курсора (char-индекс).
    pub cursor: usize,
    /// Глубина вложенных batch-операций.
    pub batch_depth: u32,
    /// Снапшот текста поля, синхронизированный из UI через `UiCmd::SyncText`.
    pub text_snapshot: String,
}

/// Абстракция сервиса ввода. Реализация не знает про Android/egui — только
/// про команды и буферные операции.
pub trait ImeService {
    /// Применить команду, вернуть буферные операции для UI.
    fn apply(&mut self, command: ImeCommand) -> Vec<ImeEvent>;
    /// Текущее состояние композиции.
    fn state(&self) -> &ImeServiceState;
}

/// Контрактная реализация `ImeService` — редьюсер команд в буферные операции.
#[derive(Debug, Default)]
pub struct DefaultImeService {
    state: ImeServiceState,
    /// Операции, накопленные внутри batch, выгружаются при `Batch(false)`.
    batch_events: Vec<ImeEvent>,
}

/// Способ применения `Composing` — вставка без предшествующего региона либо
/// замена существующего региона композиции.
enum ComposeMode {
    /// Новая композиция: вставить текст (Insert).
    Insert,
    /// Продолжение/перекомпозиция: заменить регион `start..end`.
    Replace { start: usize, end: usize },
}

impl DefaultImeService {
    fn push(&mut self, events: &mut Vec<ImeEvent>, ev: ImeEvent) {
        if self.state.batch_depth > 0 {
            self.batch_events.push(ev);
        } else {
            events.push(ev);
        }
    }

    /// Решить, как применить новый preedit (вставить или заменить существующий
    /// регион композиции).
    fn compose_mode(&self) -> ComposeMode {
        match &self.state.composition_range {
            Some(range) if !range.is_empty() => ComposeMode::Replace {
                start: range.start,
                end: range.end,
            },
            _ => ComposeMode::Insert,
        }
    }

    /// Заменить char-диапазон `start..end` снапшота на `replacement`.
    /// Диапазон клампится к длине строки; если start==end — это вставка.
    fn replace_snapshot(&mut self, start: usize, end: usize, replacement: &str) {
        let mut chars: Vec<char> = self.state.text_snapshot.chars().collect();
        let s = start.min(chars.len());
        let e = end.min(chars.len()).max(s);
        chars.splice(s..e, replacement.chars());
        self.state.text_snapshot = chars.into_iter().collect();
    }

    /// Применить preedit как вставку/замену, обновить снапшот, курсор и регион.
    /// Пустой preedit — завершение композиции: только сбрасываем регион, не
    /// двигая курсор и не порождая события.
    fn apply_composing(&mut self, text: &str, events: &mut Vec<ImeEvent>) {
        if text.is_empty() {
            log::info!(
                "IME-SVC: composing пустой — сброс region (cursor остаётся {})",
                self.state.cursor
            );
            self.state.composition_range = None;
            return;
        }
        let mode = self.compose_mode();
        let new_len = text.chars().count();
        match mode {
            ComposeMode::Insert => {
                let start = self.state.cursor;
                let end = start.saturating_add(new_len);
                self.state.cursor = end;
                self.state.composition_range = Some(start..end);
                self.replace_snapshot(start, start, text);
                log::info!(
                    "IME-SVC: composing {:?} → Insert в cursor={} (end={}) → cursor=>{}",
                    text,
                    start,
                    end,
                    self.state.cursor
                );
                self.push(events, ImeEvent::Insert(text.to_owned()));
            }
            ComposeMode::Replace { start, end } => {
                let new_end = start.saturating_add(new_len);
                self.state.cursor = new_end;
                self.state.composition_range = Some(start..new_end);
                self.replace_snapshot(start, end, text);
                log::info!(
                    "IME-SVC: composing {:?} → Replace {}..{} (region=Some({}..{})) → cursor=>{}",
                    text,
                    start,
                    end,
                    start,
                    new_end,
                    self.state.cursor
                );
                self.push(
                    events,
                    ImeEvent::Replace {
                        start,
                        end,
                        replacement: text.to_owned(),
                    },
                );
            }
        }
    }

    fn reset_composition(&mut self) {
        // Сброс состояния композиции. ВАЖНО: НЕ очищаем `batch_events` — иначе
        // `Commit`/`DeleteSurrounding`, вызванные ВНУТРИ активного batch, сотрут
        // ранее накопленные операции, и на `Batch(false)` буквы «пропадут» из
        // поля (баг «ввожу привет — половина букв не попадает»). Очистку
        // накопленного делает только `Batch(begin)`.
        log::info!(
            "IME-SVC: reset_composition (region=None, batch_events={} НЕ очищаем)",
            self.batch_events.len()
        );
        self.state.composition_range = None;
        // self.batch_events.clear();  // <-- удалено: была причина потери букв
    }

    /// Вставить `replacement` в снапшот (для `Commit` без активного preedit) на
    /// позицию курсора, вернуть событие Insert и обновить состояние.
    fn insert_at_cursor(&mut self, events: &mut Vec<ImeEvent>, replacement: &str) {
        let start = self.state.cursor;
        let end = start.saturating_add(replacement.chars().count());
        self.state.cursor = end;
        self.replace_snapshot(start, start, replacement);
        self.push(events, ImeEvent::Insert(replacement.to_owned()));
    }
}

impl ImeService for DefaultImeService {
    fn apply(&mut self, command: ImeCommand) -> Vec<ImeEvent> {
        // Диагностика (временный лог для локализации потери ввода).
        log::info!(
            "IME-SVC: apply {:?} | cursor={} region={:?} snapshot={:?} batch={}",
            command,
            self.state.cursor,
            self.state.composition_range,
            self.state.text_snapshot,
            self.state.batch_depth
        );
        let mut out: Vec<ImeEvent> = Vec::new();
        match command {
            ImeCommand::Ime(cmd) => match cmd {
                ImeCmd::Commit(text) => {
                    // Behave like a plain EditText: commitText DOPPYTSYVAET к буферу,
                    // а не заменяет уже набранный предикт. Замена предикта — только
                    // если он ещё ЖИВОЙ (не снят). После снятия `Composing("")`
                    // набранные буквы уже зафиксированы — Commit добавляет новую
                    // букву (иначе «привет» рассыпается в «ет»).
                    let live_range = self.state.composition_range.take();
                    self.state.composition_range = None;
                    match live_range {
                        Some(range) => {
                            let new_end = range.start.saturating_add(text.chars().count());
                            self.state.cursor = new_end;
                            self.replace_snapshot(range.start, range.end, &text);
                            log::info!(
                                "IME-SVC: Commit {:?} → Replace {:?} → cursor=>{}",
                                text,
                                range,
                                self.state.cursor
                            );
                            self.push(
                                &mut out,
                                ImeEvent::Replace {
                                    start: range.start,
                                    end: range.end,
                                    replacement: text.to_owned(),
                                },
                            );
                        }
                        None => {
                            log::info!("IME-SVC: Commit {:?} → Insert", text);
                            self.insert_at_cursor(&mut out, &text);
                        }
                    }
                }
                ImeCmd::Composing(text) => {
                    self.apply_composing(&text, &mut out);
                }
                ImeCmd::Region { start, end } => {
                    // Gboard помечает диапазон как active-composition: следующий
                    // preedit ЗАМЕНИТ этот диапазон (а не вставит рядом). Регион
                    // идёт в реальных char-индексах снапшота — это место, где
                    // продолжается ввод, потому cursor = end.
                    //
                    // end НЕ клампим вниз к длине снапшота: регион может указывать
                    // на «скоро введённый» текст (снапшот ещё не синхронизирован),
                    // а preedit сам достроит его через Replace. cursor клампим,
                    // чтобы не вылететь за границы текущего текста.
                    let len = self.state.text_snapshot.chars().count();
                    let cursor = end.min(len);
                    let start = start.min(cursor);
                    let end = end.max(start);
                    self.state.composition_range = Some(start..end);
                    self.state.cursor = cursor;
                    log::info!(
                        "IME-SVC: Region {}..{} → cursor=>{}",
                        start,
                        end,
                        self.state.cursor
                    );
                }
                ImeCmd::DeleteSurrounding { before, after } => {
                    self.reset_composition();
                    // Применяем удаление к снапшоту, чтобы курсор и текст остались
                    // согласованы (UI применит тот же Delete).
                    let c = self.state.cursor;
                    let chars: Vec<char> = self.state.text_snapshot.chars().collect();
                    let del_before = before.min(c);
                    let del_after = after.min(chars.len().saturating_sub(c));
                    let s = c - del_before;
                    let e = c + del_after;
                    let mut chars = chars;
                    chars.splice(s..e, std::iter::empty::<char>());
                    self.state.text_snapshot = chars.into_iter().collect();
                    self.state.cursor = s;
                    self.push(&mut out, ImeEvent::Delete { before, after });
                }
                ImeCmd::Batch(begin) => {
                    if begin {
                        self.state.batch_depth += 1;
                        self.batch_events.clear();
                    } else {
                        self.state.batch_depth = self.state.batch_depth.saturating_sub(1);
                        if self.state.batch_depth == 0 {
                            out.append(&mut self.batch_events);
                        }
                    }
                }
                ImeCmd::Next => out.push(ImeEvent::Action(ImeActionEvent::Next)),
                ImeCmd::Done => out.push(ImeEvent::Action(ImeActionEvent::Done)),
            },
            ImeCommand::Ui(cmd) => match cmd {
                UiCmd::Focus | UiCmd::Blur => {
                    self.reset_composition();
                    self.state.batch_depth = 0;
                }
                UiCmd::SyncText(text) => {
                    self.state.text_snapshot = text;
                }
                UiCmd::MoveCursor(idx) => {
                    self.state.cursor = idx;
                    self.state.composition_range = None;
                    self.push(&mut out, ImeEvent::Cursor(idx));
                }
            },
        }
        out
    }

    fn state(&self) -> &ImeServiceState {
        &self.state
    }
}

/// Перевести существующую команду из входного типа (`ime_logic::ImeCmd`, который
/// кладёт JNI-мост в очередь `PlatformState.ime_cmds`) в команду сервиса.
///
/// `None` — команда не попадает в сервис (например, `SetSelection`/`PrivateCommand`
/// не меняют состояние композиции в этой модели).
pub fn translate_legacy(cmd: &crate::ime_logic::ImeCmd) -> Option<ImeCommand> {
    use crate::ime_logic::ImeCmd as Leg;
    let inner = match cmd {
        Leg::Commit(text) => ImeCmd::Commit(text.clone()),
        Leg::Composing(text) => ImeCmd::Composing(text.clone()),
        Leg::ComposingRange {
            start_char,
            end_char,
            ..
        } => ImeCmd::Region {
            start: *start_char,
            end: *end_char,
        },
        Leg::DeleteSurrounding { before, after } => ImeCmd::DeleteSurrounding {
            before: (*before).max(0) as usize,
            after: (*after).max(0) as usize,
        },
        Leg::BeginBatchEdit => ImeCmd::Batch(true),
        Leg::EndBatchEdit => ImeCmd::Batch(false),
        Leg::Next => ImeCmd::Next,
        Leg::Done => ImeCmd::Done,
        // Эти команды не меняют композицию — пропускаем.
        Leg::SetSelection { .. } | Leg::PrivateCommand(_) => return None,
    };
    Some(ImeCommand::Ime(inner))
}

/// Перевести буферную операцию сервиса в egui-событие. `Insert` → `Event::Text`,
/// `Replace` → `ImeEvent::Preedit` с `replace_range` (egui заменяет регион),
/// `Delete` → клавиша удаления, управляющие/курсор — no-op-события.
pub fn to_egui_event(ev: &ImeEvent) -> egui::Event {
    use egui::Key;
    match ev {
        ImeEvent::Insert(text) => egui::Event::Text(text.clone()),
        ImeEvent::Replace {
            start,
            end,
            replacement,
        } => egui::Event::Ime(egui::ImeEvent::Preedit {
            text: replacement.clone(),
            active_range_chars: Some(0..replacement.chars().count()),
            replace_range: Some(*start..*end),
        }),
        ImeEvent::Delete { before, after } => {
            let key = if *before > 0 {
                Key::Backspace
            } else {
                Key::Delete
            };
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }
        }
        ImeEvent::Cursor(_) | ImeEvent::Action(_) => egui::Event::Text(String::new()),
    }
}

/// Вычислить timeout для `poll_events`.
///
/// Логика:
/// - нет graphics → `None` (бесконечно ждать InitWindow)
/// - repaint_delay==0 → немедленно (`Some(0)`)
/// - repaint_delay большой (> 1 час) → `None`
/// - есть ожидающие IME-команды → немедленно (`Some(0)`), чтобы не
///   блокироваться и не задерживать ввод пользователя
/// - иначе → repaint_delay
pub fn poll_timeout(
    no_graphics: bool,
    repaint_delay: std::time::Duration,
    has_ime_cmds: bool,
) -> Option<std::time::Duration> {
    // Максимальный таймаут даже в режиме ожидания — чтобы IME-команды,
    // пришедшие из JNI-потока, не зависали надолго. Android poll_events
    // не прерывается по Wake (баг в нашем коде: Wake просто глотается).
    const MAX_POLL_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

    if no_graphics {
        // Без graphics ждём InitWindow — но не бесконечно, чтобы не
        // подвиснуть навсегда если что-то пошло не так.
        Some(MAX_POLL_TIMEOUT)
    } else if repaint_delay == std::time::Duration::ZERO || has_ime_cmds {
        Some(std::time::Duration::ZERO)
    } else {
        Some(repaint_delay.min(MAX_POLL_TIMEOUT))
    }
}

#[cfg(test)]
mod model {
    //! Модельный буфер текста (без egui), на котором применяются операции
    //! сервиса (Insert/Replace/Delete/Cursor). Позволяет протестировать
    //! инварианты позиционирования и проверить, что сервис не вносит
    //! дубли/потери/«каши» в непрерывном вводе.
    use super::{ImeActionEvent, ImeEvent};

    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct ModelTextBuffer {
        pub chars: Vec<char>,
        pub cursor: usize,
    }

    impl ModelTextBuffer {
        pub fn text(&self) -> String {
            self.chars.iter().collect()
        }

        /// Применить операцию сервиса. Возвращает true, если буфер изменился.
        pub fn apply(&mut self, ev: &ImeEvent) -> bool {
            match ev {
                ImeEvent::Insert(text) => self.insert_at_cursor(text),
                ImeEvent::Replace {
                    start,
                    end,
                    replacement,
                } => self.replace(*start, *end, replacement),
                ImeEvent::Delete { before, after } => self.delete(*before, *after),
                ImeEvent::Cursor(idx) => {
                    self.cursor = (*idx).min(self.chars.len());
                    false
                }
                ImeEvent::Action(_) => false,
            }
        }

        fn insert_at_cursor(&mut self, text: &str) -> bool {
            let pos = self.cursor.min(self.chars.len());
            let mut insert: Vec<char> = text.chars().collect();
            self.chars.splice(pos..pos, insert.drain(..));
            self.cursor = pos + text.chars().count();
            !text.is_empty()
        }

        fn replace(&mut self, start: usize, end: usize, replacement: &str) -> bool {
            let s = start.min(self.chars.len());
            let e = end.min(self.chars.len()).max(s);
            self.chars.splice(s..e, replacement.chars());
            self.cursor = s + replacement.chars().count();
            true
        }

        fn delete(&mut self, before: usize, after: usize) -> bool {
            let c = self.cursor;
            let del_before = before.min(c);
            let del_after = after.min(self.chars.len().saturating_sub(c));
            let start = c - del_before;
            let end = c + del_after;
            self.chars.drain(start..end);
            self.cursor = start;
            del_before + del_after > 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> DefaultImeService {
        DefaultImeService::default()
    }

    fn composing(s: &mut DefaultImeService, text: &str) -> Vec<ImeEvent> {
        s.apply(ImeCommand::Ime(ImeCmd::Composing(text.into())))
    }

    fn commit(s: &mut DefaultImeService, text: &str) -> Vec<ImeEvent> {
        s.apply(ImeCommand::Ime(ImeCmd::Commit(text.into())))
    }

    /// Контракт: стартовое состояние пустое.
    #[test]
    fn service_init_is_empty() {
        let s = svc();
        assert_eq!(s.state().cursor, 0);
        assert_eq!(s.state().composition_range, None);
        assert_eq!(s.state().batch_depth, 0);
    }

    /// Режим A Gboard — посимвольный ввод: `setComposingText("п")` затем
    /// `setComposingText("")` — текст вставляется один раз (Insert), завершение
    /// нарастаний нет (курсор уже стоит).
    #[test]
    fn gboard_single_char_mode_keeps_symbol() {
        let mut s = svc();
        let ev = composing(&mut s, "п");
        assert_eq!(ev, vec![ImeEvent::Insert("п".into())]);
        assert_eq!(s.state().cursor, 1);
        // Завершение композиции
        let ev = composing(&mut s, "");
        assert_eq!(ev, vec![], "завершение не должно дублировать текст");
        assert_eq!(s.state().cursor, 1);
        assert_eq!(s.state().composition_range, None);
    }

    /// Режим B — нарастающий: `"п","пр","при"`. Каждый следующий preedit
    /// заменяет предыдущий регион (Replace), а не вставляет заново.
    #[test]
    fn gboard_growing_composition_replaces() {
        let mut s = svc();
        let ev = composing(&mut s, "п");
        assert_eq!(ev, vec![ImeEvent::Insert("п".into())]);
        let ev = composing(&mut s, "пр");
        assert_eq!(
            ev,
            vec![ImeEvent::Replace {
                start: 0,
                end: 1,
                replacement: "пр".into()
            }]
        );
        let ev = composing(&mut s, "при");
        assert_eq!(
            ev,
            vec![ImeEvent::Replace {
                start: 0,
                end: 2,
                replacement: "при".into()
            }]
        );
        assert_eq!(s.state().cursor, 3);
    }

    /// Режим C — перекомпозиция через `setComposingRegion`:
    /// Gboard отмечает первую букву как активную композицию, и следующий preedit
    /// ЗАМЕНЯЕТ помеченный регион (иначе возникает дубль «ппр» вместо «пр»).
    #[test]
    fn gboard_recomposition_through_region_replaces() {
        let mut s = svc();
        let ev = s.apply(ImeCommand::Ime(ImeCmd::Region { start: 0, end: 1 }));
        assert_eq!(
            ev,
            vec![],
            "Region не вставляет текст, только помечает диапазон для замены"
        );
        assert_eq!(s.state().composition_range, Some(0..1));
        // Следующий preedit ЗАМЕНЯЕТ регион 0..1.
        let ev = composing(&mut s, "пр");
        assert_eq!(
            ev,
            vec![ImeEvent::Replace {
                start: 0,
                end: 1,
                replacement: "пр".into()
            }]
        );
        assert_eq!(s.state().cursor, 2);
        assert_eq!(s.state().composition_range, Some(0..2));
    }

    /// Commit — вставка финала, курсор за текстом.
    #[test]
    fn commit_emits_insert_and_clears() {
        let mut s = svc();
        let ev = s.apply(ImeCommand::Ime(ImeCmd::Commit("привет".into())));
        assert_eq!(ev, vec![ImeEvent::Insert("привет".into())]);
        assert_eq!(s.state().cursor, 6);
    }

    /// DeleteSurrounding — сбрасывает композицию, эмитит Delete.
    #[test]
    fn delete_resets_and_emits() {
        let mut s = svc();
        composing(&mut s, "абв");
        let ev = s.apply(ImeCommand::Ime(ImeCmd::DeleteSurrounding {
            before: 1,
            after: 0,
        }));
        assert_eq!(
            ev,
            vec![ImeEvent::Delete {
                before: 1,
                after: 0
            }]
        );
        assert_eq!(s.state().composition_range, None);
    }

    /// Batch группирует операции до end.
    #[test]
    fn batch_groups_events() {
        let mut s = svc();
        s.apply(ImeCommand::Ime(ImeCmd::Batch(true)));
        let evs = composing(&mut s, "а");
        assert_eq!(evs, vec![], "операции буферизуются внутри batch");
        let evs = s.apply(ImeCommand::Ime(ImeCmd::Batch(false)));
        assert_eq!(evs.len(), 1);
        assert!(matches!(evs[0], ImeEvent::Insert(ref t) if t == "а"));
    }

    /// MoveCursor от UI — Cursor-ивент и сброс композиции.
    #[test]
    fn move_cursor_updates_state() {
        let mut s = svc();
        let ev = s.apply(ImeCommand::Ui(UiCmd::MoveCursor(5)));
        assert_eq!(ev, vec![ImeEvent::Cursor(5)]);
        assert_eq!(s.state().cursor, 5);
    }

    /// РЕГРЕССИЯ «ввод в середину» (реальный лог устройства): после
    /// перемещения курсора в середину слова следующий `Composing` должен
    /// ВСТАВИТЬСЯ в эту позицию, сохранив префикс (не затирать слово и не
    /// уходить в конец). Зеркало device-теста `insert_mid_word_preserves_prefix`
    /// (examples/showcase) — держим reducer-логику в хосте.
    #[test]
    fn insert_mid_word_preserves_prefix_via_sync_and_move() {
        let mut s = svc();
        let mut buf = model::ModelTextBuffer::default();
        buf.chars = "привет".chars().collect();
        buf.cursor = 0;

        // Готовое слово + каретка в середине (char 3), как после тапа.
        s.apply(ImeCommand::Ui(UiCmd::SyncText("привет".into())));
        for ev in s.apply(ImeCommand::Ui(UiCmd::MoveCursor(3))) {
            buf.apply(&ev);
        }

        // Вводим «X» в середину.
        for ev in s.apply(ImeCommand::Ime(ImeCmd::Composing("X".into()))) {
            buf.apply(&ev);
        }

        assert_eq!(
            buf.text(),
            "приXвет",
            "ввод в середину должен сохранить префикс: {:?}",
            buf.text()
        );
    }

    /// ЛОВУШКА ПОТЕРИ НАКОПЛЕННОГО: внутри активного batch (`Batch(true)..
    /// Batch(false)`) команда `DeleteSurrounding` (или `Commit`) вызывает
    /// `reset_composition()`, который делает `batch_events.clear()`. Если это
    /// стирает ранее накопленные события — буквы «пропадают» из поля.
    /// Тест падает, если reset_composition внутри batch уничтожает уже
    /// накопленные операции.
    #[test]
    fn batch_delete_inside_must_not_erase_previous_events() {
        let mut s = svc();
        // Начинаем пакет: накапливаем «пр» и «при».
        s.apply(ImeCommand::Ime(ImeCmd::Batch(true)));
        let inner = composing(&mut s, "п");
        assert_eq!(inner, vec![], "внутри batch операции буферизуются");
        let inner = composing(&mut s, "пр");
        assert_eq!(inner, vec![]);
        let inner = composing(&mut s, "при");
        assert_eq!(inner, vec![]);

        // Внутри ТОГО ЖЕ batch приходит delete (Gboard так делает часто).
        s.apply(ImeCommand::Ime(ImeCmd::DeleteSurrounding {
            before: 1,
            after: 0,
        }));

        // Завершение batch: ВСЕ накопленные события должны выгрузиться.
        let evs = s.apply(ImeCommand::Ime(ImeCmd::Batch(false)));

        // Должны быть: Insert("п" или 3 события) + Delete. Если хотя бы один
        // Insert пропал из-за batch_events.clear() в reset_composition — тест не
        // даст собрать слово на буфере.
        let text_ops: usize = evs
            .iter()
            .filter(|e| matches!(e, ImeEvent::Insert(_) | ImeEvent::Replace { .. }))
            .count();
        assert!(
            text_ops >= 3,
            "Из batch выгружено всего {} текстовых операций (ожидали >=3); reset_composition стёр накопленное — буквы пропадают: {:?}",
            text_ops,
            evs
        );
    }

    /// ЛОВУШКА ПОТЕРИ ПРИ COMMIT ВНУТРИ BATCH:
    /// Gboard подтверждает preedit через `commitText`: Commit ЗАМЕНЯЕТ активный
    /// (или последний) регион композиции и заново использует уже набранный
    /// preedit — не выстраивает его рядом (иначе «привет» превращается в кашу).
    /// При этом события, накопленные в batch, НЕ должны быть стёрты
    /// `reset_composition` (иначе буквы «пропадут» на `Batch(false)`).
    #[test]
    fn batch_commit_replaces_preedit_and_keeps_events() {
        let mut s = svc();
        // Набираем preedit «пр», затем подтверждаем «и» через commit внутри batch.
        s.apply(ImeCommand::Ime(ImeCmd::Batch(true)));
        composing(&mut s, "п");
        composing(&mut s, "пр");
        s.apply(ImeCommand::Ime(ImeCmd::Commit("и".into())));
        let evs = s.apply(ImeCommand::Ime(ImeCmd::Batch(false)));

        // После всех операций на снапшоте сервиса должно остаться «и» (commit
        // заменил preedit «пр»), а не «при» (вставка рядом) и не пусто (потеря).
        assert_eq!(
            s.state().text_snapshot,
            "и",
            "Commit должен заменить preedit на снапшоте: ушло {:?} (события {:?})",
            s.state().text_snapshot,
            evs
        );
        assert!(
            !s.state().text_snapshot.is_empty(),
            "Commit не должен терять подтверждённый текст: {:?}",
            evs
        );
    }

    /// Commit заменяет живой preedit (буфер сжимается, как в реальном логе
    /// «пр»→Commit«и»→«и»): снапшот сервиса отражает это.
    #[test]
    fn commit_replaces_active_preedit_on_snapshot() {
        let mut s = svc();
        composing(&mut s, "п");
        composing(&mut s, "пр");
        assert_eq!(s.state().text_snapshot, "пр");
        let evs = s.apply(ImeCommand::Ime(ImeCmd::Commit("и".into())));
        assert!(
            !evs.is_empty(),
            "Commit должен вернуть операцию замены preedit"
        );
        assert_eq!(
            s.state().text_snapshot,
            "и",
            "commitText заменяет preedit: снапшот должен стать «и», а стал {:?}",
            s.state().text_snapshot
        );
        assert_eq!(s.state().cursor, 1);
        assert_eq!(
            s.state().composition_range,
            None,
            "после commit нет живой композиции"
        );
    }

    /// Перевод из легаси-типа: маппинг JNI-команды → команда сервиса.
    #[test]
    fn translate_legacy_maps_commands() {
        use crate::ime_logic::ImeCmd as Leg;
        assert_eq!(
            translate_legacy(&Leg::Composing("пр".into())),
            Some(ImeCommand::Ime(ImeCmd::Composing("пр".into()))),
        );
        assert_eq!(
            translate_legacy(&Leg::ComposingRange {
                text: "пр".into(),
                start_char: 0,
                end_char: 2,
            }),
            Some(ImeCommand::Ime(ImeCmd::Region { start: 0, end: 2 })),
        );
        assert_eq!(
            translate_legacy(&Leg::BeginBatchEdit),
            Some(ImeCommand::Ime(ImeCmd::Batch(true)))
        );
        assert_eq!(
            translate_legacy(&Leg::EndBatchEdit),
            Some(ImeCommand::Ime(ImeCmd::Batch(false)))
        );
        assert_eq!(
            translate_legacy(&Leg::Done),
            Some(ImeCommand::Ime(ImeCmd::Done))
        );
        assert_eq!(
            translate_legacy(&Leg::SetSelection { start: 1, end: 2 }),
            None
        );
    }

    /// Контракт: Insert → egui::Event::Text одного символа, Delete → клавиша.
    #[test]
    fn to_egui_event_mapping() {
        use egui::Key;
        assert!(matches!(
            to_egui_event(&ImeEvent::Insert("пр".into())),
            egui::Event::Text(t) if t == "пр"
        ));
        // Replace транслируется в egui-Preedit с replace_range (egui заменит регион).
        match to_egui_event(&ImeEvent::Replace {
            start: 0,
            end: 2,
            replacement: "прив".into(),
        }) {
            egui::Event::Ime(egui::ImeEvent::Preedit {
                text,
                replace_range,
                ..
            }) => {
                assert_eq!(text, "прив");
                assert_eq!(replace_range, Some(0usize..2usize));
            }
            other => panic!("ожидал Preedit, получил {:?}", other),
        }
        assert!(matches!(
            to_egui_event(&ImeEvent::Delete {
                before: 1,
                after: 0
            }),
            egui::Event::Key {
                key: Key::Backspace,
                ..
            }
        ));
    }

    // ─── Инвариант-тесты рискованного фрагмента (позиционирование/рассинхрон) ───

    /// Помощник: прогнать команды через сервис, применить операции к модельному
    /// буферу. НЕ синхронизируем снапшот (как в рантайме: loop.rs не зовёт
    /// SyncText каждый кадр) — сервис сам ведёт снапшот, и он должен совпадать
    /// с буфером, если события применяются синхронно.
    fn run_with_buffer(
        s: &mut DefaultImeService,
        buf: &mut model::ModelTextBuffer,
        cmd: ImeCommand,
    ) {
        for ev in s.apply(cmd) {
            buf.apply(&ev);
        }
    }

    /// Инвариант: после любой цепочки команд снапшот сервиса == тексту модельного
    /// буфера — курсор и регионы сервиса не «уезжают» от реального буфера.
    #[test]
    fn invariant_snapshot_matches_buffer() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();

        let seq = [
            ImeCommand::Ime(ImeCmd::Composing("п".into())),
            ImeCommand::Ime(ImeCmd::Composing(String::new())),
            ImeCommand::Ime(ImeCmd::Composing("р".into())),
            ImeCommand::Ime(ImeCmd::Composing(String::new())),
            ImeCommand::Ime(ImeCmd::Composing("рив".into())),
            ImeCommand::Ime(ImeCmd::Composing("риве".into())),
        ];
        for cmd in seq {
            run_with_buffer(&mut s, &mut b, cmd);
        }
        assert_eq!(
            s.state().text_snapshot,
            b.text(),
            "снапшот сервиса == буфер"
        );
        assert!(
            !b.text().contains("пп") && !b.text().contains("прпр"),
            "нет дубля: {:?}",
            b.text()
        );
    }

    /// НАРРАСТОВОЕ наращивание через модельный буфер: «п»→«пр» НЕ даёт «ппр».
    /// Ловит дубль при Replace-наращивании.
    #[test]
    fn growing_does_not_duplicate_on_buffer() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("п".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("пр".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("при".into())),
        );
        assert_eq!(b.text(), "при", "наращивание не должно дублировать буквы");
        assert_eq!(b.cursor, 3);
        assert_eq!(s.state().text_snapshot, b.text());
    }

    /// Сквозной сценарий Gboard для слова «привет» (по реальному логу):
    /// непрерывный нарастающий preedit c периодическим `setComposingRegion`,
    /// БЕЗ обнуления в середине. КАЖДЫЙ наращенный preedit ЗАМЕНЯЕТ помеченный
    /// регион (а не вставляет рядом — иначе дубль «ппр»/«припривет»).
    /// Сервис должен собрать слово целиком без потери хвоста («вет»).
    #[test]
    fn gboard_word_privet_assembles_fully_on_buffer() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();

        // Первая буква — Insert, затем Gboard помечает её регионом и наращивает.
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("п".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Region { start: 0, end: 1 }),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("пр".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("при".into())),
        );
        // Наращивание до полного слова продолжает заменять регион.
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("прив".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("приве".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("привет".into())),
        );

        assert_eq!(
            b.text(),
            "привет",
            "слово должно собраться полностью, без дубля и потери: {:?}",
            b.text()
        );
        assert!(
            !b.text().contains("ппр") && !b.text().contains("приприв"),
            "нет дубля префикса: {:?}",
            b.text()
        );
        assert_eq!(b.cursor, 6);
        assert_eq!(s.state().text_snapshot, b.text(), "снапшот == буфер");
    }

    /// РЕАЛЬНЫЙ ЦИКЛ Gboard по логу устройства 21616: набираем «вет».
    /// preedit растёт «в»→«ве», Gboard снимает предикт `Composing("")` (`ве` уже
    /// зафиксировано в буфере), затем `commitText("т")` ВСТАВЛЯЕТ «т» в конец,
    /// а не перезаписывает «ве» (иначе теряется букво — баг). Проверяет, что
    /// снапшот всегда == буферу и ввод не теряется между фазами.
    #[test]
    fn real_loop_word_assembles_after_preedit_release() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();

        // Фаза 1: набираем «ве» как живой preedit, снимаем его.
        for cmd in [
            ImeCommand::Ime(ImeCmd::Composing("в".into())),
            ImeCommand::Ime(ImeCmd::Composing("ве".into())),
            ImeCommand::Ime(ImeCmd::Composing(String::new())),
        ] {
            run_with_buffer(&mut s, &mut b, cmd.clone());
            assert_eq!(s.state().text_snapshot, b.text(), "cmd {:?}", cmd);
        }
        assert_eq!(b.text(), "ве");

        // Фаза 2: подтверждаем «т» — Commit вставляет (нет живого предакта).
        run_with_buffer(&mut s, &mut b, ImeCommand::Ime(ImeCmd::Commit("т".into())));
        assert_eq!(
            b.text(),
            "вет",
            "после снятия preedit Commit должен вставить, а не затереть: {:?}",
            b.text()
        );
        assert_eq!(s.state().text_snapshot, b.text());
        assert_eq!(b.cursor, 3);

        // Фаза 3: снова набираем «!» через Commit без предшествующего предакта.
        run_with_buffer(&mut s, &mut b, ImeCommand::Ime(ImeCmd::Commit("!".into())));
        assert_eq!(
            b.text(),
            "вет!",
            "не потерялся следующий вывод: {:?}",
            b.text()
        );
        assert_eq!(s.state().text_snapshot, b.text());
    }

    /// ВОСПРОИЗВЕДЕНИЕ РЕАЛЬНОГО БАГА С УСТРОЙСТВА: Gboard набирает «привет»
    /// и шлёт в InputConnection `setComposingText` + `setComposingRegion` как
    /// в логе. Тест проходит данные через `translate_legacy` (как `loop.rs`)
    /// и требует, чтобы слово собралось целиком. Моделирует ПОСЛЕДНИЙ лог:
    /// п → Region(0..1) → пр → при → пустой preedit → в → ве → вет.
    ///
    /// Если маппинг/редьюсер затеряет «вет» (как в баге «только при») или
    /// задублирует «п» («ппр»/«приприв»), тест УПАДЁТ.
    #[test]
    fn gboard_privet_log_repro_catches_lost_tail() {
        use crate::ime_logic::ImeCmd as Leg;

        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();

        // Помощник: прогон через translate_legacy + сервис + буфер на каждой
        // фазе, как это делает главный цикл (loop.rs, шаг 2.5).
        fn drive(
            s: &mut DefaultImeService,
            b: &mut model::ModelTextBuffer,
            cmd: &crate::ime_logic::ImeCmd,
        ) {
            if let Some(svc) = translate_legacy(cmd) {
                let evs = s.apply(svc);
                for ev in &evs {
                    b.apply(ev);
                }
            }
        }

        drive(&mut s, &mut b, &Leg::Composing("п".into()));
        drive(
            &mut s,
            &mut b,
            &Leg::ComposingRange {
                text: "п".into(),
                start_char: 0,
                end_char: 1,
            },
        );
        drive(&mut s, &mut b, &Leg::Composing("пр".into()));
        drive(&mut s, &mut b, &Leg::Composing("при".into()));
        drive(&mut s, &mut b, &Leg::SetSelection { start: 3, end: 3 });
        drive(&mut s, &mut b, &Leg::Composing(String::new()));
        drive(&mut s, &mut b, &Leg::Composing("в".into()));
        drive(&mut s, &mut b, &Leg::Composing("ве".into()));
        drive(&mut s, &mut b, &Leg::Composing("вет".into()));

        assert_eq!(
            b.text(),
            "привет",
            "СБОЙ ВОСПРОИЗВЕДЕНИЯ: ожидаем «привет», получено {:?} — потерян хвост/дубль",
            b.text()
        );
        assert!(
            !b.text().contains("ппр") && !b.text().contains("ппри"),
            "дубль первой буквы: {:?}",
            b.text()
        );
        assert_eq!(s.state().text_snapshot, b.text(), "снапшот == буфер");
    }

    /// После честного завершения предикта (`Composing("")`) следующий preedit —
    /// это НОВОЕ слово/слово целиком, которое вставляется в текущую позицию
    /// курсора (Insert), а НЕ заменяет предыдущее (иначе дубль/потеря).
    #[test]
    fn composing_after_reset_inserts_new_word_not_replace() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();

        // Введено и завершено слово «при».
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("при".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing(String::new())),
        );
        assert_eq!(b.text(), "при");

        // Следующее слово вставляется после, а не заменяет «при».
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("вет".into())),
        );
        assert_eq!(
            b.text(),
            "привет",
            "новый preedit после сброса = Insert (новое слово): {:?}",
            b.text()
        );
        assert_eq!(b.cursor, 6);
        assert_eq!(s.state().text_snapshot, b.text());
    }

    /// Посимвольный ввод с завершением: символ не теряется в модельном буфере.
    #[test]
    fn single_char_not_lost_on_buffer() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("а".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing(String::new())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("б".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing(String::new())),
        );
        assert_eq!(b.text(), "аб", "посимвольный ввод ничего не теряет");
        assert_eq!(b.cursor, 2);
    }

    /// Delete в середине композиции корректно удаляет и не ломает курсор.
    #[test]
    fn delete_keeps_buffer_consistent() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("абвг".into())),
        );
        assert_eq!(b.text(), "абвг");
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::DeleteSurrounding {
                before: 2,
                after: 0,
            }),
        );
        assert_eq!(b.text(), "аб", "удаление применяется к буферу");
        assert_eq!(s.state().text_snapshot, b.text());
    }

    /// Batch: между Batch(true) и Batch(false) операции не «применяются» к буферу
    /// до выгрузки (на модельном буфере — до конца batch ни одной операции).
    #[test]
    fn batch_defers_application() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();
        run_with_buffer(&mut s, &mut b, ImeCommand::Ime(ImeCmd::Batch(true)));
        let ops_inside = s.apply(ImeCommand::Ime(ImeCmd::Composing("х".into())));
        assert!(ops_inside.is_empty(), "внутри batch операции буферизуются");
        // Применяем только после Batch(false).
        for ev in s.apply(ImeCommand::Ime(ImeCmd::Batch(false))) {
            b.apply(&ev);
        }
        assert_eq!(b.text(), "х");
    }

    // ─── EDGE-CASE: устойчивость к неупорядоченному SyncText/Region ───
    //
    // В рантайме UI публикует реальный текст поля в `SyncText`. Снапшот сервиса
    // должен оставаться консистентным с буфером ДАЖЕ если SyncText приходит с
    // текстом, не совпадающим с текущей живой композицией (egui-буфер может
    // отставать/опережать регион на кадр). Критично: live `composition_range` и
    // `last_composition_range` не должны диктовать индексы в ЧУЖОЙ текст.

    /// SyncText во время активного preedit НЕ должен сломать наращивание:
    /// снапшот перезаписывается только под согласованный с композицией текст.
    #[test]
    fn synctext_inside_live_preedit_does_not_break_growth() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();

        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("п".into())),
        );
        // egui-буфер мог ещё не применить "п" (отставание на кадр) → SyncText "".
        s.apply(ImeCommand::Ui(UiCmd::SyncText(String::new())));
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("пр".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("при".into())),
        );

        assert_eq!(
            b.text(),
            "при",
            "SyncText с отставшим текстом не должен испортить нарастающий preedit: {:?}",
            b.text()
        );
        assert!(s.state().text_snapshot == b.text(), "снапшот==буфер");
    }

    /// Инвариант снапшота на реалистичной цепочке набора слова «привет»:
    /// наращивание preedit, снятие, вставка следующей буквы через Commit, снова
    /// наращивание. Чтение состояния сервиса всегда равно применению событий
    /// к буферу — снапшот не «уезжает» (гарантия корректных индексов Replace).
    #[test]
    fn snapshot_invariant_while_typing_word() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();

        let seq = [
            ImeCommand::Ime(ImeCmd::Composing("п".into())),
            ImeCommand::Ime(ImeCmd::Composing("пр".into())),
            ImeCommand::Ime(ImeCmd::Composing("при".into())),
            ImeCommand::Ime(ImeCmd::Composing("прив".into())),
            ImeCommand::Ime(ImeCmd::Composing("приве".into())),
            ImeCommand::Ime(ImeCmd::Composing("привет".into())),
            ImeCommand::Ime(ImeCmd::Composing(String::new())), // снять предикт «привет»
            ImeCommand::Ime(ImeCmd::Batch(true)),
            ImeCommand::Ime(ImeCmd::Commit(" ".into())), // пробел — вставка
            ImeCommand::Ime(ImeCmd::Batch(false)),
        ];
        for cmd in seq {
            run_with_buffer(&mut s, &mut b, cmd.clone());
            // ВНУТРИ batch события буферизуются (применятся на Batch(false)),
            // поэтому снапшот на кадре обгоняет буфер — это ожидаемо. Снапшот
            // обязан совпадать с буфером ВНЕ batch (и после его закрытия).
            if s.state().batch_depth == 0 {
                assert_eq!(
                    s.state().text_snapshot,
                    b.text(),
                    "инвариант нарушен после: {:?}",
                    cmd
                );
            }
        }
        assert_eq!(
            b.text(),
            "привет ",
            "слово должно собраться целиком: {:?}",
            b.text()
        );
        assert_eq!(
            s.state().text_snapshot,
            b.text(),
            "снапшот==буфер после batch"
        );
    }

    /// Region, указывающий за границы снапшота (из-за отставания SyncText/чужого
    /// текста), должен кламиться, а не паниковать или ломать буфер.
    #[test]
    fn region_out_of_bounds_clamps_safely() {
        let mut s = svc();
        let mut b = model::ModelTextBuffer::default();

        // Введено "аб", но Gboard шлёт region 0..5 (шире) — сервис не должен падать
        // и preedit должен корректно заменить весь доступный текст.
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("аб".into())),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Region { start: 0, end: 5 }),
        );
        run_with_buffer(
            &mut s,
            &mut b,
            ImeCommand::Ime(ImeCmd::Composing("абв".into())),
        );
        assert_eq!(
            b.text(),
            "абв",
            "Region за границы не должен ломать наращивание: {:?}",
            b.text()
        );
        assert_eq!(s.state().text_snapshot, b.text());
    }

    /// Пустой preedit при отсутствии активной композиции не должен включать
    /// `last_composition_range` повторно и не должен оставлять «призрачный» регион,
    /// который позже заменит не тот сегмент.
    #[test]
    fn repeated_empty_preedit_does_not_leave_ghost_region() {
        let mut s = svc();
        run_with_buffer(
            &mut s,
            &mut model::ModelTextBuffer::default(),
            ImeCommand::Ui(UiCmd::SyncText(String::new())),
        );
        commit(&mut s, "аб");

        // Несколько пустых preedit подряд (Gboard так снимает предикт кадр за кадром).
        composing(&mut s, "");
        composing(&mut s, "");
        assert_eq!(s.state().composition_range, None);
    }

    // ─── Тесты на poll_timeout (loop.rs) — проверяют, что цикл не
    //     блокируется, если в очереди есть IME-команды. ───

    #[test]
    fn poll_timeout_ime_cmds_forces_zero() {
        assert_eq!(
            super::poll_timeout(false, std::time::Duration::from_secs(5), true),
            Some(std::time::Duration::ZERO)
        );
    }

    #[test]
    fn poll_timeout_no_ime_returns_repaint_delay() {
        assert_eq!(
            super::poll_timeout(false, std::time::Duration::from_millis(16), false),
            Some(std::time::Duration::from_millis(16))
        );
    }

    #[test]
    fn poll_timeout_repaint_zero_forces_zero() {
        assert_eq!(
            super::poll_timeout(false, std::time::Duration::ZERO, false),
            Some(std::time::Duration::ZERO)
        );
    }

    #[test]
    fn poll_timeout_large_delay_capped() {
        // Большой repaint_delay обрезается до MAX_POLL_TIMEOUT (100ms).
        assert_eq!(
            super::poll_timeout(false, std::time::Duration::from_secs(4000), false),
            Some(std::time::Duration::from_millis(100))
        );
    }

    #[test]
    fn poll_timeout_ime_overrides_large_delay() {
        assert_eq!(
            super::poll_timeout(false, std::time::Duration::from_secs(9999), true),
            Some(std::time::Duration::ZERO)
        );
    }

    #[test]
    fn poll_timeout_no_graphics_capped() {
        // Даже без graphics — не None, а capped timeout.
        assert_eq!(
            super::poll_timeout(true, std::time::Duration::ZERO, true),
            Some(std::time::Duration::from_millis(100))
        );
    }
}
