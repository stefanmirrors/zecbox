use std::fs;
use std::path::{Path, PathBuf};

/// Generate the contents of zebrad.toml for the given data directory.
/// When `stealth_mode` is true, restricts listen to localhost.
/// When `proxy_vps_ip` is set, adds external_addr so peers see the VPS IP.
pub fn generate_zebrad_toml(data_dir: &Path, stealth_mode: bool, proxy_vps_ip: Option<&str>) -> String {
    let cache_dir = data_dir.join("zebra");

    // DNS seeders for peer discovery — zebrad needs these to find the network.
    // In Stealth Mode, PF firewall transparently routes connections through Tor,
    // so zebrad uses the same seeders but traffic goes through the SOCKS proxy.
    let dns_seeders = r#"initial_mainnet_peers = [
    "dnsseed.z.cash:8233",
    "dnsseed.str4d.xyz:8233",
    "mainnet.seeder.zfnd.org:8233",
    "mainnet.is.yolo.money:8233",
]"#;

    let listen_addr = if stealth_mode {
        "127.0.0.1:8233"
    } else {
        "0.0.0.0:8233"
    };

    let external_addr_line = match proxy_vps_ip {
        Some(ip) => format!("\nexternal_addr = \"{}:8233\"", ip),
        None => String::new(),
    };

    format!(
        r#"[consensus]
checkpoint_sync = true

[mempool]
eviction_memory_time = "1h"

[network]
network = "Mainnet"
listen_addr = "{listen_addr}"{external_addr}
{dns_seeders}

[rpc]
listen_addr = "127.0.0.1:8232"
enable_cookie_auth = false

[state]
cache_dir = "{cache_dir}"
"#,
        listen_addr = listen_addr,
        external_addr = external_addr_line,
        dns_seeders = dns_seeders,
        cache_dir = super::toml_path(&cache_dir)
    )
}

/// Write zebrad.toml to `{data_dir}/config/zebrad.toml`, creating directories as needed.
/// Returns the path to the written config file.
pub fn write_zebrad_config(
    data_dir: &Path,
    stealth_mode: bool,
    proxy_vps_ip: Option<&str>,
) -> Result<PathBuf, std::io::Error> {
    let config_dir = data_dir.join("config");
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(data_dir.join("zebra"))?;
    fs::create_dir_all(data_dir.join("logs"))?;
    fs::create_dir_all(data_dir.join("zaino"))?;

    let config_path = config_dir.join("zebrad.toml");
    let contents = generate_zebrad_toml(data_dir, stealth_mode, proxy_vps_ip);
    fs::write(&config_path, contents)?;

    Ok(config_path)
}
