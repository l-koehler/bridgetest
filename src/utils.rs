/*
 * This file contains shared functions, for example logging
 */

use crate::settings;

use minetest_protocol::CommandRef;
use minetest_protocol::CommandDirection;
use azalea_client::Event;
use azalea_protocol::packets::game::ClientboundGamePacket;
use std::path::Path;
use std::path::PathBuf;
use std::io::Read;
use rand::Rng;

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
        _ => logger(&format!("[Minecraft] S->C {}", mc_packet_name(command)), 0)
    }
}

pub fn get_random_username() -> String {
    let hs_name = String::from(settings::HS_NAMES[rand::thread_rng().gen_range(0..31)]);
    format!("{}{:3}", hs_name, rand::thread_rng().gen_range(0..1000)) // :3
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
            ClientboundGamePacket::Cooldown(_) => "GamePacket: Cooldown",
            ClientboundGamePacket::CustomChatCompletions(_) => "GamePacket: CustomChatCompletions",
            ClientboundGamePacket::CustomPayload(_) => "GamePacket: CustomPayload",
            ClientboundGamePacket::DamageEvent(_) => "GamePacket: DamageEvent",
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
            ClientboundGamePacket::SystemChat(_) => "GamePacket: SystemChat",
            ClientboundGamePacket::TabList(_) => "GamePacket: TabList",
            ClientboundGamePacket::TagQuery(_) => "GamePacket: TagQuery",
            ClientboundGamePacket::TakeItemEntity(_) => "GamePacket: TakeItemEntity",
            ClientboundGamePacket::TeleportEntity(_) => "GamePacket: TeleportEntity",
            ClientboundGamePacket::TickingState(_) => "GamePacket: TickingState",
            ClientboundGamePacket::TickingStep(_) => "GamePacket: TickingStep",
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
