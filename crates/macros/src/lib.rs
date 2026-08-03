//! Процедурные макросы для egui-android-framework.
//!
//! # Макросы
//!
//! - `#[derive(PersistentState)]` — derive-макрос, генерирует `PersistentState`
//!   для структуры-компонента. Сохраняемые поля указываются через
//!   helper-атрибут `#[persistent_fields(...)]`.
//!
//! - `#[derive(ComponentNode)]` — derive-макрос, генерирует `ComponentNode`
//!   для структуры, реализующей `Component`. Тип сообщения указывается
//!   через атрибут `#[component_message(MsgType)]`.
//!
//!   Если структура также использует `#[derive(PersistentState)]` с
//!   `#[persistent_fields(...)]`, макрос генерирует `save_state`/`restore_state`
//!   через PersistentState. Иначе — `save_state = None`.
//!
//! Заменяет blanket-impl из `component_node.rs`.
//!
//! # Кастомная обработка Back
//!
//! По умолчанию `handle_dyn` считает все сообщения обычными (не навигационными)
//! и возвращает `None`, а `handle_back` возвращает `Propagate`. Это подходит
//! экранам, где Back обрабатывает `ChildStack` стандартно (pop).
//!
//! Если у экрана **есть свой вариант сообщения «назад»** и кастомная логика Back,
//! используются два helper-атрибута:
//!
//! - `#[back_message(Enum::Variant)]` — какой вариант сообщения является «назад».
//!   Макрос сгенерирует `handle_dyn`, который для этого варианта вернёт
//!   `Some(BackAction::Propagate)` (навигация поднимается в `ChildStack::on_back`),
//!   а для остальных — `None`.
//!
//! - `#[back_handler(method_name)]` — имя метода на структуре, реализующего
//!   кастомную логику Back. Макрос сгенерирует `handle_back`, делегирующий в
//!   `self.<method_name>(ctx)`. Сигнатура метода:
//!   `fn <method_name>(&mut self, ctx: &mut ComponentContext) -> BackAction`.
//!
//! Два атрибута идут вместе: `#[back_message]` указывает навигационное сообщение,
//! `#[back_handler]` — как его обработать.
//!
//! ```ignore
//! use egui_android_macros::{PersistentState, ComponentNode};
//! use egui_android_framework::core::{BackAction, ComponentContext};
//!
//! #[derive(PersistentState, ComponentNode)]
//! #[persistent_fields(counter)]
//! #[component_message(StateScreenMsg)]
//! #[back_message(StateScreenMsg::Back)]
//! #[back_handler(on_back)]
//! struct CounterScreen { counter: i32 }
//!
//! impl CounterScreen {
//!     fn on_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
//!         self.counter = 0;
//!         BackAction::Pop
//!     }
//! }
//! ```
//!
//! **Когда макрос НЕ подходит** (нужен ручной `impl ComponentNode`):
//!
//! 1. Экран владеет внутренним `ChildStack` и рекурсивно обрабатывает Back
//!    (например, вложенная навигация) — `handle_dyn` и `handle_back` требуют
//!    доступа к внутреннему стеку.
//! 2. Логика Back зависит от типа нажатия (рисованная vs платформенная кнопка).
//! 3. Back требует диспатча/команд в data layer, а не только мутацию `self`.
//!
//! # Примеры
//!
//! ```ignore
//! use egui_android_macros::{PersistentState, ComponentNode};
//!
//! // Компонент без сохранения состояния и без кастомного Back:
//! #[derive(ComponentNode)]
//! #[component_message(RootMsg)]
//! struct HomeScreen;
//!
//! // Компонент с сохранением состояния:
//! #[derive(PersistentState, ComponentNode)]
//! #[persistent_fields(counter)]
//! #[component_message(StateScreenMsg)]
//! struct StateScreen { counter: i32 }
//! ```
//!
//! В фабрике обёртка не нужна:
//! ```ignore
//! Route::State => Box::new(StateScreen::new())
//! ```

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Fields};

/// Перечисление имён сохраняемых полей из `#[persistent_fields(field1, field2)]`.
fn parse_persistent_fields(attrs: &[syn::Attribute]) -> Vec<String> {
    for attr in attrs {
        if attr.path().is_ident("persistent_fields") {
            if let syn::Meta::List(list) = &attr.meta {
                return list
                    .parse_args_with(
                        syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated,
                    )
                    .map(|idents| idents.iter().map(|i| i.to_string()).collect())
                    .unwrap_or_default();
            }
        }
    }
    vec![]
}

/// Извлекает тип сообщения из `#[component_message(MsgType)]`.
fn parse_component_message(attrs: &[syn::Attribute]) -> Option<syn::Type> {
    for attr in attrs {
        if attr.path().is_ident("component_message") {
            if let syn::Meta::List(list) = &attr.meta {
                return list.parse_args::<syn::Type>().ok();
            }
        }
    }
    None
}

/// Извлекает паттерн-вариант Back из `#[back_message(Enum::Variant)]`.
///
/// Например, `#[back_message(StateScreenMsg::Back)]` вернёт `StateScreenMsg::Back`.
/// Используется в `handle_dyn`, чтобы отличить Back от обычных сообщений.
fn parse_back_message(attrs: &[syn::Attribute]) -> Option<syn::Path> {
    for attr in attrs {
        if attr.path().is_ident("back_message") {
            if let syn::Meta::List(list) = &attr.meta {
                return list.parse_args::<syn::PatPath>().ok().map(|pp| pp.path);
            }
        }
    }
    None
}

/// Извлекает имя метода-обработчика Back из `#[back_handler(method_name)]`.
///
/// Например, `#[back_handler(on_back)]` — макрос сгенерирует `handle_back`,
/// делегирующий в `self.on_back(ctx)`.
fn parse_back_handler(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    for attr in attrs {
        if attr.path().is_ident("back_handler") {
            if let syn::Meta::List(list) = &attr.meta {
                return list.parse_args::<syn::Ident>().ok();
            }
        }
    }
    None
}

/// Проверяет, есть ли `#[persistent_fields(...)]` на структуре.
fn has_persistent_fields(attrs: &[syn::Attribute]) -> bool {
    !parse_persistent_fields(attrs).is_empty()
}

/// Derive-макрос `PersistentState` — генерирует `impl PersistentState` для структуры.
///
/// Сохраняемые поля указываются через `#[persistent_fields(...)]`
/// на той же структуре.
#[proc_macro_derive(PersistentState, attributes(persistent_fields))]
pub fn derive_persistent_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let saved_state_name = format_ident!("__{}PersistentState", name);

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "PersistentState поддерживает только struct с именованными полями",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "PersistentState поддерживает только struct")
                .to_compile_error()
                .into();
        }
    };

    // Парсим #[persistent_fields(...)] из атрибутов структуры
    let persistent_field_names = parse_persistent_fields(&input.attrs);

    // Фильтруем поля, которые есть в списке persistent_fields
    let persistent_fields: Vec<_> = fields
        .iter()
        .filter(|f| {
            f.ident
                .as_ref()
                .map(|id| persistent_field_names.contains(&id.to_string()))
                .unwrap_or(false)
        })
        .collect();

    if persistent_fields.is_empty() {
        return syn::Error::new_spanned(
            &input,
            "Не указаны сохраняемые поля. Добавьте #[persistent_fields(field1, field2)]",
        )
        .to_compile_error()
        .into();
    }

    let pf_names: Vec<_> = persistent_fields
        .iter()
        .map(|f| f.ident.as_ref().unwrap())
        .collect();
    let pf_types: Vec<_> = persistent_fields.iter().map(|f| &f.ty).collect();

    // Структура SavedState
    let field_defs: Vec<_> = pf_names
        .iter()
        .zip(pf_types.iter())
        .map(|(name, ty)| quote! { pub #name: #ty })
        .collect();

    let saved_state_def = quote! {
        #[doc(hidden)]
        #[derive(::serde::Serialize, ::serde::Deserialize, Clone, Debug)]
        #[allow(non_camel_case_types)]
        pub struct #saved_state_name {
            #(#field_defs,)*
        }
    };

    // Save: self.field.clone()
    let saves: Vec<_> = pf_names
        .iter()
        .map(|name| quote! { #name: self.#name.clone() })
        .collect();

    // Restore: self.field = state.field;
    let restores: Vec<_> = pf_names
        .iter()
        .map(|name| quote! { self.#name = state.#name; })
        .collect();

    let persistent_state_impl = quote! {
        impl ::egui_android_framework::core::PersistentState for #name {
            type State = #saved_state_name;

            fn save(&self) -> Self::State {
                #saved_state_name {
                    #(#saves,)*
                }
            }

            fn restore(&mut self, state: Self::State) {
                #(#restores)*
            }
        }
    };

    let expanded = quote! {
        #saved_state_def
        #persistent_state_impl
    };

    TokenStream::from(expanded)
}

/// Derive-макрос `ComponentNode` — генерирует конкретный impl `ComponentNode`.
///
/// Заменяет blanket-impl из `component_node.rs`. Требует указания типа сообщения
/// через `#[component_message(MsgType)]`.
///
/// Если структура также использует `#[derive(PersistentState)]` с
/// `#[persistent_fields(...)]`, макрос генерирует `save_state`/`restore_state`
/// через PersistentState. Иначе — `save_state = None`.
///
/// # Пример
///
/// ```ignore
/// #[derive(ComponentNode)]
/// #[component_message(MyMsg)]
/// struct MyScreen;
///
/// #[derive(PersistentState, ComponentNode)]
/// #[persistent_fields(counter)]
/// #[component_message(MyMsg)]
/// struct StatefulScreen { counter: i32 }
/// ```
#[proc_macro_derive(
    ComponentNode,
    attributes(persistent_fields, component_message, back_message, back_handler)
)]
pub fn derive_component_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let msg_type = match parse_component_message(&input.attrs) {
        Some(t) => t,
        None => {
            return syn::Error::new_spanned(
                name,
                "ComponentNode требует указания типа сообщения через #[component_message(MsgType)]",
            )
            .to_compile_error()
            .into();
        }
    };

    // Проверяем, есть ли persistent_fields на этой структуре
    let has_persistent = has_persistent_fields(&input.attrs);

    // ─── Back-обработка: #[back_message(Enum::Variant)] и #[back_handler(method)] ───
    //
    // #back_message — какой вариант сообщения является «назад».
    //   Позволяет handle_dyn отделить Back от обычных сообщений.
    //
    // #back_handler — метод на структуре, реализующий кастомную логику Back.
    //   Заменяет сгенерированный handle_back на делегирование в self.<method>(ctx).
    let back_message = parse_back_message(&input.attrs);
    let back_handler = parse_back_handler(&input.attrs);

    // handle_dyn: если downcast удался и сообщение совпадает с back_message —
    // возвращаем Some(Propagate) (навигация передаётся в ChildStack::on_back).
    // Иначе — обычное сообщение, навигация не требуется (None).
    let handle_dyn_body = if let Some(back_pat) = &back_message {
        quote! {
            if let Ok(typed) = msg.downcast::<#msg_type>() {
                // Back-вариант — навигационное сообщение, поднимаем наверх.
                // app.rs вызовет on_back() → ChildStack::on_back() → handle_back().
                if matches!(&*typed, #back_pat) {
                    return Some(::egui_android_framework::core::BackAction::Propagate);
                }
                // Обычное сообщение обработано — навигация не требуется.
                ::egui_android_framework::core::Component::handle(self, *typed, ctx);
                None
            } else {
                log::error!(
                    "ComponentNode::handle_dyn: ошибка типа сообщения — ожидался {}, получен неизвестный тип",
                    std::any::type_name::<#msg_type>()
                );
                Some(::egui_android_framework::core::BackAction::Propagate)
            }
        }
    } else {
        quote! {
            if let Ok(typed) = msg.downcast::<#msg_type>() {
                ::egui_android_framework::core::Component::handle(self, *typed, ctx);
                // Обычное (не-навигационное) сообщение обработано — навигация не требуется.
                None
            } else {
                log::error!(
                    "ComponentNode::handle_dyn: ошибка типа сообщения — ожидался {}, получен неизвестный тип",
                    std::any::type_name::<#msg_type>()
                );
                Some(::egui_android_framework::core::BackAction::Propagate)
            }
        }
    };

    // handle_back: если указан back_handler — делегируем в self.<method>(ctx),
    // полностью заменяя реализацию. Иначе — дефолт Propagate.
    let handle_back = if let Some(method) = &back_handler {
        quote! {
            fn handle_back(
                &mut self,
                ctx: &mut ::egui_android_framework::core::ComponentContext,
            ) -> ::egui_android_framework::core::BackAction {
                self.#method(ctx)
            }
        }
    } else {
        quote! {
            fn handle_back(
                &mut self,
                _ctx: &mut ::egui_android_framework::core::ComponentContext,
            ) -> ::egui_android_framework::core::BackAction {
                ::egui_android_framework::core::BackAction::Propagate
            }
        }
    };

    let save_restore = if has_persistent {
        quote! {
            fn save_state(&self) -> Option<Box<dyn std::any::Any + Send>> {
                ::egui_android_framework::core::PersistentState::save_to_boxed(self)
            }

            fn restore_state(&mut self, state: Box<dyn std::any::Any + Send>) {
                ::egui_android_framework::core::PersistentState::restore_from_boxed(self, state);
            }
        }
    } else {
        quote! {
            fn save_state(&self) -> Option<Box<dyn std::any::Any + Send>> {
                None
            }

            fn restore_state(&mut self, _state: Box<dyn std::any::Any + Send>) {}
        }
    };

    let expanded = quote! {
        impl ::egui_android_framework::core::ComponentNode for #name {
            fn render(
                &self,
                ui: &mut ::egui_android_framework::core::UiWrapper,
                dispatch: &::egui_android_framework::runtime::DynDispatcher,
                ctx: &::egui_android_framework::core::ComponentContext,
            ) {
                let typed = dispatch.wrap::<#msg_type>();
                ::egui_android_framework::core::Component::render(self, ui, &typed, ctx);
            }

            fn handle_dyn(
                &mut self,
                msg: Box<dyn std::any::Any + Send>,
                ctx: &mut ::egui_android_framework::core::ComponentContext,
            ) -> Option<::egui_android_framework::core::BackAction> {
                #handle_dyn_body
            }

            #handle_back

            #save_restore

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };

    TokenStream::from(expanded)
}

/// Оставляет оригинальный `#[component]` для обратной совместимости.
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
