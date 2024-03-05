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

// formspec string for inventory
pub const INV_FORMSPEC: &str = "size[8,7.5]list[current_player;main;0,3.5;8,4;]list[current_player;craft;3,0;3,3;]listring[]list[current_player;craftpreview;7,1;1,1;]";
// Whatgv Rzhgb Fuckl is a firmstßpc s/formrpec whatg iswrong with thign fuico nertwio protoopcko what the fuchjing hell whgt this mess
pub const FORMSPEC_BLOBS: [&str; 3] = [
    "size[8,7.5]list[current_player;main;0,3.5;8,4;]list[current_player;craft;3,0;3,3;]listring[]list[current_player;craftpreview;7,1;1,1;]",
    "formspec_version[6]size[13,8.75]style_type[image;noclip=true]image[0.325,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,7.325;1.1,1.1;mcl_formspec_itemslot.png]list[current_player;main;0.375,7.375;9,1;]image[11.575,7.325;1.1,1.1;crafting_creative_trash.png]list[detached:trash;main;11.625,7.375;1,1;]image[0.325,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[0.325,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[0.325,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[0.325,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[0.325,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,5.825;1.1,1.1;mcl_formspec_itemslot.png]list[detached:creative_random;main;0.375,0.875;9,5;0]label[11.65,4.33;\u{1b}(T@mcl_inventory)\u{1b}F1\u{1b}E / \u{1b}F33\u{1b}E\u{1b}E]image_button[11.575,4.58;1.1,1.1;crafting_creative_prev.png^[transformR270;creative_prev;]image_button[11.575,5.83;1.1,1.1;crafting_creative_next.png^[transformR270;creative_next;]label[0.375,0.375;\u{1b}(c@#313131)\u{1b}(T@mcl_inventory)Search Items\u{1b}E\u{1b}(c@#ffffff)]listring[detached:creative_random;main]listring[current_player;main]listring[detached:trash;main]style[blocks;border=false;bgimg=;bgimg_pressed=;noclip=true]image[0.2,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[0.44,-1.09;1,1;mcl_core:brick_block;blocks;]tooltip[blocks;\u{1b}(T@mcl_inventory)Building Blocks\u{1b}E]style[deco;border=false;bgimg=;bgimg_pressed=;noclip=true]image[1.8,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[2.04,-1.09;1,1;mcl_flowers:peony;deco;]tooltip[deco;\u{1b}(T@mcl_inventory)Decoration Blocks\u{1b}E]style[redstone;border=false;bgimg=;bgimg_pressed=;noclip=true]image[3.4,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[3.64,-1.09;1,1;mesecons:redstone;redstone;]tooltip[redstone;\u{1b}(T@mcl_inventory)Redstone\u{1b}E]style[rail;border=false;bgimg=;bgimg_pressed=;noclip=true]image[5,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[5.24,-1.09;1,1;mcl_minecarts:golden_rail;rail;]tooltip[rail;\u{1b}(T@mcl_inventory)Transportation\u{1b}E]style[misc;border=false;bgimg=;bgimg_pressed=;noclip=true]image[8.2,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[8.44,-1.09;1,1;mcl_buckets:bucket_lava;misc;]tooltip[misc;\u{1b}(T@mcl_inventory)Miscellaneous\u{1b}E]style[nix;border=false;bgimg=;bgimg_pressed=;noclip=true]image[11.3,-1.34;1.5,1.44;crafting_creative_active.png]item_image_button[11.54,-1.09;1,1;mcl_compass:compass;nix;]tooltip[nix;\u{1b}(T@mcl_inventory)Search Items\u{1b}E]style[food;border=false;bgimg=;bgimg_pressed=;noclip=true]image[0.2,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[0.44,8.89;1,1;mcl_core:apple;food;]tooltip[food;\u{1b}(T@mcl_inventory)Foodstuffs\u{1b}E]style[tools;border=false;bgimg=;bgimg_pressed=;noclip=true]image[1.8,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[2.04,8.89;1,1;mcl_core:axe_iron;tools;]tooltip[tools;\u{1b}(T@mcl_inventory)Tools\u{1b}E]style[combat;border=false;bgimg=;bgimg_pressed=;noclip=true]image[3.4,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[3.64,8.89;1,1;mcl_core:sword_gold;combat;]tooltip[combat;\u{1b}(T@mcl_inventory)Combat\u{1b}E]style[mobs;border=false;bgimg=;bgimg_pressed=;noclip=true]image[5,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[5.24,8.89;1,1;mobs_mc:cow;mobs;]tooltip[mobs;\u{1b}(T@mcl_inventory)Mobs\u{1b}E]style[brew;border=false;bgimg=;bgimg_pressed=;noclip=true]image[6.6,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[6.84,-1.09;1,1;mcl_potions:dragon_breath;brew;]tooltip[brew;\u{1b}(T@mcl_inventory)Brewing\u{1b}E]style[matr;border=false;bgimg=;bgimg_pressed=;noclip=true]image[6.6,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[6.84,8.89;1,1;mcl_core:stick;matr;]tooltip[matr;\u{1b}(T@mcl_inventory)Materials\u{1b}E]style[inv;border=false;bgimg=;bgimg_pressed=;noclip=true]image[11.3,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[11.54,8.89;1,1;mcl_chests:chest;inv;]tooltip[inv;\u{1b}(T@mcl_inventory)Survival Inventory\u{1b}E]field[5.325,0.15;6.1,0.6;search;;]field_enter_after_edit[search;true]field_close_on_enter[search;false]set_focus[search;true]p1",
    "formspec_version[6]size[13,8.75]style_type[image;noclip=true]image[0.325,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,7.325;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,7.325;1.1,1.1;mcl_formspec_itemslot.png]list[current_player;main;0.375,7.375;9,1;]image[11.575,7.325;1.1,1.1;crafting_creative_trash.png]list[detached:trash;main;11.625,7.375;1,1;]image[0.325,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[0.325,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[0.325,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[0.325,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[0.325,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[1.575,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[2.825,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[4.075,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[5.325,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[6.575,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[7.825,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[9.075,5.825;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,0.825;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,2.075;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,3.325;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,4.575;1.1,1.1;mcl_formspec_itemslot.png]image[10.325,5.825;1.1,1.1;mcl_formspec_itemslot.png]list[detached:creative_random;main;0.375,0.875;9,5;0]label[11.65,4.33;\u{1b}(T@mcl_inventory)\u{1b}F1\u{1b}E / \u{1b}F33\u{1b}E\u{1b}E]image_button[11.575,4.58;1.1,1.1;crafting_creative_prev.png^[transformR270;creative_prev;]image_button[11.575,5.83;1.1,1.1;crafting_creative_next.png^[transformR270;creative_next;]label[0.375,0.375;\u{1b}(c@#313131)\u{1b}(T@mcl_inventory)Search Items\u{1b}E\u{1b}(c@#ffffff)]listring[detached:creative_random;main]listring[current_player;main]listring[detached:trash;main]style[blocks;border=false;bgimg=;bgimg_pressed=;noclip=true]image[0.2,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[0.44,-1.09;1,1;mcl_core:brick_block;blocks;]tooltip[blocks;\u{1b}(T@mcl_inventory)Building Blocks\u{1b}E]style[deco;border=false;bgimg=;bgimg_pressed=;noclip=true]image[1.8,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[2.04,-1.09;1,1;mcl_flowers:peony;deco;]tooltip[deco;\u{1b}(T@mcl_inventory)Decoration Blocks\u{1b}E]style[redstone;border=false;bgimg=;bgimg_pressed=;noclip=true]image[3.4,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[3.64,-1.09;1,1;mesecons:redstone;redstone;]tooltip[redstone;\u{1b}(T@mcl_inventory)Redstone\u{1b}E]style[rail;border=false;bgimg=;bgimg_pressed=;noclip=true]image[5,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[5.24,-1.09;1,1;mcl_minecarts:golden_rail;rail;]tooltip[rail;\u{1b}(T@mcl_inventory)Transportation\u{1b}E]style[misc;border=false;bgimg=;bgimg_pressed=;noclip=true]image[8.2,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[8.44,-1.09;1,1;mcl_buckets:bucket_lava;misc;]tooltip[misc;\u{1b}(T@mcl_inventory)Miscellaneous\u{1b}E]style[nix;border=false;bgimg=;bgimg_pressed=;noclip=true]image[11.3,-1.34;1.5,1.44;crafting_creative_active.png]item_image_button[11.54,-1.09;1,1;mcl_compass:compass;nix;]tooltip[nix;\u{1b}(T@mcl_inventory)Search Items\u{1b}E]style[food;border=false;bgimg=;bgimg_pressed=;noclip=true]image[0.2,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[0.44,8.89;1,1;mcl_core:apple;food;]tooltip[food;\u{1b}(T@mcl_inventory)Foodstuffs\u{1b}E]style[tools;border=false;bgimg=;bgimg_pressed=;noclip=true]image[1.8,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[2.04,8.89;1,1;mcl_core:axe_iron;tools;]tooltip[tools;\u{1b}(T@mcl_inventory)Tools\u{1b}E]style[combat;border=false;bgimg=;bgimg_pressed=;noclip=true]image[3.4,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[3.64,8.89;1,1;mcl_core:sword_gold;combat;]tooltip[combat;\u{1b}(T@mcl_inventory)Combat\u{1b}E]style[mobs;border=false;bgimg=;bgimg_pressed=;noclip=true]image[5,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[5.24,8.89;1,1;mobs_mc:cow;mobs;]tooltip[mobs;\u{1b}(T@mcl_inventory)Mobs\u{1b}E]style[brew;border=false;bgimg=;bgimg_pressed=;noclip=true]image[6.6,-1.34;1.5,1.44;crafting_creative_inactive.png]item_image_button[6.84,-1.09;1,1;mcl_potions:dragon_breath;brew;]tooltip[brew;\u{1b}(T@mcl_inventory)Brewing\u{1b}E]style[matr;border=false;bgimg=;bgimg_pressed=;noclip=true]image[6.6,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[6.84,8.89;1,1;mcl_core:stick;matr;]tooltip[matr;\u{1b}(T@mcl_inventory)Materials\u{1b}E]style[inv;border=false;bgimg=;bgimg_pressed=;noclip=true]image[11.3,8.64;1.5,1.44;crafting_creative_inactive_down.png]item_image_button[11.54,8.89;1,1;mcl_chests:chest;inv;]tooltip[inv;\u{1b}(T@mcl_inventory)Survival Inventory\u{1b}E]field[5.325,0.15;6.1,0.6;search;;]field_enter_after_edit[search;true]field_close_on_enter[search;false]set_focus[search;true]p1",
];

// IDs for various HUD things
pub const HEALTHBAR_ID: u32 = 0;
pub const FOODBAR_ID:   u32 = 1;
pub const AIRBAR_ID:    u32 = 2;

// max "disagreement" between server and client about position
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
"Jade_English"];

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
