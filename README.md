# classics-ranking-bot
This program is an on-command ranking bot that ranks users in [-Classics-](https://www.roblox.com/groups/3489210/Classics) based on their account age. 
Ranking rules and info can be found [here](https://devforum.roblox.com/t/classics-rules-and-info/477028).

# Usage
1. Download the latest executable and example config from [here](https://github.com/Chloe-Woahie/classics-ranking-bot/releases/)
2. Make a copy of config-example.json and rename it to config.json.
3. Fill in the required fields in config.json.
    * `group_id` - The group ID.
    * `roblosecurity` - The .ROBLOSECURITY of the account that will be used to rank users.
    * `scanned_roles` - The roles to scan users you want to rank from.
    * `role_year_pairs` - A map where the keys are the roles, and the values are the account age years that the role corresponds to.
    * `wildcard_role` - The role to give users that don't have an account age that corresponds to a role.
4. Inside a terminal window, run the executable with ```classics-ranking-bot.exe config.json```.

# License 
MIT License
