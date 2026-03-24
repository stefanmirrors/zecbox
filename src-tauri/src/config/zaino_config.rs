use std::fs;
use std::path::{Path, PathBuf};

/// Generate the contents of zaino.toml.
/// Points Zaino at zebrad RPC on localhost:8232 and listens for gRPC on 0.0.0.0:9067.
pub fn generate_zaino_toml(data_dir: &Path) -> String {
    let index_dir = data_dir.join("zaino");

    format!(
        r#"[rpc]
zebrad_uri = "http://127.0.0.1:8232"

[grpc]
listen_addr = "0.0.0.0:9067"

[state]
index_dir = "{index_dir}"
"#,
        index_dir = index_dir.display()
    )
}

/// Write zaino.toml to `{data_dir}/config/zaino.toml`, creating directories as needed.
/// Returns the path to the written config file.
pub fn write_zaino_config(data_dir: &Path) -> Result<PathBuf, std::io::Error> {
    let config_dir = data_dir.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(data_dir.join("zaino"))?;
    fs::create_dir_all(data_dir.join("logs"))?;

    let config_path = config_dir.join("zaino.toml");
    let contents = generate_zaino_toml(data_dir);
    fs::write(&config_path, contents)?;

    Ok(config_path)
}
