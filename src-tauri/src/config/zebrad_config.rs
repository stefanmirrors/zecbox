use std::fs;
use std::path::{Path, PathBuf};

/// Generate the contents of zebrad.toml for the given data directory.
/// `socks_proxy` is reserved for Phase 5 (Shield Mode).
pub fn generate_zebrad_toml(data_dir: &Path, _socks_proxy: Option<&str>) -> String {
    let cache_dir = data_dir.join("zebra");
    format!(
        r#"[consensus]
checkpoint_sync = true

[mempool]
eviction_memory_time = "1h"

[mining]
miner_address = ""

[network]
network = "Mainnet"
listen_addr = "0.0.0.0:8233"

[rpc]
listen_addr = "127.0.0.1:8232"

[state]
cache_dir = "{cache_dir}"

[tracing]
progress_bar = "never"
"#,
        cache_dir = cache_dir.display()
    )
}

/// Write zebrad.toml to `{data_dir}/config/zebrad.toml`, creating directories as needed.
/// Returns the path to the written config file.
pub fn write_zebrad_config(
    data_dir: &Path,
    socks_proxy: Option<&str>,
) -> Result<PathBuf, std::io::Error> {
    let config_dir = data_dir.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(data_dir.join("zebra"))?;
    fs::create_dir_all(data_dir.join("logs"))?;
    fs::create_dir_all(data_dir.join("zaino"))?;

    let config_path = config_dir.join("zebrad.toml");
    let contents = generate_zebrad_toml(data_dir, socks_proxy);
    fs::write(&config_path, contents)?;

    Ok(config_path)
}
