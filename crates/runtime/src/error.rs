use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Ошибка инициализации графики: {0}")]
    GraphicInit(String),

    #[error("Ошибка создания поверхности: {0}")]
    SurfaceCreation(String),

    #[error("Ошибка жизненного цикла: {0}")]
    Lifecycle(String),

    #[error("Ошибка EGL: {0}")]
    Egl(String),
}
