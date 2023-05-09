use safelog::Sensitive;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;

#[derive(Deserialize, Debug)]
struct Config {
    group_id: u64,
    roblosecurity: Sensitive<String>,
    scanned_ranks: Vec<String>,
    rank_year_pairs: HashMap<String, Vec<u64>>,
}

#[derive(Debug, Clone, thiserror::Error)]
enum Error {
    #[error("Please provide a config file as an argument. Usage: classics-ranking-bot.exe <config_file>")]
    ConfigFileNotProvided,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_file = match env::args().nth(1) {
        Some(file_path) => serde_json::from_str::<Config>(&fs::read_to_string(file_path)?)?,
        None => return Err(Error::ConfigFileNotProvided.into()),
    };

    dbg!(config_file);

    Ok(())
}
