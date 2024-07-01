use serde::{Deserialize, Serialize};
use solana_sdk::signature::Keypair;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for the solana CLI.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// URL of the solana JSON RPC.
    json_rpc_url: String,

    /// URL of the solana websocket.
    websocket_url: String,

    /// Path to the keypair file.
    keypair_path: PathBuf,

    /// Mapping of address to label.
    address_labels: HashMap<String, String>,

    /// Commitment level.
    /// Options: "max", "recent", "root", "single", "singleGossip", "processed", "confirmed", "finalized"
    commitment: String,
}

impl Config {
    /// Load the configuration from a file.
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let yaml = fs::read_to_string(path)?;
        let config = serde_yaml::from_str::<Config>(&yaml)?;

        Ok(config)
    }

    /// Get the JSON RPC URL.
    pub fn json_rpc_url(&self) -> &str {
        &self.json_rpc_url
    }

    /// Get the websocket URL.
    #[allow(dead_code)]
    pub fn websocket_url(&self) -> &str {
        &self.websocket_url
    }

    /// Get the keypair path.
    #[allow(dead_code)]
    pub fn keypair_path(&self) -> &Path {
        &self.keypair_path
    }

    /// Get the label for an address.
    #[allow(dead_code)]
    pub fn label(&self, address: &str) -> Option<&str> {
        self.address_labels.get(address).map(|s| s.as_str())
    }

    /// Get the commitment level.
    pub fn commitment(&self) -> &str {
        &self.commitment
    }

    /// Load the keypair from the keypair path.
    pub fn load_keypair(&self) -> anyhow::Result<Keypair> {
        let json = fs::read_to_string(&self.keypair_path)?;
        let bytes = serde_json::from_str::<Vec<u8>>(&json)?;
        let key_pair = Keypair::from_bytes(&bytes)?;

        Ok(key_pair)
    }
}
