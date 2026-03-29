//! Configuration file generation for zebrad, Zaino, and ZecBox app settings.

pub mod app_config;
pub mod proxy_config;
pub mod zaino_config;
pub mod zebrad_config;

use std::path::Path;

/// Convert a path to a TOML-safe string.
/// Strips the Windows `\\?\` extended-length prefix and converts backslashes
/// to forward slashes so TOML parsers don't interpret them as escape sequences.
pub fn toml_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/")
}
