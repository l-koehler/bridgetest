/*
 * This file contains shared functions, for example logging
 */

use crate::settings;
use crate::MTServerState;

use azalea::inventory::ItemSlot;
use minetest_protocol::CommandRef;
use minetest_protocol::CommandDirection;
use minetest_protocol::wire::types::{v3f, MapNode};
use azalea_client::Event;
use azalea_core::position::Vec3;
use azalea_protocol::packets::game::ClientboundGamePacket;
use azalea_registry::{EntityKind, Registry};
use azalea_block::BlockState;
use std::path::Path;
use std::path::PathBuf;
use std::io::Read;
use rand::Rng;

pub fn texture_from_itemslot(item: &ItemSlot, mt_server_state: &MTServerState) -> String {
    match item {
        ItemSlot::Empty => String::from("block-air.png"),
        ItemSlot::Present(slot_data) => {
            let item_name = slot_data.kind.to_string().replace("minecraft:", "").to_lowercase() + ".png";
            if mt_server_state.sent_media.contains(&format!("item-{}", &item_name)) {
                // the thing is a item
                format!("item-{}", item_name)
            } else {
                format!("block-{}", item_name)
            }
        }
    }
}

pub fn state_to_node(state: BlockState, cave_air_glow: bool) -> MapNode {
    let mut param0: u16;
    let param1: u8;
    let param2: u8 = 0;
    param0 = azalea_registry::Block::try_from(state).unwrap().to_u32() as u16 + 128;
    
    // param1: transparency i think
    if state.is_air() {
        param0 = 126;
        param1 = 0xEE;
    } else if (azalea_registry::Block::try_from(state).unwrap() == azalea_registry::Block::CaveAir) && cave_air_glow {
        param0 = 120; // custom node: glowing_air
        param1 = 0xEE;
    } else {
        param1 = 0x00;
    }
    
    MapNode {
        param0,
        param1,
        param2,
    }
}

pub fn vec3_to_v3f(input_vector: &Vec3, scale: f64) -> v3f {
    // loss of precision, f64 -> f32
    let Vec3 { x: xf64, y: yf64, z: zf64 } = input_vector;
    v3f {
        x: (*xf64/scale) as f32,
        y: (*yf64/scale) as f32,
        z: (*zf64/scale) as f32
    }
}

pub fn b3d_sanitize(input_path: String) -> String {
    input_path
    .replace("amc_", "")
    .replace("extra_mobs_", "")
    .replace("mcl_boats_", "")
    .replace("mcl_bows_", "")
    .replace("mcl_chests_", "")
    .replace("mcl_minecarts_", "")
    .replace("mcl_", "")
    .replace("mobs_mc_", "")
}

pub fn get_colormap(texture: &str) -> Option<(u8, u8, u8)> {
    // use the "Plains" texture. per-biome textures dont really work in mt afaik
    // https://minecraft.fandom.com/wiki/Color#Block_and_fluid_colors - what blocks use the colormaps
    // https://minecraft.fandom.com/wiki/Block_colors                 - what colors are to be used
    let grass_group = ["block-grass_block_top.png", "block-grass_block_side_overlay.png", "block-short_grass.png", "block-tall_grass_bottom.png", "block-tall_grass_top.png", "block-fern.png", "block-large_fern_bottom.png", "block-large_fern_top.png"];
    if grass_group.contains(&texture) {
        return Some((0x91, 0xBD, 0x59))
    }
    let foliage_group = ["block-oak_leaves.png", "block-jungle_leaves.png", "block-acacia_leaves.png", "block-dark_oak_leaves.png", "block-vine.png"];
    if foliage_group.contains(&texture) {
        return Some((0x77, 0xAB, 0x2F))
    }
    let water_group = ["block-water_still.png", "block-water_flow.png"];
    if water_group.contains(&texture) {
        return Some((0x3F, 0x76, 0xE4))
    }
    let stem_group = ["block-attached_melon_stem.png", "block-attached_pumpkin_stem.png", "block-melon_stem.png", "block-pumpkin_stem.png", "pink_petals_stem.png"];
    if stem_group.contains(&texture) {
        return Some((0xE0, 0xC7, 0x1C))
    }
    // these textures are colormapped but constant for some stupid reason
    if texture == "block-birch_leaves.png" { return Some((0x80, 0xA7, 0x55)) }
    if texture == "block-spruce_leaves.png" { return Some((0x61, 0x99, 0x61)) }
    if texture == "block-lily_pad.png" { return Some((0x20, 0x80, 0x30)) }
    None
}

pub fn ask_confirm(question: &str) -> bool {
    println!("{}",question);
    let mut input = [0];
    let _ = std::io::stdin().read(&mut input);
    match char::from_u32(input[0].into()).expect("Failed to read STDIN") {
        'y' | 'Y' => return true,
        _ => return false,
    }
}

pub fn possibly_create_dir(path: &PathBuf) -> bool {
    if !Path::new(path.as_path()).exists() {
        logger(&format!("Creating directory \"{}\"", path.display()), 0);
        let _ = std::fs::create_dir(path); // TODO check if this worked
        return true;
    } else {
        logger(&format!("Found \"{}\", not creating it", path.display()), 0);
        return false;
    }
}

pub fn show_mt_command(command: &dyn CommandRef) {
    let dir = match command.direction() {
        CommandDirection::ToClient => "S->C",
        CommandDirection::ToServer => "C->S",
    };
    logger(&format!("[Minetest] {} {}", dir, command.command_name()), 0);
    //println!("{} {:#?}", dir, command); // overly verbose
}

pub fn logger(text: &str, level: i8) {
    /*
     * Level 0: Debug - Everything that makes some sense to have
     * Level 1: Stats - Status updates and other sort-of-useful stuff
     * Level 2: Error - Packet got dropped or something
     * Level 3: Fatal - Cannot recover, will panic or drop connections
     */
    if settings::DROP_LOG_BELOW <= level {
        if level == 0 {
            println!("\x1b[0;37m[{:?}] [DEBUG] {}\x1b[0m", chrono::Utc::now().timestamp(), text)
        } else if level == 1{
            println!("[{:?}] [STATS] {}", chrono::Utc::now().timestamp(), text)
        } else if level == 2 {
            println!("\x1b[1;33m[{:?}] [ERROR] {}\x1b[0m", chrono::Utc::now().timestamp(), text)
        } else {
            println!("\x1b[0;31m[{:?}] [FATAL] {}\x1b[0m", chrono::Utc::now().timestamp(), text)
        }
    }


}

pub fn show_mc_command(command: &Event) {
    match command {
        Event::Tick => (), // Don't log ticks, these happen far too often for that
        _ => logger(&format!("[Minecraft] S->C {}", mc_packet_name(command)), 1)
    }
}

pub fn get_random_username() -> String {
    let hs_name = String::from(settings::HS_NAMES[rand::thread_rng().gen_range(0..26)]);
    format!("{}{:0>3}", hs_name, rand::thread_rng().gen_range(0..1000))
}

pub fn get_entity_model(entity: &EntityKind) -> (&str, Vec<String>) {
    match entity {
        // TODO for entitys without models choose the least stupid-looking fallback
        EntityKind::Axolotl    => ("entitymodel-axolotl.b3d" , vec![String::from("entity-axolotl-cyan.png")]),
        EntityKind::Bat        => ("entitymodel-bat.b3d"     , vec![String::from("entity-bat.png")]),
        EntityKind::Blaze      => ("entitymodel-blaze.b3d"   , vec![String::from("entity-blaze.png")]),
        EntityKind::Boat       => ("entitymodel-boat.b3d"    , vec![String::from("entity-boat-oak.png")]), // TODO use the actual textures for variants
        EntityKind::Cat        => ("entitymodel-cat.b3d"     , vec![String::from("entity-cat-red.png")]),
        EntityKind::CaveSpider => ("entitymodel-spider.b3d"  , vec![String::from("entity-spider-cave_spider.png")]),
        EntityKind::ChestMinecart => ("entitymodel-minecart_chest.b3d", vec![String::from("entity-minecart.png")]), // minecraft adds the chest texture, there is no separate minecart texture
        EntityKind::Chicken    => ("entitymodel-chicken.b3d" , vec![String::from("entity-chicken.png")]),
        EntityKind::Cod        => ("entitymodel-cod.b3d"     , vec![String::from("entity-fish-cod.png")]),
        EntityKind::CommandBlockMinecart => ("entitymodel-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-command_block_side.png")]),
        EntityKind::Cow        => ("entitymodel-cow.b3d"     , vec![String::from("entity-cow-cow.png"), String::from("block-red_mushroom.png^[opacity:0")]), // transparent
        EntityKind::Creeper    => ("entitymodel-creeper.b3d" , vec![String::from("entity-creeper-creeper.png")]),
        EntityKind::Dolphin    => ("entitymodel-dolphin.b3d" , vec![String::from("entity-dolphin.png")]),
        EntityKind::Donkey     => ("entitymodel-horse.b3d"   , vec![String::from("entity-horse-donkey.png")]),
        EntityKind::Drowned    => ("entitymodel-zombie.b3d"  , vec![String::from("entity-zombie-zombie.png")]), // drowned is a layered texture
        EntityKind::ElderGuardian => ("entitymodel-guardian.b3d", vec![String::from("entity-guardian_elder.png")]),
        EntityKind::EndCrystal => ("entitymodel-end_crystal.b3d", vec![String::from("entity-end_crystal-end_crystal.png")]),
        EntityKind::EnderDragon => ("entitymodel-dragon.b3d" , vec![String::from("entity-enderdragon-dragon.png")]),
        EntityKind::Enderman   => ("entitymodel-enderman.b3d", vec![String::from("entity-enderman-enderman.png")]),
        EntityKind::Endermite  => ("entitymodel-endermite.b3d", vec![String::from("entity-endermite.png")]),
        EntityKind::Evoker     => ("entitymodel-evoker.b3d"  , vec![String::from("entity-illager-evoker.png")]),
        EntityKind::Fox        => ("entitymodel-cat.b3d"     , vec![String::from("entity-fox-fox.png")]),
        EntityKind::FurnaceMinecart => ("entitymodel-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-furnace_side.png")]),
        EntityKind::Ghast      => ("entitymodel-ghast.b3d"   , vec![String::from("entity-ghast-ghast.png")]),
        EntityKind::GlowSquid  => ("entitymodel-glow_squid.b3d", vec![String::from("entity-squid-glow_squid.png")]),
        EntityKind::Goat       => ("entitymodel-sheepfur.b3d", vec![String::from("entity-goat-goat.png")]),
        EntityKind::Guardian   => ("entitymodel-guardian.b3d", vec![String::from("entity-guardian.png")]),
        EntityKind::Hoglin     => ("entitymodel-hoglin.b3d"  , vec![String::from("entity-hoglin-hoglin.png")]),
        EntityKind::HopperMinecart => ("entitymodel-minecart_hopper.b3d", vec![String::from("entity-minecart.png")]),
        EntityKind::Horse      => ("entitymodel-horse.b3d"   , vec![String::from("entity-horse-horse_brown.png")]),
        EntityKind::Husk       => ("entitymodel-zombie.b3d"  , vec![String::from("entity-zombie-husk.png")]),
        EntityKind::Illusioner => ("entitymodel-illusioner.b3d", vec![String::from("entity-illager-illusioner.png")]),
        EntityKind::IronGolem  => ("entitymodel-iron_golem.b3d", vec![String::from("entity-iron_golem-iron_golem.png")]),
        EntityKind::Llama      => ("entitymodel-llama.b3d"   , vec![String::from("entity-llama-creamy.png")]),
        EntityKind::MagmaCube  => ("entitymodel-magmacube.b3d", vec![String::from("entity-slime-magmacube.png")]),
        EntityKind::Minecart   => ("entitymodel-minecart.b3d", vec![String::from("entity-minecart.png")]),
        EntityKind::Mooshroom  => ("entitymodel-cow.b3d"     , vec![String::from("entity-cow-red_mooshroom.png"), String::from("block-red_mushroom.png")]),
        EntityKind::Mule       => ("entitymodel-horse.b3d"   , vec![String::from("entity-horse-mule.png")]),
        EntityKind::Ocelot     => ("entitymodel-cat.b3d"     , vec![String::from("entity-cat-ocelot.png")]),
        EntityKind::Parrot     => ("entitymodel-parrot.b3d"  , vec![String::from("entity-parrot-parrot_red_blue.png")]),
        EntityKind::Pig        => ("entitymodel-pig.b3d"     , vec![String::from("entity-pig-pig.png")]),
        EntityKind::Piglin     => ("entitymodel-sword_piglin.b3d", vec![String::from("entity-piglin-piglin.png"), String::from("item-golden_sword.png")]),
        EntityKind::PiglinBrute => ("entitymodel-sword_piglin.b3d", vec![String::from("entity-piglin-piglin_brute.png"), String::from("item-golden_axe.png")]),
        EntityKind::Pillager   => ("entitymodel-pillager.b3d", vec![String::from("entity-illager-pillager.png"), String::from("item-crossbow_arrow.png")]),
        EntityKind::PolarBear  => ("entitymodel-polarbear.b3d", vec![String::from("entity-bear-polarbear.png")]),
        EntityKind::Rabbit     => ("entitymodel-rabbit.b3d"  , vec![String::from("entity-rabbit-brown.png")]),
        EntityKind::Salmon     => ("entitymodel-salmon.b3d"  , vec![String::from("entity-fish-salmon.png")]),
        EntityKind::Sheep      => ("entitymodel-sheepfur.b3d", vec![String::from("entity-sheep-sheep_fur.png"),String::from("entity-sheep-sheep.png")]),
        EntityKind::Shulker    => ("entitymodel-shulker.b3d" , vec![String::from("entity-shulker-shulker.png")]),
        EntityKind::Silverfish => ("entitymodel-silverfish.b3d", vec![String::from("entity-silverfish.png")]),
        EntityKind::Skeleton   => ("entitymodel-skeleton.b3d", vec![String::from("entity-skeleton-skeleton.png"), String::from("bow_pulling_2.png")]),
        EntityKind::Slime      => ("entitymodel-slime.b3d"   , vec![String::from("entity-slime-slime.png")]),
        EntityKind::SnowGolem  => ("entitymodel-snowman.b3d" , vec![String::from("entity-snow_golem.png")]),
        EntityKind::SpawnerMinecart => ("entitymodel-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-spawner.png")]),
        EntityKind::Spider     => ("entitymodel-spider.b3d"  , vec![String::from("entity-spider-spider.png")]),
        EntityKind::Squid      => ("entitymodel-squid.b3d"   , vec![String::from("entity-squid-squid.png")]),
        EntityKind::Stray      => ("entitymodel-stray.b3d"   , vec![String::from("entity-skeleton-stray.png")]), //TODO layered
        EntityKind::Strider    => ("entitymodel-strider.b3d" , vec![String::from("entity-strider.png")]),
        EntityKind::TntMinecart => ("entitymodel-minecart_block.b3d", vec![String::from("entity-minecart.png"), String::from("block-tnt_side.png")]),
        EntityKind::TraderLlama => ("entitymodel-llama.b3d"  , vec![String::from("entity-llama-brown.png")]),
        EntityKind::TropicalFish => ("entitymodel-tropical_fish_a.b3d", vec![String::from("entity-fish-tropical_a.png")]), // a/b textures with patterns. no way am i going to deal with that
        EntityKind::Vex        => ("entitymodel-vex.b3d"     , vec![String::from("entity-illager-vex.png")]),
        EntityKind::Villager   => ("entitymodel-villager.b3d", vec![String::from("entity-villager-villager.png")]),
        EntityKind::Vindicator => ("entitymodel-vindicator.b3d", vec![String::from("entity-illager-vindicator.png"), String::from("item-iron_axe.png")]),
        EntityKind::WanderingTrader => ("entitymodel-villager.b3d", vec![String::from("entity-wandering_trader.png")]),
        EntityKind::Warden     => ("entitymodel-iron_golem.b3d", vec![String::from("entity-warden-warden.png")]),
        EntityKind::Witch      => ("entitymodel-witch.b3d"   , vec![String::from("entity-witch.png")]),
        EntityKind::Wither     => ("entitymodel-wither.b3d"  , vec![String::from("entity-wither-wither.png")]),
        EntityKind::WitherSkeleton => ("entitymodel-witherskeleton.b3d", vec![String::from("entity-skeleton-wither_skeleton.png")]),
        EntityKind::Wolf       => ("entitymodel-wolf.b3d"    , vec![String::from("entity-wolf-wolf.png")]),
        EntityKind::Zoglin     => ("entitymodel-hoglin.b3d"  , vec![String::from("entity-hoglin-zoglin.png")]),
        EntityKind::Zombie     => ("entitymodel-zombie.b3d"  , vec![String::from("entity-zombie-zombie.png")]),
        EntityKind::ZombieHorse => ("entitymodel-horse.b3d"  , vec![String::from("entity-horse-horse_zombie.png")]),
        EntityKind::ZombieVillager => ("entitymodel-villager.b3d", vec![String::from("entity-zombie_villager-zombie_villager.png")]),
        EntityKind::ZombifiedPiglin => ("entitymodel-sword_piglin.b3d", vec![String::from("entity-piglin-zombified_piglin.png"), String::from("item-golden_sword.png")]),
        EntityKind::Player     => ("entitymodel-armor_character.b3d", vec![String::from("entity-player-wide-steve.png")]),
        _                      => ("entitymodel-pig.b3d"     , vec![String::from("entity-pig-pig.png")])
    }
}

pub fn mc_packet_name(command: &Event) -> &str {
    match command {
        Event::Init => "Init",
        Event::Login => "Login",
        Event::Chat(_) => "Chat",
        Event::Tick => "Tick",
        Event::Packet(packet_value) => match **packet_value {
            // There are 117 possible cases here and most of them do not matter
            // Uncomment below if you do not actually need this
            //_ => "GamePacket: Detail disabled!",
            ClientboundGamePacket::Bundle(_) => "GamePacket: Bundle",
            ClientboundGamePacket::AddEntity(_) => "GamePacket: AddEntity",
            ClientboundGamePacket::AddExperienceOrb(_) => "GamePacket: AddExperienceOrb",
            ClientboundGamePacket::Animate(_) => "GamePacket: Animate",
            ClientboundGamePacket::AwardStats(_) => "GamePacket: AwardStats",
            ClientboundGamePacket::BlockChangedAck(_) => "GamePacket: BlockChangedAck",
            ClientboundGamePacket::BlockDestruction(_) => "GamePacket: BlockDestruction",
            ClientboundGamePacket::BlockEntityData(_) => "GamePacket: BlockEntityData",
            ClientboundGamePacket::BlockEvent(_) => "GamePacket: BlockEvent",
            ClientboundGamePacket::BlockUpdate(_) => "GamePacket: BlockUpdate",
            ClientboundGamePacket::BossEvent(_) => "GamePacket: BossEvent",
            ClientboundGamePacket::ChangeDifficulty(_) => "GamePacket: ChangeDifficulty",
            ClientboundGamePacket::ChunkBatchFinished(_) => "GamePacket: ChunkBatchFinished",
            ClientboundGamePacket::ChunkBatchStart(_) => "GamePacket: ChunkBatchStart",
            ClientboundGamePacket::ChunksBiomes(_) => "GamePacket: ChunksBiomes",
            ClientboundGamePacket::ClearTitles(_) => "GamePacket: ClearTitles",
            ClientboundGamePacket::CommandSuggestions(_) => "GamePacket: CommandSuggestions",
            ClientboundGamePacket::Commands(_) => "GamePacket: Commands",
            ClientboundGamePacket::ContainerClose(_) => "GamePacket: ContainerClose",
            ClientboundGamePacket::ContainerSetContent(_) => "GamePacket: ContainerSetContent",
            ClientboundGamePacket::ContainerSetData(_) => "GamePacket: ContainerSetData",
            ClientboundGamePacket::ContainerSetSlot(_) => "GamePacket: ContainerSetSlot",
            ClientboundGamePacket::CookieRequest(_) => "GamePacket: CookieRequest",
            ClientboundGamePacket::Cooldown(_) => "GamePacket: Cooldown",
            ClientboundGamePacket::CustomChatCompletions(_) => "GamePacket: CustomChatCompletions",
            ClientboundGamePacket::CustomPayload(_) => "GamePacket: CustomPayload",
            ClientboundGamePacket::DamageEvent(_) => "GamePacket: DamageEvent",
            ClientboundGamePacket::DebugSample(_) => "GamePacket: DebugSample",
            ClientboundGamePacket::DeleteChat(_) => "GamePacket: DeleteChat",
            ClientboundGamePacket::Disconnect(_) => "GamePacket: Disconnect",
            ClientboundGamePacket::DisguisedChat(_) => "GamePacket: DisguisedChat",
            ClientboundGamePacket::EntityEvent(_) => "GamePacket: EntityEvent",
            ClientboundGamePacket::Explode(_) => "GamePacket: Explode",
            ClientboundGamePacket::ForgetLevelChunk(_) => "GamePacket: ForgetLevelChunk",
            ClientboundGamePacket::GameEvent(_) => "GamePacket: GameEvent",
            ClientboundGamePacket::HorseScreenOpen(_) => "GamePacket: HorseScreenOpen",
            ClientboundGamePacket::HurtAnimation(_) => "GamePacket: HurtAnimation",
            ClientboundGamePacket::InitializeBorder(_) => "GamePacket: InitializeBorder",
            ClientboundGamePacket::KeepAlive(_) => "GamePacket: KeepAlive",
            ClientboundGamePacket::LevelChunkWithLight(_) => "GamePacket: LevelChunkWithLight",
            ClientboundGamePacket::LevelEvent(_) => "GamePacket: LevelEvent",
            ClientboundGamePacket::LevelParticles(_) => "GamePacket: LevelParticles",
            ClientboundGamePacket::LightUpdate(_) => "GamePacket: LightUpdate",
            ClientboundGamePacket::Login(_) => "GamePacket: Login",
            ClientboundGamePacket::MapItemData(_) => "GamePacket: MapItemData",
            ClientboundGamePacket::MerchantOffers(_) => "GamePacket: MerchantOffers",
            ClientboundGamePacket::MoveEntityPos(_) => "GamePacket: MoveEntityPos",
            ClientboundGamePacket::MoveEntityPosRot(_) => "GamePacket: MoveEntityPosRot",
            ClientboundGamePacket::MoveEntityRot(_) => "GamePacket: MoveEntityRot",
            ClientboundGamePacket::MoveVehicle(_) => "GamePacket: MoveVehicle",
            ClientboundGamePacket::OpenBook(_) => "GamePacket: OpenBook",
            ClientboundGamePacket::OpenScreen(_) => "GamePacket: OpenScreen",
            ClientboundGamePacket::OpenSignEditor(_) => "GamePacket: OpenSignEditor",
            ClientboundGamePacket::Ping(_) => "GamePacket: Ping",
            ClientboundGamePacket::PongResponse(_) => "GamePacket: PongResponse",
            ClientboundGamePacket::PlaceGhostRecipe(_) => "GamePacket: PlaceGhostRecipe",
            ClientboundGamePacket::PlayerAbilities(_) => "GamePacket: PlayerAbilities",
            ClientboundGamePacket::PlayerChat(_) => "GamePacket: PlayerChat",
            ClientboundGamePacket::PlayerCombatEnd(_) => "GamePacket: PlayerCombatEnd",
            ClientboundGamePacket::PlayerCombatEnter(_) => "GamePacket: PlayerCombatEnter",
            ClientboundGamePacket::PlayerCombatKill(_) => "GamePacket: PlayerCombatKill",
            ClientboundGamePacket::PlayerInfoRemove(_) => "GamePacket: PlayerInfoRemove",
            ClientboundGamePacket::PlayerInfoUpdate(_) => "GamePacket: PlayerInfoUpdate",
            ClientboundGamePacket::PlayerLookAt(_) => "GamePacket: PlayerLookAt",
            ClientboundGamePacket::PlayerPosition(_) => "GamePacket: PlayerPosition",
            ClientboundGamePacket::Recipe(_) => "GamePacket: Recipe",
            ClientboundGamePacket::RemoveEntities(_) => "GamePacket: RemoveEntities",
            ClientboundGamePacket::RemoveMobEffect(_) => "GamePacket: RemoveMobEffect",
            ClientboundGamePacket::ResetScore(_) => "GamePacket: ResetScore",
            ClientboundGamePacket::ResourcePackPop(_) => "GamePacket: ResourcePackPop",
            ClientboundGamePacket::ResourcePackPush(_) => "GamePacket: ResourcePackPush",
            ClientboundGamePacket::Respawn(_) => "GamePacket: Respawn",
            ClientboundGamePacket::RotateHead(_) => "GamePacket: RotateHead",
            ClientboundGamePacket::SectionBlocksUpdate(_) => "GamePacket: SectionBlocksUpdate",
            ClientboundGamePacket::SelectAdvancementsTab(_) => "GamePacket: SelectAdvancementsTab",
            ClientboundGamePacket::ServerData(_) => "GamePacket: ServerData",
            ClientboundGamePacket::SetActionBarText(_) => "GamePacket: SetActionBarText",
            ClientboundGamePacket::SetBorderCenter(_) => "GamePacket: SetBorderCenter",
            ClientboundGamePacket::SetBorderLerpSize(_) => "GamePacket: SetBorderLerpSize",
            ClientboundGamePacket::SetBorderSize(_) => "GamePacket: SetBorderSize",
            ClientboundGamePacket::SetBorderWarningDelay(_) => "GamePacket: SetBorderWarningDelay",
            ClientboundGamePacket::SetBorderWarningDistance(_) => "GamePacket: SetBorderWarningDistance",
            ClientboundGamePacket::SetCamera(_) => "GamePacket: SetCamera",
            ClientboundGamePacket::SetCarriedItem(_) => "GamePacket: SetCarriedItem",
            ClientboundGamePacket::SetChunkCacheCenter(_) => "GamePacket: SetChunkCacheCenter",
            ClientboundGamePacket::SetChunkCacheRadius(_) => "GamePacket: SetChunkCacheRadius",
            ClientboundGamePacket::SetDefaultSpawnPosition(_) => "GamePacket: SetDefaultSpawnPosition",
            ClientboundGamePacket::SetDisplayObjective(_) => "GamePacket: SetDisplayObjective",
            ClientboundGamePacket::SetEntityData(_) => "GamePacket: SetEntityData",
            ClientboundGamePacket::SetEntityLink(_) => "GamePacket: SetEntityLink",
            ClientboundGamePacket::SetEntityMotion(_) => "GamePacket: SetEntityMotion",
            ClientboundGamePacket::SetEquipment(_) => "GamePacket: SetEquipment",
            ClientboundGamePacket::SetExperience(_) => "GamePacket: SetExperience",
            ClientboundGamePacket::SetHealth(_) => "GamePacket: SetHealth",
            ClientboundGamePacket::SetObjective(_) => "GamePacket: SetObjective",
            ClientboundGamePacket::SetPassengers(_) => "GamePacket: SetPassengers",
            ClientboundGamePacket::SetPlayerTeam(_) => "GamePacket: SetPlayerTeam",
            ClientboundGamePacket::SetScore(_) => "GamePacket: SetScore",
            ClientboundGamePacket::SetSimulationDistance(_) => "GamePacket: SetSimulationDistance",
            ClientboundGamePacket::SetSubtitleText(_) => "GamePacket: SetSubtitleText",
            ClientboundGamePacket::SetTime(_) => "GamePacket: SetTime",
            ClientboundGamePacket::SetTitleText(_) => "GamePacket: SetTitleText",
            ClientboundGamePacket::SetTitlesAnimation(_) => "GamePacket: SetTitlesAnimation",
            ClientboundGamePacket::SoundEntity(_) => "GamePacket: SoundEntity",
            ClientboundGamePacket::Sound(_) => "GamePacket: Sound",
            ClientboundGamePacket::StartConfiguration(_) => "GamePacket: StartConfiguration",
            ClientboundGamePacket::StopSound(_) => "GamePacket: StopSound",
            ClientboundGamePacket::StoreCookie(_) => "GamePacket: StoreCookie",
            ClientboundGamePacket::SystemChat(_) => "GamePacket: SystemChat",
            ClientboundGamePacket::TabList(_) => "GamePacket: TabList",
            ClientboundGamePacket::TagQuery(_) => "GamePacket: TagQuery",
            ClientboundGamePacket::TakeItemEntity(_) => "GamePacket: TakeItemEntity",
            ClientboundGamePacket::TeleportEntity(_) => "GamePacket: TeleportEntity",
            ClientboundGamePacket::TickingState(_) => "GamePacket: TickingState",
            ClientboundGamePacket::TickingStep(_) => "GamePacket: TickingStep",
            ClientboundGamePacket::Transfer(_) => "GamePacket: Transfer",
            ClientboundGamePacket::UpdateAdvancements(_) => "GamePacket: UpdateAdvancements",
            ClientboundGamePacket::UpdateAttributes(_) => "GamePacket: UpdateAttributes",
            ClientboundGamePacket::UpdateMobEffect(_) => "GamePacket: UpdateMobEffect",
            ClientboundGamePacket::UpdateRecipes(_) => "GamePacket: UpdateRecipes",
            ClientboundGamePacket::UpdateTags(_) => "GamePacket: UpdateTags",
        },
        Event::AddPlayer(_) => "AddPlayer",
        Event::RemovePlayer(_) => "RemovePlayer",
        Event::UpdatePlayer(_) => "UpdatePlayer",
        Event::Death(_) => "Death",
        Event::KeepAlive(_) => "KeepAlive",
        Event::Disconnect(_) => "Disconnect",
    }
}
