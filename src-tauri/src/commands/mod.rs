//! Tauri command handlers exposed to the frontend via invoke().

pub mod logs;
pub mod node;
pub mod onboarding;
pub mod shield;
pub mod storage;
pub mod updates;
pub mod wallet;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! ZecBox is running.", name)
}
