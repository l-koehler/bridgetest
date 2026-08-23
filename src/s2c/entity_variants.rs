use azalea::Client;
use azalea::block::BlockState;
use azalea::core::data_registry::ResolvableDataRegistry;
use azalea::ecs::entity::Entity;
use azalea::entity::metadata;
use azalea::registry::DataRegistryKey;
use azalea::registry::builtin::{BlockKind, EntityKind};
use log::warn;

use crate::state;
use crate::utils;

/// Falling blocks: Like dropped items we dont want a separate variant for each
/// The data is sent right away though, so we dont have to do that whole thing
/// (why would you ever expect any amount of consistency what)
pub fn falling_block_textures(data: i32, media_state: &state::MediaState) -> Option<[String; 6]> {
    let state = BlockState::try_from(data).ok()?;
    let kind = BlockKind::from(state);
    let mapping = media_state.block_texture_map.get(kind.to_str())?;
    Some(mapping.to_entity_textures())
}

/// Extra per-instance data that can affect model/texture selection
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtraData {
    pub baby: bool,
    pub variant: Option<String>,
}

/// Reads ECS metadata, applies defaults for missing metadata, returns the combined ExtraData
/// Dropped items are a special case to not add a variant per item stack
pub fn extract(
    mc_client: &Client,
    entity: Entity,
    kind: EntityKind,
    media_state: &state::MediaState,
) -> ExtraData {
    if kind == EntityKind::Item {
        let texture = mc_client
            .get_entity_component::<metadata::ItemItem>(entity)
            .map(|item| utils::texture_from_itemstack(&item.0, media_state));
        return ExtraData {
            baby: false,
            variant: texture,
        };
    }
    ExtraData {
        baby: is_baby(mc_client, entity, kind),
        variant: get_variant(mc_client, entity, kind),
    }
}

fn is_baby(mc_client: &Client, entity: Entity, kind: EntityKind) -> bool {
    match kind {
        // zombie-derived mobs carry their own baby flag rather than the
        // generic AbstractAgeableBaby one
        EntityKind::Zombie
        | EntityKind::Husk
        | EntityKind::Drowned
        | EntityKind::ZombieVillager
        | EntityKind::ZombifiedPiglin => mc_client
            .get_entity_component::<metadata::ZombieBaby>(entity)
            .map(|c| c.0)
            .unwrap_or(false),
        EntityKind::Piglin => mc_client
            .get_entity_component::<metadata::PiglinBaby>(entity)
            .map(|c| c.0)
            .unwrap_or(false),
        EntityKind::Zoglin => mc_client
            .get_entity_component::<metadata::ZoglinBaby>(entity)
            .map(|c| c.0)
            .unwrap_or(false),
        // covers every other Animal/Villager-derived kind via AbstractAgeableBaby
        _ => mc_client
            .get_entity_component::<metadata::AbstractAgeableBaby>(entity)
            .map(|c| c.0)
            .unwrap_or(false),
    }
}

fn get_variant(mc_client: &Client, entity: Entity, kind: EntityKind) -> Option<String> {
    match kind {
        EntityKind::Wolf => mc_client
            .get_entity_component::<metadata::WolfVariant>(entity)
            .and_then(|c| registry_variant_name(mc_client, c.0)),
        EntityKind::Cat => mc_client
            .get_entity_component::<metadata::CatVariant>(entity)
            .and_then(|c| registry_variant_name(mc_client, c.0)),
        EntityKind::Cow => mc_client
            .get_entity_component::<metadata::CowVariant>(entity)
            .and_then(|c| registry_variant_name(mc_client, c.0)),
        EntityKind::Chicken => mc_client
            .get_entity_component::<metadata::ChickenVariant>(entity)
            .and_then(|c| registry_variant_name(mc_client, c.0)),
        EntityKind::Frog => mc_client
            .get_entity_component::<metadata::FrogVariant>(entity)
            .and_then(|c| registry_variant_name(mc_client, c.0)),
        EntityKind::Pig => mc_client
            .get_entity_component::<metadata::PigVariant>(entity)
            .and_then(|c| registry_variant_name(mc_client, c.0)),
        EntityKind::Mooshroom => mc_client
            .get_entity_component::<metadata::MooshroomKind>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        EntityKind::Pufferfish => mc_client
            .get_entity_component::<metadata::PuffState>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        EntityKind::Fox => mc_client
            .get_entity_component::<metadata::FoxKind>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        EntityKind::Rabbit => mc_client
            .get_entity_component::<metadata::RabbitKind>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        EntityKind::Parrot => mc_client
            .get_entity_component::<metadata::ParrotVariant>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        // trader llamas are llamas - same component, same ordinal meaning
        EntityKind::Llama | EntityKind::TraderLlama => mc_client
            .get_entity_component::<metadata::LlamaVariant>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        EntityKind::Salmon => mc_client
            .get_entity_component::<metadata::SalmonKind>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        EntityKind::Axolotl => mc_client
            .get_entity_component::<metadata::AxolotlVariant>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        EntityKind::TropicalFish => mc_client
            .get_entity_component::<metadata::TropicalFishTypeVariant>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        EntityKind::Horse => mc_client
            .get_entity_component::<metadata::HorseTypeVariant>(entity)
            .and_then(|c| raw_int_variant_name(kind, c.0)),
        _ => None,
    }
}

/// (EntityKind, raw metadata int) -> variant suffix name
/// https://minecraft.wiki/w/Java_Edition_protocol/Entity_metadata
fn raw_int_variant_name(kind: EntityKind, value: i32) -> Option<String> {
    match kind {
        // mooshroom type: 0/1 red/brown
        EntityKind::Mooshroom => match value {
            1 => Some(String::from("brown")),
            _ => None,
        },
        // puff state: 0/1/2 deflated/inflating/inflated
        EntityKind::Pufferfish => match value {
            1 => Some(String::from("medium")),
            2 => Some(String::from("big")),
            _ => None,
        },
        // fox region: 0/1 normal/snow
        EntityKind::Fox => match value {
            1 => Some(String::from("snow")),
            _ => None,
        },
        // not on the protocol page - taken from https://minecraft.wiki/w/Rabbit#Entity_data:
        // rabbit type: 0/1/2/3/4/5/99  brown/white/black/splotched/gold/salt/evil
        EntityKind::Rabbit => match value {
            1 => Some(String::from("white")),
            2 => Some(String::from("black")),
            3 => Some(String::from("white_splotched")),
            4 => Some(String::from("gold")),
            5 => Some(String::from("salt")),
            99 => Some(String::from("caerbannog")),
            _ => None,
        },
        // parrot color: 0/1/2/3/4 redblue/blue/green/yellowblue/grey
        EntityKind::Parrot => match value {
            1 => Some(String::from("blue")),
            2 => Some(String::from("green")),
            3 => Some(String::from("yellow_blue")),
            4 => Some(String::from("grey")),
            _ => None,
        },
        // llama color: 0/1/2/3 creamy/white/brown/gray
        EntityKind::Llama | EntityKind::TraderLlama => match value {
            0 => Some(String::from("creamy")),
            1 => Some(String::from("white")),
            2 => Some(String::from("brown")),
            3 => Some(String::from("gray")),
            _ => None,
        },
        // salmon size: 0/1/2 small/medium/large
        EntityKind::Salmon => match value {
            0 => Some(String::from("small")),
            1 => Some(String::from("medium")),
            2 => Some(String::from("large")),
            _ => None,
        },
        // Mineclonia colors don't map
        // we remap: lucy->pink, wild->brown, gold->yellow, cyan->white, blue->purple
        // axolotl colors: 0/1/2/3/4 lucy/wild/gold/cyan/blue
        EntityKind::Axolotl => match value {
            0 => Some(String::from("lucy")),
            2 => Some(String::from("gold")),
            3 => Some(String::from("cyan")),
            4 => Some(String::from("blue")),
            _ => None,
        },
        // https://minecraft.wiki/w/Tropical_Fish#Entity_data
        // (pattern_color << 24) | (body_color << 16) | (pattern << 8) | size
        // we only use size, the rest seems exceedingly complicated for now
        EntityKind::TropicalFish => match value & 0xFF {
            1 => Some(String::from("alternate")),
            _ => None,
        },
        // https://minecraft.wiki/w/Horse#Entity_data
        // color | (markings << 8)
        // 0/1/2/3/4/5/6 white/creamy/chestnut/brown/black/gray/dark_brown
        EntityKind::Horse => match value & 0xFF {
            0 => Some(String::from("white")),
            1 => Some(String::from("creamy")),
            2 => Some(String::from("chestnut")),
            4 => Some(String::from("black")),
            5 => Some(String::from("gray")),
            6 => Some(String::from("dark_brown")),
            _ => None,
        },
        _ => None,
    }
}

/// resolve ECS (WolfVariant, CatVariant) to its name ("black", "siamese")
/// Needs to match entity_variants file
fn registry_variant_name<R: ResolvableDataRegistry>(
    mc_client: &Client,
    value: R,
) -> Option<String> {
    mc_client
        .resolve_registry_key(&value)
        .ok()
        .flatten()
        .map(|key| key.into_ident().path().to_string())
}

/// Compose one texture slot [base_or_empty, overlay, overlay, ...] onto an accumulated string for that slot
fn compose_slot(mut acc: String, pieces: &[String]) -> String {
    let mut pieces = pieces.iter();
    if let Some(base) = pieces.next()
        && !base.is_empty()
    {
        acc = base.clone();
    }
    for overlay in pieces {
        acc = if acc.is_empty() {
            overlay.clone()
        } else {
            format!("{acc}^{overlay}")
        };
    }
    acc
}

fn visual_str(visual_kind: utils::VisualKind) -> &'static str {
    match visual_kind {
        utils::VisualKind::Texture => "sprite",
        utils::VisualKind::Block => "cube",
        utils::VisualKind::Model => "mesh",
    }
}

/// (EntityKind, extra instance data) -> (visual, mesh, textures, size).
/// visual is a Luanti ObjectProperties.visual string ("sprite"/"cube"/"mesh")
pub fn get_entity_model(
    kind: EntityKind,
    extra: ExtraData,
) -> (String, String, Vec<String>, [f32; 3]) {
    let base = utils::entity_info_by_key(kind.to_str()).unwrap_or_else(|| {
        utils::entity_info_by_key("_default")
            .expect("extra_data/entity_info.json missing a \"_default\" entry")
    });
    let visual_kind = base
        .visual
        .unwrap_or_else(|| panic!("entity_info.json entry for {kind:?} has no \"type\""));

    // dont make me create one variant per item stack, special-case those
    if kind == EntityKind::Item {
        return (
            String::from(visual_str(visual_kind)),
            String::new(),
            vec![extra.variant.unwrap_or_default()],
            base.size.unwrap_or([1.0, 1.0, 1.0]),
        );
    }

    let mut model = base.model.clone().unwrap_or_default();
    let base_slots = base
        .textures
        .clone()
        .unwrap_or_else(|| panic!("entity_info.json entry for {:?} has no textures", kind));
    let mut size = base.size.unwrap_or([1.0, 1.0, 1.0]);

    let mut textures: Vec<String> = base_slots
        .iter()
        .map(|slot| compose_slot(String::new(), slot))
        .collect();

    // needed for conflict reporting
    let (mut model_set, mut size_set) = (false, false);
    let mut texture_base_set = vec![false; textures.len()];

    let mut suffixes: Vec<String> = Vec::new();
    if extra.baby {
        suffixes.push(String::from("baby"));
    }
    if let Some(variant) = extra.variant {
        suffixes.push(variant);
    }

    for suffix in suffixes {
        let key = format!("{}+{}", kind.to_str(), suffix);
        let Some(modifier) = utils::entity_info_by_key(&key) else {
            continue; // no dedicated entry for the modifier
        };
        if let Some(m) = &modifier.model {
            if model_set {
                warn!(
                    "entity_info.json: {kind:?} active modifiers conflict on model ({model} vs {m})"
                );
            }
            model = m.clone();
            model_set = true;
        }
        if let Some(s) = modifier.size {
            if size_set {
                warn!(
                    "entity_info.json: {kind:?} active modifiers conflict on size ({size:?} vs {s:?})"
                );
            }
            size = s;
            size_set = true;
        }
        if let Some(slots) = &modifier.textures {
            for (i, slot) in slots.iter().enumerate() {
                let Some(acc) = textures.get_mut(i) else {
                    warn!(
                        "entity_info.json: {kind:?} {suffix:?} modifier has more texture slots than the base entry has, ignoring extras"
                    );
                    break;
                };
                if slot.first().is_some_and(|base| !base.is_empty()) {
                    if texture_base_set[i] {
                        warn!(
                            "entity_info.json: {kind:?} active modifiers conflict on texture slot {i} base ({acc:?} vs {slot:?})"
                        );
                    }
                    texture_base_set[i] = true;
                }
                *acc = compose_slot(std::mem::take(acc), slot);
            }
        }
    }

    match visual_kind {
        utils::VisualKind::Model if model.is_empty() => {
            panic!("entity_info.json entry for {kind:?} is type \"model\" but has no model")
        }
        utils::VisualKind::Texture if textures.len() != 1 => {
            warn!(
                "entity_info.json: {kind:?} is type \"texture\" but has {} texture slot(s), expected 1",
                textures.len()
            );
        }
        utils::VisualKind::Block if textures.len() != 6 => {
            warn!(
                "entity_info.json: {kind:?} is type \"block\" but has {} texture slot(s), expected 6",
                textures.len()
            );
        }
        _ => (),
    }

    let mesh = if visual_kind == utils::VisualKind::Model {
        model
    } else {
        String::new()
    };

    (String::from(visual_str(visual_kind)), mesh, textures, size)
}
