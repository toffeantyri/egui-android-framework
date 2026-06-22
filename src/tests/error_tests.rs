#[test]
fn test_error_display() {
    let error = crate::AppError::Egl("test".to_string());
    assert_eq!(format!("{}", error), "EGL error: test");
}
