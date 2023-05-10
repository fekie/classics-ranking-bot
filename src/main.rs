use roboat::{Client, ClientBuilder, Limit, RoboatError};
use safelog::Sensitive;
use serde::Deserialize;
use std::collections::HashMap;
use std::{env, fs};
use tokio::time::Duration;

const PAGE_LIMIT: Limit = Limit::Hundred;
const TOO_MANY_REQUESTS_COOLDOWN: Duration = Duration::from_secs(60);

const ACCOUNT_AGE_RETRIES: usize = 5;
const SET_GROUP_MEMBER_ROLE_RETRIES: usize = 5;

/// If a user already has a role, this is the error code that will be returned.
/// We can ignore this.
const USER_ALREADY_HAS_ROLE_ROBLOX_ERROR_CODE: u16 = 26;

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
    #[error("{0} endpoint exceeded retry limit")]
    EndpointExceededRetryLimit(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match env::args().nth(1) {
        Some(file_path) => serde_json::from_str::<Config>(&fs::read_to_string(file_path)?)?,
        None => return Err(Error::ConfigFileNotProvided.into()),
    };

    let client = ClientBuilder::new()
        .roblosecurity(config.roblosecurity.into_inner())
        .build();

    // We make basically a reverse of `role_year_pairs` so that we can
    // easily get the role name from the year.
    let year_role_pairs = reverse_role_year_pairs(&config.role_year_pairs);

    let role_id_lookup = generate_role_id_lookup(
        &client,
        config.group_id,
        &config.scanned_roles,
        &config.role_year_pairs,
        config.wildcard_role.clone(),
    )
    .await?;

    // We loop through each rank we need to scan.
    for role_to_scan in config.scanned_roles {
        // We now page through the members of the group and assign them.
        // We go in pages of 100 at a time.

        let role_to_scan_id = role_id_lookup
            .get(&role_to_scan)
            .ok_or(Error::RoleNotFound(role_to_scan.clone()))?;

        let mut next_cursor = None;

        loop {
            let (member_ids, new_cursor) =
                page_of_members(&client, config.group_id, *role_to_scan_id, next_cursor).await?;

            if member_ids.is_empty() {
                break;
            }

            next_cursor = new_cursor;

            // We now loop through each member and assign them their role based on their account age.
            for member_id in member_ids {
                let account_age = year_created(&client, member_id).await?;

                let corresponding_role = year_role_pairs.get(&account_age);

                match corresponding_role {
                    Some(role) => {
                        let role_id = role_id_lookup.get(role).unwrap();

                        set_group_member_role(&client, config.group_id, member_id, *role_id)
                            .await?;

                        println!(
                            "Assigned role {} to user {} (account age: {})",
                            role, member_id, account_age
                        );
                    }
                    None => {
                        // If the user doesn't have a corresponding role, we assign them the wildcard role.

                        let role = config.wildcard_role.clone();
                        let role_id = role_id_lookup.get(&role).unwrap();

                        set_group_member_role(&client, config.group_id, member_id, *role_id)
                            .await?;

                        println!(
                            "Assigned role {} to user {} (account age: {})",
                            &config.wildcard_role, member_id, account_age
                        );
                    }
                }
            }

            if next_cursor.is_none() {
                break;
            }
        }
    }

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
    scanned_roles: &[String],
    role_year_pairs: &HashMap<String, Vec<u64>>,
    wildcard_role: String,
) -> Result<HashMap<String, u64>, Error> {
    let group_roles = client
        .group_roles(group_id)
        .await
        .map_err(Error::NonRecoverableRoboatError)?;

    let mut role_id_lookup = HashMap::new();

    for role_name in scanned_roles {
        let role_id = group_roles
            .iter()
            .find(|role| &role.name == role_name)
            .map(|role| role.id)
            .ok_or(Error::RoleNotFound(role_name.clone()))?;

        role_id_lookup.insert(role_name.clone(), role_id);
    }

    for role_name in role_year_pairs.keys() {
        let role_id = group_roles
            .iter()
            .find(|role| &role.name == role_name)
            .map(|role| role.id)
            .ok_or(Error::RoleNotFound(role_name.clone()))?;

        role_id_lookup.insert(role_name.clone(), role_id);
    }

    let wildcard_role_id = group_roles
        .iter()
        .find(|role| role.name == wildcard_role)
        .map(|role| role.id)
        .ok_or(Error::RoleNotFound(wildcard_role.clone()))?;

    role_id_lookup.insert(wildcard_role, wildcard_role_id);

    Ok(role_id_lookup)
}

async fn page_of_members(
    client: &Client,
    group_id: u64,
    role_id: u64,
    cursor: Option<String>,
) -> Result<(Vec<u64>, Option<String>), Error> {
    let (members, next_cursor) = client
        .group_role_members(group_id, role_id, PAGE_LIMIT, cursor)
        .await
        .map_err(Error::NonRecoverableRoboatError)?;

    let mut member_ids = Vec::new();

    for member in members {
        member_ids.push(member.user_id);
    }

    Ok((member_ids, next_cursor))
}

async fn year_created(client: &Client, user_id: u64) -> Result<u64, Error> {
    let mut retries_remaining = ACCOUNT_AGE_RETRIES;

    loop {
        match client.user_details(user_id).await {
            Ok(user_details) => return Ok(user_details.created_at[0..4].parse().unwrap()),
            Err(e) => {
                if retries_remaining == 0 {
                    return Err(Error::EndpointExceededRetryLimit("Account age".to_owned()));
                }

                retries_remaining -= 1;

                // If the error is too many requests, then we sleep for 60 seconds.
                if let RoboatError::TooManyRequests = e {
                    tokio::time::sleep(TOO_MANY_REQUESTS_COOLDOWN).await;
                }
            }
        }
    }
}

async fn set_group_member_role(
    client: &Client,
    group_id: u64,
    user_id: u64,
    role_id: u64,
) -> Result<(), Error> {
    let mut retries_remaining = SET_GROUP_MEMBER_ROLE_RETRIES;

    loop {
        match client
            .set_group_member_role(user_id, group_id, role_id)
            .await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                if retries_remaining == 0 {
                    return Err(Error::EndpointExceededRetryLimit(
                        "Set group member role".to_owned(),
                    ));
                }

                retries_remaining -= 1;

                match e {
                    RoboatError::InvalidRoblosecurity => {
                        return Err(Error::NonRecoverableRoboatError(e))
                    }
                    RoboatError::TooManyRequests => {
                        tokio::time::sleep(TOO_MANY_REQUESTS_COOLDOWN).await;
                    }
                    RoboatError::UnknownRobloxErrorCode { code, .. } => {
                        if code == USER_ALREADY_HAS_ROLE_ROBLOX_ERROR_CODE {
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
