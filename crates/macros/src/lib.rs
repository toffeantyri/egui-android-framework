//! Процедурные макросы для egui-android-framework.
//!
//! # Макросы
//!
//! - `#[derive(Component)]` — derive-макрос, генерирует `PersistentState`
//!   для структуры-компонента. Сохраняемые поля указываются через
//!   helper-атрибут `#[persistent_fields(...)]`.
//!
//! - `#[derive(ComponentNode)]` — derive-макрос, генерирует `ComponentNode`
//!   для структуры, реализующей `Component`. Тип сообщения указывается
//!   через атрибут `#[component_message(MsgType)]`.
//!
//!   Если структура также использует `#[derive(Component)]` с
//!   `#[persistent_fields(...)]`, макрос генерирует `save_state`/`restore_state`
//!   через PersistentState. Иначе — `save_state = None`.
//!
//! Заменяет blanket-impl из `component_node.rs`.
//!
//! # Примеры
//!
//! ```ignore
//! use egui_android_macros::{Component, ComponentNode};
//!
//! // Компонент без сохранения состояния:
//! #[derive(ComponentNode)]
//! #[component_message(RootMsg)]
//! struct HomeScreen;
//!
//! // Компонент с сохранением состояния:
//! #[derive(Component, ComponentNode)]
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

/// Проверяет, есть ли `#[persistent_fields(...)]` на структуре.
fn has_persistent_fields(attrs: &[syn::Attribute]) -> bool {
    !parse_persistent_fields(attrs).is_empty()
}

/// Derive-макрос `Component` — генерирует `PersistentState` для структуры.
///
/// Сохраняемые поля указываются через `#[persistent_fields(...)]`
/// на той же структуре.
#[proc_macro_derive(Component, attributes(persistent_fields))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let saved_state_name = format_ident!("__{}PersistentState", name);

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "Component поддерживает только struct с именованными полями",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "Component поддерживает только struct")
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
/// Если структура также использует `#[derive(Component)]` с
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
/// #[derive(Component, ComponentNode)]
/// #[persistent_fields(counter)]
/// #[component_message(MyMsg)]
/// struct StatefulScreen { counter: i32 }
/// ```
#[proc_macro_derive(ComponentNode, attributes(persistent_fields, component_message))]
pub fn derive_component_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let msg_type = match parse_component_message(&input.attrs) {
        Some(t) => t,
        None => {
            return syn::Error::new_spanned(
                &name,
                "ComponentNode требует указания типа сообщения через #[component_message(MsgType)]",
            )
            .to_compile_error()
            .into();
        }
    };

    // Проверяем, есть ли persistent_fields на этой структуре
    let has_persistent = has_persistent_fields(&input.attrs);

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
            fn render(&self, ui: &mut ::egui_android_framework::core::UiWrapper, dispatch: &::egui_android_framework::runtime::DynDispatcher) {
                let typed = dispatch.wrap::<#msg_type>();
                ::egui_android_framework::core::Component::render(self, ui, &typed);
            }

            fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>) {
                if let Ok(typed) = msg.downcast::<#msg_type>() {
                    ::egui_android_framework::core::Component::handle(self, *typed);
                } else {
                    log::error!(
                        "ComponentNode::handle_dyn: ошибка типа сообщения — ожидался {}, получен неизвестный тип",
                        std::any::type_name::<#msg_type>()
                    );
                }
            }

            fn handle_back(&mut self) -> bool {
                false
            }

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
