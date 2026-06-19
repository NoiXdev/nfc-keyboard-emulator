#[cfg(target_os = "macos")]
pub fn is_trusted() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

#[cfg(not(target_os = "macos"))]
pub fn is_trusted() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn prompt_or_open() {
    // Zeigt den System-Prompt; öffnet sonst direkt das Privacy-Panel.
    if !macos_accessibility_client::accessibility::application_is_trusted_with_prompt() {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

#[cfg(not(target_os = "macos"))]
pub fn prompt_or_open() {}
