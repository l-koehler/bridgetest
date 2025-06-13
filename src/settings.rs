/*
 * This file contains various defaults used when creating
 * a initial configuration file.
 *
 * Also some other constants and hacks
 */

// default contents of config file
pub const CONF_FALLBACK: &str ="\
[net]
# must be IPv4:PORT, domains are not supported (yet)
# can be overridden on command line (-s/--server)
mc_server = \"127.0.0.1:25565\"
# port the proxy will listen on
# can be overridden on command line (-p/--port)
luanti_port = 30000
# binds to 127.0.0.1 (local loopback) if true, else 0.0.0.0 (all available addresses)
# can be overridden on command line (--local-only)
local_only = true

[auth]
# username 'random' selects a random username
allow_random_user = true
# this will require interaction for microsoft auth
# it also disables the allow_random_user function above
online_mode = false

[media]
# url to a zip file containing the mineclonia models
model_url = \"https://codeberg.org/mineclonia/mineclonia/archive/main:mods/ENTITIES/mobs_mc/models.zip\"
# resolution of installed textures
# should be 16 unless you changed them
texture_pack_res = 16
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
// these get three random digits appended
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

// How many layers deep we recurse into the assets when building the announcement
pub const TEXTURE_MAX_RECURSION: u8 = 6;
