pub mod egl_backend;
pub mod input;
pub mod run;
pub mod run2;

#[cfg(target_os = "android")]
pub use run::run;
#[cfg(target_os = "android")]
pub use run2::run2;
