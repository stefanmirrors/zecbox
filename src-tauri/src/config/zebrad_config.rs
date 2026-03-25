use std::fs;
use std::path::{Path, PathBuf};

/// Generate the contents of zebrad.toml for the given data directory.
/// When `shield_mode` is true, restricts network to localhost and adds .onion seed peers.
pub fn generate_zebrad_toml(data_dir: &Path, shield_mode: bool) -> String {
    let cache_dir = data_dir.join("zebra");

    if shield_mode {
        format!(
            r#"[consensus]
checkpoint_sync = true

[mempool]
eviction_memory_time = "1h"

[network]
network = "Mainnet"
listen_addr = "127.0.0.1:8233"
initial_mainnet_peers = []

[rpc]
listen_addr = "127.0.0.1:8232"

[state]
cache_dir = "{cache_dir}"
"#,
            cache_dir = cache_dir.display()
        )
    } else {
        format!(
            r#"[consensus]
checkpoint_sync = true

[mempool]
eviction_memory_time = "1h"

[network]
network = "Mainnet"
listen_addr = "0.0.0.0:8233"

[rpc]
listen_addr = "127.0.0.1:8232"

[state]
cache_dir = "{cache_dir}"
"#,
            cache_dir = cache_dir.display()
        )
    }
}

/// Write zebrad.toml to `{data_dir}/config/zebrad.toml`, creating directories as needed.
/// Returns the path to the written config file.
pub fn write_zebrad_config(
    data_dir: &Path,
    shield_mode: bool,
) -> Result<PathBuf, std::io::Error> {
    let config_dir = data_dir.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(data_dir.join("zebra"))?;
    fs::create_dir_all(data_dir.join("logs"))?;
    fs::create_dir_all(data_dir.join("zaino"))?;

    let config_path = config_dir.join("zebrad.toml");
    let contents = generate_zebrad_toml(data_dir, shield_mode);
    fs::write(&config_path, contents)?;

    Ok(config_path)
}
