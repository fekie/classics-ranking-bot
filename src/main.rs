use safelog::{sensitive, Sensitive};
use std::env;

struct Config {
    group_id: u64,
    roblosecurity: Sensitive<String>,
}

#[tokio::main]
async fn main() {
    let config_file = match env::args().nth(1) {
        Some(file) => file,
        None => {
            eprintln!("Please provide a config file as an argument. Usage: classics-ranking-bot.exe <config_file>");
            return;
        }
    };

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
}
