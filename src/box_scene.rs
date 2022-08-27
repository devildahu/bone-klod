use bevy::{
    ecs::{system::EntityCommands, world::EntityRef},
    prelude::*,
};
use bevy_rapier3d::prelude::RapierConfiguration;
use bevy_scene_hook::{HookedSceneBundle, SceneHook};

use crate::{
    audio::ImpactSound,
    prefabs::{Scenery, SerdeCollider},
    scene::{ObjectType, PhysicsObject},
};

pub(crate) fn load_box_level(
    mut cmds: Commands,
    assets: Res<AssetServer>,
    // mut rapier_config: ResMut<RapierConfiguration>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // rapier_config.physics_pipeline_active = false;
    cmds.spawn_bundle(HookedSceneBundle {
        scene: SceneBundle {
            scene: assets.load("import/hitboxes.glb#Scene0"),
            ..default()
        },
        hook: SceneHook::new(hook),
    });
    let data = PhysicsObject::new(
        "graybox".to_owned(),
        Some("import/hitboxes.glb#Scene0".to_owned()),
        default(),
        SerdeCollider::Ball { radius: 1.0 },
        0.8,
        0.1,
        vec![],
        ObjectType::Scenery(Scenery { weakness: vec![] }),
    );
    data.spawn(&mut cmds, &assets, &mut meshes, false);
}

fn hook(entity: &EntityRef, cmds: &mut EntityCommands) {
    let mut run = || {
        if let Some(mesh) = entity.get::<Handle<Mesh>>().clone() {
            let this = entity.id();
            let parent = entity.get::<Parent>()?.get();
            let mut parent = cmds.commands().entity(parent);
            parent.insert(mesh.clone()).remove_children(&[this]);
            cmds.despawn();
            return None;
        }
        let name = entity.get::<Name>()?.as_str();
        let mut transform = *entity.get::<Transform>()?;
        let collider = match () {
            () if name.starts_with("Cube") => {
                SerdeCollider::Cuboid { half_extents: Vec3::splat(1.0) }
            }
            () if name.starts_with("Sphere") => SerdeCollider::Ball { radius: 1.0 },
            () => return None,
        };
        transform.scale = transform.scale.abs();
        let data = PhysicsObject::new(
            name.to_string(),
            None,
            transform,
            collider,
            0.8,
            0.1,
            vec![ImpactSound::GenericMetal],
            ObjectType::Scenery(Scenery { weakness: vec![] }),
        );
        data.spawn_light(cmds);
        cmds.remove::<Parent>();
        Some(())
    };
    run();
}
