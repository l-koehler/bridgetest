/*
 * This file contains various defaults used when creating
 * a initial configuration file.
 */

pub const CONF_FALLBACK: &str ="\
## feel free to change these values, shouldn't break anything important.
# download link to a complete minecraft texture pack
# delete data_dir/pack after changing this [if it doesn't work]
texture_pack_url = \"https://database.faithfulpack.net/packs/32x-Java/December%202023/Faithful%2032x%20-%201.20.4.zip\"
# IP address of the minecraft server. domains DO NOT work.
mc_server_addr = \"127.0.0.1:25565\"
# address you will need to point your minetest client to
mt_server_addr = \"127.0.0.1:30000\"
";

/*
 * 0: Display every recieved packet
 * 1: Display some extra status messages (and every SENT packet lol i suck at this)
 * 2: Display dropped packets/calls to unimplemented stuff (currently also everything, this program is utterly broken)
 * 3: Only display fatal errors
 * +: Disable utils::logger entirely
 *
 * This is not in the config file yet, mostly due to concerns on
 * how to implement that without performance drop.
 */
pub const DROP_LOG_BELOW: i8 = 1;
