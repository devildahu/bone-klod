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
        super::KlodScene {
            klod_spawn_transform: v1.klod_spawn_transform,
            objects: v1.objects,
            music_triggers: v1.music_triggers,
            finish_zone: super::FinishZone {
                collider: SerdeCollider::Cuboid { half_extents: Vec3::ONE * 5.0 },
                transform: Default::default(),
            },
            game_timer_seconds: 1.5 * 60.0,
        }
    }
}

pub(super) fn migrate(scene_path: impl AsRef<Path>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let file = std::fs::File::open(&scene_path)?;
    let scene: KlodSceneV1 = ron::de::from_reader(file)?;
    let new_scene_format: super::KlodScene = scene.into();
    let serialized = ron::ser::to_string_pretty(
        &new_scene_format,
        ron::ser::PrettyConfig::new()
            .indentor(" ".to_owned())
            .depth_limit(80),
    )?;

    std::fs::write(scene_path, serialized)?;
    Ok(())
}
