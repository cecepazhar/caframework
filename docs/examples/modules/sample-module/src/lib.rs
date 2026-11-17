//! Sample product module reference implementation.
//! Declares sample services without importing undeclared modules.

pub fn sample_service_action() -> Result<String, String> {
    Ok("sample_module action executed".to_string())
}
