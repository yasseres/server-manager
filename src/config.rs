use serde::Deserialize;
use std::fs;

// OS type enum
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OsType {
    Linux,
    Windows,
}

// This struct matches ONE server entry in servers.toml
#[derive(Deserialize, Debug, Clone)]
pub struct Server {
    pub name: String,
    pub ip: String,
    pub username: String,
    pub os_type: OsType,
}

// This struct matches the overall structure of servers.toml
#[derive(Deserialize, Debug)]
pub struct Config {
    pub servers: Vec<Server>,
}

// Function to read and parse the servers.toml file
pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    // Read the file content as a string
    let content = fs::read_to_string(path)?;

    // Parse the TOML string into our Config struct
    let config: Config = toml::from_str(&content)?;

    Ok(config)
}