use roboat::{Client, ClientBuilder, RoboatError};
use safelog::Sensitive;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;

#[derive(Deserialize, Debug)]
struct Config {
    /// The group ID of the group to scan.
    group_id: u64,
    /// The cookie that will be used to authenticate the bot.
    roblosecurity: Sensitive<String>,
    /// Which ranks to scan for members.
    scanned_roles: Vec<String>,
    /// The key is the role name, and the value is a list of years
    /// corresponding to that role.
    role_year_pairs: HashMap<String, Vec<u64>>,
    /// The name of the role that will be
    /// given to users who don't fall into `rank_year_pairs`.
    wildcard_role: String,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Please provide a config file as an argument. Usage: classics-ranking-bot.exe <config_file>")]
    ConfigFileNotProvided,
    #[error("Non-Recoverable Roboat Error: ({0})")]
    NonRecoverableRoboatError(RoboatError),
    #[error("Role {0} not found")]
    RoleNotFound(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match env::args().nth(1) {
        Some(file_path) => serde_json::from_str::<Config>(&fs::read_to_string(file_path)?)?,
        None => return Err(Error::ConfigFileNotProvided.into()),
    };

    let client = ClientBuilder::new()
        .roblosecurity(config.roblosecurity.to_string())
        .build();

    // We make basically a reverse of `role_year_pairs` so that we can
    // easily get the role name from the year.
    let year_role_pairs = reverse_role_year_pairs(&config.role_year_pairs);

    let role_id_lookup = generate_role_id_lookup(
        &client,
        config.group_id,
        &config.role_year_pairs,
        config.wildcard_role,
    )
    .await?;

    Ok(())
}

fn reverse_role_year_pairs(role_year_pairs: &HashMap<String, Vec<u64>>) -> HashMap<u64, String> {
    let mut reversed_map = HashMap::new();

    for (role, years) in role_year_pairs {
        for year in years {
            reversed_map.insert(*year, role.clone());
        }
    }

    reversed_map
}

async fn generate_role_id_lookup(
    client: &Client,
    group_id: u64,
    role_year_pairs: &HashMap<String, Vec<u64>>,
    wildcard_role: String,
) -> Result<HashMap<String, u64>, Error> {
    let group_roles = client
        .group_roles(group_id)
        .await
        .map_err(Error::NonRecoverableRoboatError)?;

    let mut role_id_lookup = HashMap::new();

    for (role_name, years) in role_year_pairs {
        let role_id = group_roles
            .iter()
            .find(|role| &role.name == role_name)
            .map(|role| role.id)
            .ok_or(Error::RoleNotFound(role_name.clone()))?;

        role_id_lookup.insert(role_name.clone(), role_id);
    }

    let wildcard_role_id = group_roles
        .iter()
        .find(|role| &role.name == &wildcard_role)
        .map(|role| role.id)
        .ok_or(Error::RoleNotFound(wildcard_role.clone()))?;

    role_id_lookup.insert(wildcard_role, wildcard_role_id);

    Ok(role_id_lookup)
}

async fn role_name_to_id(client: &Client, group_id: u64, role_name: &str) -> Result<u64, Error> {
    let group_roles = client
        .group_roles(group_id)
        .await
        .map_err(Error::NonRecoverableRoboatError)?;

    for role in group_roles {
        if role.name == role_name {
            return Ok(role.id);
        }
    }

    Err(Error::RoleNotFound(role_name.to_string()))
}
