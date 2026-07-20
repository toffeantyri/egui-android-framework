//! Процедурные макросы для egui-android-framework.
//!
//! # Макросы
//!
//! - `#[derive(Component)]` — derive-макрос, генерирует `PersistentState`
//!   для структуры-компонента. Сохраняемые поля указываются через
//!   helper-атрибут `#[persistent_fields(...)]`.
//!
//! # Аналогия с Decompose
//!
//! В Decompose компонент регистрирует `stateKeeper<T>(init)` для сохранения состояния.
//! У нас `#[derive(Component)]` с `#[persistent_fields(...)]` генерирует `PersistentState`.
//!
//! # Пример
//!
//! ```ignore
//! use egui_android_macros::Component;
//!
//! #[derive(Component)]
//! #[persistent_fields(counter, name)]
//! struct CounterScreen {
//!     counter: i32,
//!     name: String,
//!     expanded: bool,  // не сохраняется
//! }
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
        impl ::egui_android_core::PersistentState for #name {
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

/// Оставляет оригинальный `#[component]` для обратной совместимости.
/// В будущем будет расширен до генерации `ComponentNode`.
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Пока заглушка — просто возвращает item без изменений
    // В будущем будет генерировать ComponentNode для компонентов
    // которые не могут использовать blanket-impl
    item
}
