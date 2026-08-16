//! Автономный тестовый прогон IME-сервиса — запускается при старте showcase
//! на Android-устройстве и выводит результат в logcat (тег: egui-showcase).
//!
//! Проверяет:
//! - Сборку слова «привет» чистыми командами (сервис)
//! - translate_legacy (JNI-конвейер без JNI)
//! - poll_timeout (не блокируемся при IME-командах)
//! - batch+commit/delete не стирают накопленное (reset_composition)
//!
//! ПРОГОН: все тесты запланированы как fn() -> Result<(), String>,
//! что позволяет вывести суммарно passed/failed без паники при первом FAIL.

#[cfg(target_os = "android")]
pub fn run_ime_tests() {
    use egui_android_platform_android::ime_service::{
        poll_timeout, translate_legacy, DefaultImeService, ImeCmd, ImeCommand, ImeEvent,
        ImeService, UiCmd,
    };
    use std::time::Duration;

    // ── тестовые функции ──────────────────────────────────────────
    type TestFn = fn() -> Result<(), String>;

    /// Прогнать IME-команду через сервис и применить события к модельному буферу.
    /// НЕ синхронизируем снапшот (как в рантайме: loop.rs не зовёт SyncText
    /// каждый кадр) — сервис сам ведёт снапшот, и он должен совпадать с буфером,
    /// если события применяются синхронно.
    fn drive(
        s: &mut DefaultImeService,
        buf: &mut String,
        cursor: &mut usize,
        cmd: ImeCommand,
    ) -> Result<(), String> {
        let _ = drive_and_return(s, buf, cursor, cmd)?;
        Ok(())
    }

    /// Как `drive`, но позволяет проверить события, выгруженные в конце batch.
    fn drive_and_return(
        s: &mut DefaultImeService,
        buf: &mut String,
        cursor: &mut usize,
        cmd: ImeCommand,
    ) -> Result<Vec<ImeEvent>, String> {
        let evs = s.apply(cmd);
        for ev in &evs {
            apply_to_string(buf, cursor, ev);
        }
        Ok(evs)
    }

    fn test_word_privet_assembles() -> Result<(), String> {
        let mut s = DefaultImeService::default();
        let mut buf = String::new();
        let mut cursor = 0usize;

        // Реальный набор «привет»: предикт растёт непрерывно, затем снимается
        // и подтверждается пробелом. Никакого SyncText между командами — сервис
        // сам ведёт снапшот, который должен совпадать с буфером.
        for text in ["п", "пр", "при", "прив", "приве", "привет"] {
            drive(
                &mut s,
                &mut buf,
                &mut cursor,
                ImeCommand::Ime(ImeCmd::Composing(text.into())),
            )?;
        }
        // Снимаем предикт и подтверждаем пробелом.
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Composing(String::new())),
        )?;
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Commit(" ".into())),
        )?;

        if buf != "привет " {
            return Err(format!(
                "слово не собралось: ожидалось «привет », получено {:?}",
                buf
            ));
        }
        if s.state().text_snapshot != buf {
            return Err(format!(
                "снапшот разошёлся с буфером: snp={:?} buf={:?}",
                s.state().text_snapshot,
                buf
            ));
        }
        Ok(())
    }

    fn test_translate_legacy_privet() -> Result<(), String> {
        use egui_android_platform_android::ime_logic::ImeCmd as Leg;
        let mut s = DefaultImeService::default();
        let mut buf = String::new();
        let mut cursor = 0usize;

        let cmds: &[Leg] = &[
            Leg::Composing("п".into()),
            Leg::Composing("пр".into()),
            Leg::Composing("при".into()),
            Leg::Composing("прив".into()),
            Leg::Composing("приве".into()),
            Leg::Composing("привет".into()),
        ];
        for cmd in cmds {
            if let Some(ic) = translate_legacy(cmd) {
                for ev in s.apply(ic) {
                    apply_to_string(&mut buf, &mut cursor, &ev);
                }
            }
        }
        if buf != "привет" {
            return Err(format!(
                "translate_legacy: ожидалось «привет», получено {:?}",
                buf
            ));
        }
        Ok(())
    }

    fn test_batch_delete_keeps_events() -> Result<(), String> {
        let mut s = DefaultImeService::default();
        s.apply(ImeCommand::Ime(ImeCmd::Batch(true)));
        s.apply(ImeCommand::Ime(ImeCmd::Composing("п".into())));
        s.apply(ImeCommand::Ime(ImeCmd::Composing("пр".into())));
        s.apply(ImeCommand::Ime(ImeCmd::Composing("при".into())));
        s.apply(ImeCommand::Ime(ImeCmd::DeleteSurrounding {
            before: 1,
            after: 0,
        }));
        let evs = s.apply(ImeCommand::Ime(ImeCmd::Batch(false)));
        let text_ops: usize = evs
            .iter()
            .filter(|e| matches!(e, ImeEvent::Insert(_) | ImeEvent::Replace { .. }))
            .count();
        if text_ops < 3 {
            return Err(format!(
                "batch потерял события: всего {} текстовых операций (>=3 ожидалось): {:?}",
                text_ops, evs
            ));
        }
        Ok(())
    }

    fn test_batch_commit_keeps_events() -> Result<(), String> {
        let mut s = DefaultImeService::default();
        let mut buf = String::new();
        let mut cursor = 0usize;

        // Gboard подтверждает preedit: Batch + нарастить «пр» + Commit«и».
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Batch(true)),
        )?;
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Composing("п".into())),
        )?;
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Composing("пр".into())),
        )?;
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Composing("при".into())),
        )?;
        // Commit заменяет preedit «при» на «и».
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Commit("и".into())),
        )?;
        let rest = drive_and_return(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Batch(false)),
        )?;

        // Не должно быть дубля «прии» — Commit должен заменить preedit.
        if buf.contains("прии") {
            return Err(format!("Commit не заменил preedit, дубль: {:?}", buf));
        }
        if buf != "и" {
            return Err(format!(
                "Commit внутри batch должен заменить preedit на «и», получено {:?} (oперации {:?})",
                buf, rest
            ));
        }
        Ok(())
    }

    fn test_poll_timeout_ime_commands() -> Result<(), String> {
        if poll_timeout(false, Duration::from_secs(5), true) != Some(Duration::ZERO) {
            return Err("poll_timeout с IME-командами не дал ZERO".into());
        }
        if poll_timeout(false, Duration::from_millis(100), false)
            != Some(Duration::from_millis(100))
        {
            return Err("poll_timeout без IME-команд должен сохранять repaint_delay".into());
        }
        if poll_timeout(false, Duration::from_secs(9999), true) != Some(Duration::ZERO) {
            return Err(
                "poll_timeout с IME-командами и большим repaint_delay должен дать ZERO".into(),
            );
        }
        Ok(())
    }

    fn test_region_replace_avoids_duplicate() -> Result<(), String> {
        let mut s = DefaultImeService::default();
        let mut buf = String::new();
        let mut cursor = 0usize;

        let seq: [ImeCmd; 4] = [
            ImeCmd::Composing("п".into()),
            ImeCmd::Region { start: 0, end: 1 },
            ImeCmd::Composing("пр".into()),
            ImeCmd::Composing("при".into()),
        ];
        for cmd in seq {
            let evs = s.apply(ImeCommand::Ime(cmd));
            for ev in evs {
                apply_to_string(&mut buf, &mut cursor, &ev);
            }
        }
        if buf.contains("пп") {
            return Err(format!("Region+Replace дал дубль «пп»: {:?}", buf));
        }
        if buf != "при" {
            return Err(format!("Region+Replace не собрал «при»: {:?}", buf));
        }
        Ok(())
    }

    /// РЕГРЕССИЯ: при активной IME-композиции на Android должны сохраняться
    /// мигающая каретка и выделение (не пропадать). Первопричина — egui переводит
    /// непустой `Preedit` в режим ImeComposition, в котором каретка/выделение не
    /// рисуются. Это чинится через `Visuals::ime_composition.legacy_visuals = true`
    /// на Android. Проверяем фактическое значение стиля на САМОМ УСТРОЙСТВЕ.
    fn test_ime_caret_visible_uses_legacy_visuals() -> Result<(), String> {
        let dark = egui::Visuals::dark();
        let light = egui::Visuals::light();
        for (name, v) in [("dark", dark), ("light", light)] {
            if !v.ime_composition.legacy_visuals {
                return Err(format!(
                    "[]{name}] legacy_visuals = false — при предикте каретка/выделение пропадут"
                ));
            }
        }
        Ok(())
    }

    /// РЕГРЕССИЯ с устройства (реальный лог «привет» → тап в середину → ввод):
    /// после перемещения курсора в середину слова следующий ввод должен
    /// ВСТАВИТЬСЯ в эту позицию (не затереть слово и не уйти в конец).
    ///
    /// Это reducer-половина бага: сервис должен нарастить слово С середины,
    /// сохранив префикс. В рантайме Gboard шлёт `setComposingRegion` от места
    /// каретки (см. лог: `setComposingRegion 10..16 text="ривет "`), сервис
    /// получает `Region`/`Composing` — здесь эмулируем это через SyncText+MoveCursor.
    fn test_insert_mid_word_preserves_prefix() -> Result<(), String> {
        let mut s = DefaultImeService::default();
        // Буфер уже содержит «привет» (реальное поле). `apply_to_string` не
        // заполняет его из `SyncText` — `SyncText`/`MoveCursor` лишь синхронизируют
        // внутреннее состояние сервиса, а буфер инициализируем явно.
        let mut buf = String::from("привет");
        let mut cursor = 0usize;

        // Готовое слово «привет», каретка в середине (char 3).
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ui(UiCmd::SyncText("привет".into())),
        )?;
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ui(UiCmd::MoveCursor(3)),
        )?;

        // Вводим «X» в середину — должен получиться «приXвет».
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Composing("X".into())),
        )?;

        match buf.as_str() {
            "приXвет" => Ok(()),
            "приветX" => Err(format!(
                "ввод ушёл в конец вместо середины: получено {:?}",
                buf
            )),
            "X" => Err(format!("слово затёрто: получено {:?}", buf)),
            other => Err(format!(
                "неожиданный результат вставки в середину: {:?}",
                other
            )),
        }
    }

    /// РЕГРЕССИЯ с устройства: МНОГОСИМВОЛЬНЫЙ набор из середины слова.
    ///
    /// Одиночный `commitText` после тапа в середину уже вставляется в каретку
    /// egui через `Event::Text`. Но если Gboard шлёт `setComposingText` с
    /// несколькими символами (предсказание), сервис должен нарастить слово
    /// ИМЕННО с позиции каретки (после `sync_from_editor_state`), а не с конца
    /// и не затирать префикс. Это главный кейс, где раньше проявлялся рассинхрон
    /// `text_snapshot`.
    fn test_insert_multi_char_mid_word() -> Result<(), String> {
        // Буфер уже содержит «привет» (реальное поле). Далее через `drive`
        // подаём `SyncText`+`MoveCursor(3)`, как это делает `loop.rs` — это
        // синхронизирует и сервис, и курсор буфера с позицией середины.
        let mut s = DefaultImeService::default();
        let mut buf = String::from("привет");
        let mut cursor = 0usize;

        // Каретка в середине (char 3) — эмуляция тапа.
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ui(UiCmd::SyncText("привет".into())),
        )?;
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ui(UiCmd::MoveCursor(3)),
        )?;

        // Gboard начинает предсказание с середины двумя символами.
        drive(
            &mut s,
            &mut buf,
            &mut cursor,
            ImeCommand::Ime(ImeCmd::Composing("XL".into())),
        )?;

        match buf.as_str() {
            "приXLвет" => Ok(()),
            "приветXL" => Err(format!("много-символ ушёл в конец: {:?}", buf)),
            "XL" => Err(format!("много-символ затёр слово: {:?}", buf)),
            other => Err(format!(
                "неожиданный результат много-символьной вставки в середину: {:?}",
                other
            )),
        }
    }

    // ── зарегистрированные тесты ──────────────────────────────────
    let tests: &[(&str, TestFn)] = &[
        ("word_privet_assembles", test_word_privet_assembles),
        ("translate_legacy_privet", test_translate_legacy_privet),
        ("batch_delete_keeps_events", test_batch_delete_keeps_events),
        ("batch_commit_keeps_events", test_batch_commit_keeps_events),
        ("poll_timeout_ime_commands", test_poll_timeout_ime_commands),
        (
            "region_replace_avoids_duplicate",
            test_region_replace_avoids_duplicate,
        ),
        (
            "ime_caret_visible_uses_legacy_visuals",
            test_ime_caret_visible_uses_legacy_visuals,
        ),
        (
            "insert_mid_word_preserves_prefix",
            test_insert_mid_word_preserves_prefix,
        ),
        (
            "insert_multi_char_mid_word",
            test_insert_multi_char_mid_word,
        ),
    ];

    let mut passed: Vec<&str> = Vec::new();
    let mut failed: Vec<(&str, String)> = Vec::new();

    for (name, test_fn) in tests {
        match test_fn() {
            Ok(()) => passed.push(name),
            Err(e) => failed.push((name, e)),
        }
    }

    log::info!("=== IME-ТЕСТЫ НА УСТРОЙСТВЕ ===");
    log::info!("  PASSED ({}/{}): {:?}", passed.len(), tests.len(), passed);
    if !failed.is_empty() {
        log::error!("  FAILED ({}/{}):", failed.len(), tests.len());
        for (name, err) in &failed {
            log::error!("    {} → {}", name, err);
        }
    } else {
        log::info!("  ВСЕ ТЕСТЫ ПРОЙДЕНЫ");
    }
}

/// Применить одно ImeEvent к модельному буферу (String + cursor).
#[cfg(target_os = "android")]
fn apply_to_string(
    buf: &mut String,
    cursor: &mut usize,
    ev: &egui_android_platform_android::ime_service::ImeEvent,
) {
    use egui_android_platform_android::ime_service::ImeEvent;

    fn char_to_byte(s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(s.len())
    }

    match ev {
        ImeEvent::Insert(text) => {
            let pos = (*cursor).min(buf.chars().count());
            let byte_pos = char_to_byte(buf, pos);
            buf.insert_str(byte_pos, text);
            *cursor = pos + text.chars().count();
        }
        ImeEvent::Replace {
            start,
            end,
            replacement,
        } => {
            let s = (*start).min(buf.chars().count());
            let e = (*end).min(buf.chars().count()).max(s);
            let byte_s = char_to_byte(buf, s);
            let byte_e = char_to_byte(buf, e);
            buf.replace_range(byte_s..byte_e, replacement);
            *cursor = s + replacement.chars().count();
        }
        ImeEvent::Delete { before, after } => {
            let c = *cursor;
            let del_before = (*before).min(c);
            let del_after = (*after).min(buf.chars().count().saturating_sub(c));
            let start = c - del_before;
            let end = c + del_after;
            let byte_s = char_to_byte(buf, start);
            let byte_e = char_to_byte(buf, end);
            buf.replace_range(byte_s..byte_e, "");
            *cursor = start;
        }
        ImeEvent::Cursor(idx) => {
            *cursor = (*idx).min(buf.chars().count());
        }
        ImeEvent::Action(_) => {}
    }
}
