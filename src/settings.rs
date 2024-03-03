/*
 * This file contains various defaults used when creating
 * a initial configuration file.
 * 
 * Also some other constants and hacks
 */

// world height limit
pub const Y_LOWER: i16 = -64;
pub const Y_UPPER: i16 = 320;

// default text for config file
pub const CONF_FALLBACK: &str ="\
## feel free to change these values, shouldn't break anything important.
# download link to a complete minecraft texture pack
texture_pack_url = \"https://database.faithfulpack.net/packs/32x-Java/December%202023/Faithful%2032x%20-%201.20.4.zip\"
# resolution of the texture pack
texture_pack_res = 32
# IP address of the minecraft server. domains DO NOT work.
mc_server_addr = \"127.0.0.1:25565\"
# address you will need to point your minetest client to
mt_server_addr = \"127.0.0.1:30000\"

## these values should not need to be changed, don't change them unless you need to
# URL to fetch the block definitions from
arcticdata_blocks = \"https://raw.githubusercontent.com/Articdive/ArticData/1.20.4/1_20_4_blocks.json\"
# URL to fetch the item definitions from
arcticdata_items = \"https://raw.githubusercontent.com/Articdive/ArticData/1.20.4/1_20_4_items.json\"
";

// formspec string for inventory
pub const INV_FORMSPEC: &str = "size[8,7.5]list[current_player;main;0,3.5;8,4;]list[current_player;craft;3,0;3,3;]listring[]list[current_player;craftpreview;7,1;1,1;]";

// IDs for various HUD things
pub const HEALTHBAR_ID: u32 = 0;
pub const FOODBAR_ID:   u32 = 1;
pub const AIRBAR_ID:    u32 = 2;

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
