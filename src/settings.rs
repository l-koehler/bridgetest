/*
 * This file contains various defaults used when creating
 * a initial configuration file.
 * 
 * Also some other constants and hacks
 */

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

// IDs for various HUD things
pub const HEALTHBAR_ID: u32 = 0;
pub const FOODBAR_ID:   u32 = 1;
pub const AIRBAR_ID:    u32 = 2;

// max "disagreement" between server and client about position
pub const POS_DIFF_TOLERANCE: f32 = 0.0;
// max ticks without forcing the mt client to fit the tolerance above (20=1sec)
pub const POS_FORCE_AFTER: u32 = 15;

// names to use for random name generation
pub const HS_NAMES: [&str; 26] = [
"Aradia_Megido",
"Tavros_Nitram",
"Sollux_Captor",
"Karkat_Vantas",
"Nepeta_Leijon",
"Kanaya_Maryam",
"Terezi_Pyrope",
"Vriska_Serket",
"Equius_Zahhak",
"Gamzee_Makara",
"Eridan_Ampora",
"Feferi_Peixes",
"John_Egbert",
"Rose_Lalonde",
"Dave_Strider",
"Jade_Harley",
"Jane_Egbert",
"Roxy_Lalonde",
"Jake_Harley",
"Dad_Egbert",
"Jane_Crocker",
"Dirk_Strider",
"Jake_English",
"Dad_Crocker",
"John_Crocker",
"Jade_English"];

// formspecs (basically UI definitions)
pub const FORMSPEC_PREPEND: &str = "\
";

// list[current_player; _NAME_ ; x,y ; size_x,size_y;]
pub const ALL_INV_FIELDS: [&str; 5] = ["main", "armor", "offhand", "craft", "craftpreview"];
pub const PLAYER_INV_FORMSPEC: &str = "\
formspec_version[7]
size[12,11.3]
background[0,0;17.45,17.45;container-inventory.png]
style_type[list;spacing=0.135,0.135;size=1.09,1.09;border=false]
listcolors[#0000;#0002]
list[current_player;armor;0.55,0.575;1,4]
list[current_player;craft;6.7,1.26;2,2]
list[current_player;craftpreview;10.5,1.9;1,1]
list[current_player;offhand;5.29,4.25;1,1]
list[current_player;main;0.55,9.7;9,1;27]
list[current_player;main;0.55,5.75;9,3]
";
pub const HOTBAR_SIZE: i32 = 9;

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
pub const DROP_LOG_BELOW: i8 = 2;
