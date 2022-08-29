use std::fmt;

use bevy::{
    prelude::{Plugin as BevyPlugin, *},
    utils::HashMap,
};
use bevy_debug_text_overlay::screen_print;
#[cfg(feature = "debug")]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ball::KlodElem;

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component, Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(crate) enum Power {
    Fire,
    Water,
    Cat,
    AmberRod,
    Dig,
    Saw,
    #[default]
    None,
}
impl fmt::Display for Power {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Power::Fire => write!(f, "Fire"),
            Power::Water => write!(f, "Water"),
            Power::Cat => write!(f, "Cat"),
            Power::AmberRod => write!(f, "AmberRod"),
            Power::Dig => write!(f, "Dig"),
            Power::Saw => write!(f, "Saw"),
            Power::None => write!(f, "None"),
        }
    }
}
#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component, Serialize, Deserialize)]
pub(crate) struct ElementalObstacle {
    pub(crate) required_powers: Vec<Power>,
}

fn break_elemental_obstacle(
    kloded: Query<(&Power, Entity, &KlodElem)>,
    obstacles: Query<&ElementalObstacle>,
    mut collisions: EventReader<ContactForceEvent>,
    mut cmds: Commands,
) {
    for ContactForceEvent { collider1, collider2, .. } in collisions.iter() {
        let obstacle_entity = match (kloded.contains(*collider1), kloded.contains(*collider2)) {
            (true, _) => *collider2,
            (_, true) => *collider1,
            _ => continue,
        };
        if let Ok(obstacle) = obstacles.get(obstacle_entity) {
            let kloded: HashMap<_, _> = kloded
                .iter()
                .map(|(power, entity, elem)| (*power, (entity, elem.scene)))
                .collect();
            let destroys_obstacle: Vec<_> = obstacle
                .required_powers
                .iter()
                .filter_map(|power| kloded.get(power))
                .collect();
            if !destroys_obstacle.is_empty() {
                screen_print!(
                    sec: 4.0,
                    col: Color::MAROON,
                    "Destroyed obstacle with powers: {:?}",
                    obstacle.required_powers
                );

                for &&(elem, scene) in &destroys_obstacle {
                    cmds.entity(elem).despawn_recursive();
                    if let Some(scene) = scene {
                        cmds.get_or_spawn(scene).despawn_recursive();
                    }
                }
                cmds.entity(obstacle_entity).despawn_recursive();
            }
        }
    }
}
pub(crate) struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<Power>()
            .register_inspectable::<ElementalObstacle>();

        app.add_system(break_elemental_obstacle);
    }
}
