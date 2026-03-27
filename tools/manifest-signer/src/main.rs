//! Keygen and manifest signing tool for ZecBox binary updates.
//!
//! Usage:
//!   manifest-signer keygen                          — generate Ed25519 keypair
//!   manifest-signer sign --key <private.key> <manifest.json>  — sign a manifest
//!   manifest-signer verify --pubkey <hex> <manifest.json>     — verify a signed manifest

use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "manifest-signer", about = "ZecBox manifest signing tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a new Ed25519 keypair for manifest signing
    Keygen {
        /// Output directory for key files (default: current directory)
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    /// Sign an unsigned manifest JSON file
    Sign {
        /// Path to the Ed25519 private key file (hex-encoded)
        #[arg(short, long)]
        key: PathBuf,
        /// Path to the unsigned manifest.json
        manifest: PathBuf,
    },
    /// Verify a signed manifest JSON file
    Verify {
        /// Hex-encoded Ed25519 public key
        #[arg(short, long)]
        pubkey: String,
        /// Path to the signed manifest.json
        manifest: PathBuf,
    },
}

/// Unsigned manifest payload (what gets signed).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestPayload {
    app_version: String,
    binaries: Vec<serde_json::Value>,
}

/// Full signed manifest (with optional signature).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignedManifest {
    app_version: String,
    binaries: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Keygen { output } => {
            let signing_key = SigningKey::generate(&mut OsRng);
            let verifying_key = signing_key.verifying_key();

            let private_hex = hex_encode(signing_key.as_bytes());
            let public_hex = hex_encode(verifying_key.as_bytes());
            let public_bytes = verifying_key.as_bytes();

            // Write private key
            let priv_path = output.join("manifest-signing.key");
            fs::write(&priv_path, &private_hex).expect("Failed to write private key");
            println!("Private key written to: {}", priv_path.display());

            // Write public key
            let pub_path = output.join("manifest-signing.pub");
            fs::write(&pub_path, &public_hex).expect("Failed to write public key");
            println!("Public key written to:  {}", pub_path.display());

            println!();
            println!("Public key (hex): {}", public_hex);
            println!();
            println!("Rust const for src-tauri/src/updates/mod.rs:");
            println!(
                "const MANIFEST_SIGNING_PUBKEY: [u8; 32] = {:?};",
                public_bytes
            );
            println!();
            println!("IMPORTANT:");
            println!("  1. Copy the Rust const above into updates/mod.rs");
            println!("  2. Store the private key hex as GitHub secret MANIFEST_SIGNING_KEY");
            println!("  3. Delete the local manifest-signing.key file after copying");
        }

        Command::Sign { key, manifest } => {
            // Load private key
            let key_hex = fs::read_to_string(&key)
                .expect("Failed to read key file")
                .trim()
                .to_string();
            let key_bytes = hex_decode(&key_hex);
            let signing_key = SigningKey::from_bytes(
                key_bytes
                    .as_slice()
                    .try_into()
                    .expect("Invalid key length — expected 32 bytes"),
            );

            // Load manifest
            let manifest_contents =
                fs::read_to_string(&manifest).expect("Failed to read manifest");
            let mut signed: SignedManifest =
                serde_json::from_str(&manifest_contents).expect("Failed to parse manifest JSON");

            // Extract unsigned payload for canonical serialization
            let payload = ManifestPayload {
                app_version: signed.app_version.clone(),
                binaries: signed.binaries.clone(),
            };
            let canonical =
                serde_json::to_string(&payload).expect("Failed to serialize canonical payload");

            // Sign
            let signature: Signature = signing_key.sign(canonical.as_bytes());
            signed.signature = Some(hex_encode(signature.to_bytes().as_slice()));

            // Write signed manifest (pretty-printed)
            let output =
                serde_json::to_string_pretty(&signed).expect("Failed to serialize signed manifest");
            fs::write(&manifest, output).expect("Failed to write signed manifest");

            println!("Signed manifest written to: {}", manifest.display());
            println!(
                "Signature: {}",
                signed.signature.as_deref().unwrap_or("none")
            );
        }

        Command::Verify { pubkey, manifest } => {
            let pub_bytes = hex_decode(&pubkey);
            let verifying_key = VerifyingKey::from_bytes(
                pub_bytes
                    .as_slice()
                    .try_into()
                    .expect("Invalid public key length — expected 32 bytes"),
            )
            .expect("Invalid Ed25519 public key");

            let manifest_contents =
                fs::read_to_string(&manifest).expect("Failed to read manifest");
            let signed: SignedManifest =
                serde_json::from_str(&manifest_contents).expect("Failed to parse manifest JSON");

            let sig_hex = signed
                .signature
                .as_deref()
                .expect("Manifest has no signature field");

            let payload = ManifestPayload {
                app_version: signed.app_version.clone(),
                binaries: signed.binaries.clone(),
            };
            let canonical =
                serde_json::to_string(&payload).expect("Failed to serialize canonical payload");

            let sig_bytes = hex_decode(sig_hex);
            let signature = Signature::from_slice(&sig_bytes).expect("Invalid signature length");

            match verifying_key.verify(canonical.as_bytes(), &signature) {
                Ok(()) => {
                    println!("Signature is VALID");
                    println!("  App version: {}", signed.app_version);
                    println!("  Binaries: {}", signed.binaries.len());
                }
                Err(e) => {
                    eprintln!("Signature verification FAILED: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
