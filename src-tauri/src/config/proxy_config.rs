//! Proxy Mode configuration: WireGuard keys, VPS IP, tunnel addressing.
//! Keys are generated using x25519-dalek for Curve25519 key exchange.

use std::path::Path;

use base64::Engine;
use serde::{Deserialize, Serialize};

const PROXY_CONFIG_FILE: &str = "proxy_config.json";

/// WireGuard tunnel addressing
const TUNNEL_IP_HOME: &str = "10.13.37.2/24";
const TUNNEL_IP_VPS: &str = "10.13.37.1/24";
const DEFAULT_WG_PORT: u16 = 51820;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub vps_ip: String,
    pub vps_wg_port: u16,
    pub home_private_key: String,
    pub home_public_key: String,
    pub vps_private_key: String,
    pub vps_public_key: String,
    pub preshared_key: String,
    pub tunnel_ip_home: String,
    pub tunnel_ip_vps: String,
    pub setup_complete: bool,
}

impl ProxyConfig {
    /// Generate a new proxy configuration with fresh WireGuard keys.
    pub fn generate(vps_ip: &str, vps_wg_port: Option<u16>) -> Self {
        let b64 = base64::engine::general_purpose::STANDARD;

        // Generate home (client) keypair
        let home_secret = x25519_dalek::StaticSecret::random_from_rng(rand::thread_rng());
        let home_public = x25519_dalek::PublicKey::from(&home_secret);

        // Generate VPS (server) keypair
        let vps_secret = x25519_dalek::StaticSecret::random_from_rng(rand::thread_rng());
        let vps_public = x25519_dalek::PublicKey::from(&vps_secret);

        // Generate preshared key (32 random bytes)
        let mut psk = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut psk);

        Self {
            vps_ip: vps_ip.to_string(),
            vps_wg_port: vps_wg_port.unwrap_or(DEFAULT_WG_PORT),
            home_private_key: b64.encode(home_secret.to_bytes()),
            home_public_key: b64.encode(home_public.as_bytes()),
            vps_private_key: b64.encode(vps_secret.to_bytes()),
            vps_public_key: b64.encode(vps_public.as_bytes()),
            preshared_key: b64.encode(psk),
            tunnel_ip_home: TUNNEL_IP_HOME.to_string(),
            tunnel_ip_vps: TUNNEL_IP_VPS.to_string(),
            setup_complete: false,
        }
    }

    /// Generate the WireGuard client config for the home side (boringtun).
    pub fn generate_home_wg_conf(&self) -> String {
        format!(
            r#"[Interface]
PrivateKey = {home_private}
Address = {tunnel_home}

[Peer]
PublicKey = {vps_public}
PresharedKey = {psk}
Endpoint = {vps_ip}:{vps_port}
AllowedIPs = {tunnel_vps_ip}/32
PersistentKeepalive = 25
"#,
            home_private = self.home_private_key,
            tunnel_home = self.tunnel_ip_home,
            vps_public = self.vps_public_key,
            psk = self.preshared_key,
            vps_ip = self.vps_ip,
            vps_port = self.vps_wg_port,
            tunnel_vps_ip = self.tunnel_ip_vps.split('/').next().unwrap_or("10.13.37.1"),
        )
    }

    /// Generate the WireGuard server config for the VPS side.
    pub fn generate_vps_wg_conf(&self) -> String {
        format!(
            r#"[Interface]
PrivateKey = {vps_private}
Address = {tunnel_vps}
ListenPort = {vps_port}

[Peer]
PublicKey = {home_public}
PresharedKey = {psk}
AllowedIPs = {tunnel_home_ip}/32
"#,
            vps_private = self.vps_private_key,
            tunnel_vps = self.tunnel_ip_vps,
            vps_port = self.vps_wg_port,
            home_public = self.home_public_key,
            psk = self.preshared_key,
            tunnel_home_ip = self.tunnel_ip_home.split('/').next().unwrap_or("10.13.37.2"),
        )
    }

    /// Generate docker-compose.yml for the VPS relay container.
    pub fn generate_docker_compose(&self) -> String {
        format!(
            r#"version: "3.8"
services:
  zecbox-relay:
    image: alpine:3.20
    container_name: zecbox-relay
    restart: unless-stopped
    cap_add:
      - NET_ADMIN
    sysctls:
      - net.ipv4.ip_forward=1
    ports:
      - "{vps_port}:{vps_port}/udp"
      - "8233:8233/tcp"
    volumes:
      - ./wg0.conf:/etc/wireguard/wg0.conf:ro
    entrypoint: /bin/sh
    command:
      - -c
      - |
        apk add --no-cache wireguard-tools socat iptables
        wg-quick up wg0
        socat TCP-LISTEN:8233,fork,reuseaddr TCP:{tunnel_home_ip}:8233 &
        echo "ZecBox relay active — forwarding :8233 to {tunnel_home_ip}:8233"
        trap "wg-quick down wg0; kill %1" TERM INT
        wait
"#,
            vps_port = self.vps_wg_port,
            tunnel_home_ip = self.tunnel_ip_home.split('/').next().unwrap_or("10.13.37.2"),
        )
    }

    /// Generate a one-liner install command for the VPS.
    pub fn generate_install_command(&self) -> String {
        let b64 = base64::engine::general_purpose::STANDARD;
        let wg_conf = self.generate_vps_wg_conf();
        let compose = self.generate_docker_compose();

        let payload = format!("{}|||{}", wg_conf, compose);
        let encoded = b64.encode(payload.as_bytes());

        format!(
            r#"mkdir -p /opt/zecbox-relay && echo '{}' | base64 -d | awk -F'|||' '{{print $1 > "/opt/zecbox-relay/wg0.conf"; print $2 > "/opt/zecbox-relay/docker-compose.yml"}}' && cd /opt/zecbox-relay && docker compose up -d"#,
            encoded,
        )
    }

    fn config_path(default_data_dir: &Path) -> std::path::PathBuf {
        default_data_dir.join("config").join(PROXY_CONFIG_FILE)
    }

    pub fn load(default_data_dir: &Path) -> Result<Self, String> {
        let path = Self::config_path(default_data_dir);
        if !path.exists() {
            return Err("Proxy config not found. Run proxy setup first.".into());
        }
        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read proxy config: {}", e))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse proxy config: {}", e))
    }

    pub fn save(&self, default_data_dir: &Path) -> Result<(), String> {
        let path = Self::config_path(default_data_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize proxy config: {}", e))?;
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &contents)
            .map_err(|e| format!("Failed to write proxy config: {}", e))?;
        std::fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to rename proxy config: {}", e))?;

        // Restrict permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }

    pub fn delete(default_data_dir: &Path) -> Result<(), String> {
        let path = Self::config_path(default_data_dir);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete proxy config: {}", e))?;
        }
        Ok(())
    }

    pub fn exists(default_data_dir: &Path) -> bool {
        Self::config_path(default_data_dir).exists()
    }
}

/// Validate that a string is a public IP address (not RFC1918 private, not loopback, not link-local).
pub fn validate_public_ip(ip: &str) -> Result<(), String> {
    let addr: std::net::IpAddr = ip.parse()
        .map_err(|_| format!("'{}' is not a valid IP address", ip))?;

    match addr {
        std::net::IpAddr::V4(v4) => {
            if v4.is_loopback() {
                return Err("Loopback addresses (127.x) are not valid VPS IPs.".into());
            }
            if v4.is_private() {
                return Err("Private addresses (10.x, 172.16-31.x, 192.168.x) are not valid VPS IPs. Enter the public IP of your VPS.".into());
            }
            if v4.is_link_local() {
                return Err("Link-local addresses (169.254.x) are not valid VPS IPs.".into());
            }
            if v4.is_unspecified() {
                return Err("0.0.0.0 is not a valid VPS IP.".into());
            }
        }
        std::net::IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_unspecified() {
                return Err("Not a valid public VPS IP address.".into());
            }
        }
    }

    Ok(())
}
