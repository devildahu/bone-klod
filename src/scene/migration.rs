use std::{error::Error, path::Path};

use bevy::prelude::Vec3;
use serde::Deserialize;

use crate::prefabs::SerdeCollider;

#[derive(Deserialize, Debug)]
struct KlodSceneV1 {
    klod_spawn_transform: super::SerdeTransform,
    objects: Vec<super::PhysicsObject>,
    music_triggers: Vec<super::MusicTriggerData>,
}
impl From<KlodSceneV1> for super::KlodScene {
    fn from(v1: KlodSceneV1) -> Self {
        let KlodSceneV1 { klod_spawn_transform, objects, music_triggers } = v1;
        super::KlodScene {
            klod_spawn_transform,
            objects,
            music_triggers,
            finish_zone: super::FinishZone {
                collider: SerdeCollider::Cuboid { half_extents: Vec3::ONE * 5.0 },
                transform: Default::default(),
            },
            game_timer_seconds: 1.5 * 60.0,
            required_score: 1000.0,
            lights: Vec::new(),
        }
    }
}
#[derive(Deserialize, Debug)]
pub(crate) struct KlodSceneV2 {
    klod_spawn_transform: super::SerdeTransform,
    finish_zone: super::FinishZone,
    game_timer_seconds: f32,
    objects: Vec<super::PhysicsObject>,
    music_triggers: Vec<super::MusicTriggerData>,
}
impl From<KlodSceneV2> for super::KlodScene {
    fn from(v2: KlodSceneV2) -> Self {
        let KlodSceneV2 {
            klod_spawn_transform,
            finish_zone,
            game_timer_seconds,
            objects,
            music_triggers,
        } = v2;
        super::KlodScene {
            klod_spawn_transform,
            finish_zone,
            game_timer_seconds,
            objects,
            music_triggers,
            required_score: 1000.0,
            lights: Vec::new(),
        }
    }
}
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct KlodSceneV3 {
    klod_spawn_transform: super::SerdeTransform,
    finish_zone: super::FinishZone,
    game_timer_seconds: f32,
    objects: Vec<super::PhysicsObject>,
    music_triggers: Vec<super::MusicTriggerData>,
    required_score: f32,
}
impl From<KlodSceneV3> for super::KlodScene {
    fn from(v3: KlodSceneV3) -> Self {
        let KlodSceneV3 {
            klod_spawn_transform,
            finish_zone,
            game_timer_seconds,
            objects,
            music_triggers,
            required_score,
        } = v3;
        super::KlodScene {
            klod_spawn_transform,
            finish_zone,
            game_timer_seconds,
            objects,
            music_triggers,
            required_score,
            lights: Vec::new(),
        }
    }
}

fn try_load<V>(
    scene_path: impl AsRef<Path>,
) -> Result<super::KlodScene, Box<dyn Error + Send + Sync>>
where
    V: for<'a> Deserialize<'a> + Into<super::KlodScene>,
{
    let file = std::fs::File::open(&scene_path)?;
    let scene: V = ron::de::from_reader(file)?;
    Ok(scene.into())
}

pub(super) fn migrate(scene_path: impl AsRef<Path>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let new_scene_format = Err(())
        .or_else(|_| try_load::<KlodSceneV1>(&scene_path))
        .or_else(|_| try_load::<KlodSceneV2>(&scene_path))?;
    let serialized = ron::ser::to_string_pretty(
        &new_scene_format,
        ron::ser::PrettyConfig::new()
            .indentor(" ".to_owned())
            .depth_limit(80),
    )?;
    std::fs::write(scene_path, serialized)?;
    Ok(())
}
