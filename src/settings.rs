/*
 * This file contains various defaults used when creating
 * a initial configuration file.
 */

pub const CONF_FALLBACK: &str ="# CHANGEME!! 
texture_pack_path = \"\"
mc_server_addr = \"127.0.0.1:25565\"
mt_server_addr = \"127.0.0.1:30000\"
";

/*
 * 0: Display every recieved packet
 * 1: Display some extra status messages (and every SENT packet lol i suck at this)
 * 2: Display dropped packets/calls to unimplemented stuff (currently also everything, this program is utterly broken)
 * 3: Only display fatal errors
 * +: Disable utils::logger entirely
 */
pub const DROP_LOG_BELOW: i8 = 1;
