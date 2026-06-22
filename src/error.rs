use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Graphic initialization failed: {0}")]
    GraphicInit(String),

    #[error("Surface creation failed: {0}")]
    SurfaceCreation(String),

    #[error("Lifecycle error: {0}")]
    Lifecycle(String),

    #[error("EGL error: {0}")]
    Egl(String),
}
