/*
 * This file contains various defaults used when creating
 * a initial configuration file.
 */

pub const CONF_FALLBACK: &str ="\
# change the line below to point to a complete minecraft texture pack
texture_pack_path = \"\"
# address you will need to point your minetest client to
mc_server_addr = \"127.0.0.1:25565\"
# IP address of the minecraft server. domains DO NOT work.
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
