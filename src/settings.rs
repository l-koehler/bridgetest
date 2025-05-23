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
";

// IDs for various HUD things
pub const HEALTHBAR_ID: u32 = 0;
pub const FOODBAR_ID: u32 = 1;
pub const AIRBAR_ID: u32 = 2;
pub const SUBTITLE_ID: u32 = 3;

// max "disagreement" between server and client about position
// y distance is only weighted half:
// sqrt(sqrt(delta_x^2 + delta_y^2) + (delta_y/2)^2)
pub const POS_DIFF_TOLERANCE: f32 = 0.5;

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
    "Jade_English",
];

// list[current_player; _NAME_ ; x,y ; size_x,size_y;]
pub const ALL_INV_FIELDS: [&str; 6] = [
    "main",
    "armor",
    "offhand",
    "craft",
    "craftpreview",
    "container",
]; // container is dynamic in size
pub const PLAYER_INV_FORMSPEC: &str = "\
formspec_version[7]
size[12,11.3]
background[0,0;17.45,17.45;gui-container-inventory.png]
style_type[list;spacing=0.135,0.135;size=1.09,1.09;border=false]
listcolors[#0000;#0002]
list[current_player;armor;0.55,0.575;1,4]
list[current_player;craft;6.7,1.26;2,2]
list[current_player;craftpreview;10.5,1.9;1,1]
list[current_player;offhand;5.29,4.25;1,1]
list[current_player;main;0.55,9.7;9,1]
list[current_player;main;0.55,5.75;9,3;9]
list[current_player;container;0,0;0,0]
";
pub const HOTBAR_SIZE: i32 = 9;

/*
 * 0: Display every recieved packet
 * 1: Display some extra status messages
 * 2: Display dropped packets/calls to unimplemented stuff
 * 3: Only display fatal errors
 * +: Disable utils::logger entirely
 *
 * This is not in the config file yet, mostly due to concerns on
 * how to implement that without performance drop. (currently messages that don't get displayed get optimized out)
 */
pub const DROP_LOG_BELOW: i8 = 3;

// How many layers deep we recurse into the assets when building the announcement
pub const TEXTURE_MAX_RECURSION: u8 = 6;
