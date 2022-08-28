use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{animate::Animate, collision_groups as groups};

use super::{KlodBall, KlodElem};

#[derive(Component)]
pub(super) struct KlodVisualElem;

pub(crate) struct DestroyKlodEvent;

pub(super) fn spawn_klod_visuals(cmds: &mut ChildBuilder, assets: &AssetServer) {
    let handle = assets.load("hand_greybox.glb#Scene0");
    for target in [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::Z,
        Vec3::NEG_Z,
    ] {
        let rotation = Quat::from_rotation_arc(Vec3::Y, target);
        let target = target * super::KLOD_INITIAL_RADIUS * 0.8;
        cmds.spawn_bundle(SceneBundle {
            scene: handle.clone(),
            transform: Transform {
                translation: target * 10.0,
                rotation,
                scale: Vec3::ONE * 0.7,
            },
            ..default()
        })
        .insert_bundle((
            Name::new("HandPart"),
            Animate::MoveToward { target, speed: 10.0 },
            KlodVisualElem,
        ));
    }
}

// TODO: deparent the camera as well
pub(super) fn destroy_klod(
    mut cmds: Commands,
    klod_visuals: Query<(Entity, &Transform, &GlobalTransform, &Parent), With<KlodVisualElem>>,
    klod_elems: Query<(
        Entity,
        &Collider,
        &Transform,
        &GlobalTransform,
        &Parent,
        &KlodElem,
    )>,
    mut destroy_events: EventReader<DestroyKlodEvent>,
) {
    if destroy_events.iter().next().is_none() {
        return;
    }
    for (entity, transform, global_transform, parent) in &klod_visuals {
        cmds.entity(parent.get()).remove_children(&[entity]);
        cmds.entity(entity).insert_bundle((
            groups::KLOD,
            Collider::cuboid(1.0, 0.5, 1.0),
            global_transform.compute_transform(),
            Velocity { linvel: transform.translation * 3.0, ..default() },
            RigidBody::Dynamic,
        ));
    }
    for (entity, collider, transform, global_transform, parent, elem) in &klod_elems {
        cmds.entity(entity).despawn();
        if let Some(entity) = elem.scene {
            cmds.entity(parent.get()).remove_children(&[entity]);
            cmds.entity(entity).insert_bundle((
                groups::KLOD,
                global_transform.compute_transform(),
                Velocity { linvel: transform.translation * 10.0, ..default() },
                RigidBody::Dynamic,
                collider.clone(),
            ));
        }
    }
}
