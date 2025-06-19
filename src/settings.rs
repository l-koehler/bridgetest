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
# this requires some interaction with the proxy for microsoft auth
# it also disables the allow_random_user function above, as the chosen name in luanti will not matter anymore
online_mode = false
# only used to log into microsoft when online_mode = true
# _technically_ only a cache key, but use your email to avoid confusion
microsoft_email = \"\"

[media]
# url to a zip file containing the mineclonia models
model_url = \"https://codeberg.org/mineclonia/mineclonia/archive/main:mods/ENTITIES/mobs_mc/models.zip\"
# resolution of installed textures
# should be 16 unless you changed them
texture_pack_res = 16
";

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

// How many layers deep we recurse into the assets when building the announcement
pub const TEXTURE_MAX_RECURSION: u8 = 6;
