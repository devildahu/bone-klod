use bevy::{
    ecs::{system::EntityCommands, world::EntityRef},
    prelude::*,
};
use bevy_scene_hook::{HookedSceneBundle, SceneHook, SceneHooked};

use crate::{
    audio::ImpactSound,
    powers::Power,
    prefabs::{AggloData, Scenery, SerdeCollider},
    scene::{save_scene, ObjectType, PhysicsObject},
};

pub(crate) fn load_box_level(
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<AssetServer>,
) {
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

pub(crate) fn save_box_level(world: &mut World) {
    let mut hooked = world.query_filtered::<(Entity, &Name), With<SceneHooked>>();
    let hooked = hooked
        .iter(world)
        .find_map(|(entity, n)| (n.as_str() == "graybox").then(|| entity));
    if let Some(hooked) = hooked {
        save_scene(world);
        world
            .entity_mut(hooked)
            .remove_bundle::<(SceneHook, SceneHooked)>();
    }
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
        let name_any_of = |names: &[&str]| names.iter().any(|n| name.starts_with(n));
        let collider = match () {
            () if name_any_of(&["Cube", "Torch", "Shovel", "Flask", "SpecialDoor", "Bone"]) => {
                SerdeCollider::Cuboid { half_extents: Vec3::splat(1.0) }
            }
            () if name.starts_with("Sphere") => SerdeCollider::Ball { radius: 1.0 },
            () => return None,
        };
        let agglo = |power| ObjectType::Agglomerable(AggloData::new(0.1, power));
        let object = match () {
            () if name.starts_with("Bone") => agglo(Power::None),
            () if name.starts_with("Shovel") => agglo(Power::Dig),
            () if name.starts_with("Torch") => agglo(Power::Fire),
            () if name.starts_with("Flask") => agglo(Power::Water),
            () if name.starts_with("SpecialDoor") => {
                ObjectType::Scenery(Scenery { weakness: vec![Power::None] })
            }
            _ => ObjectType::Scenery(Scenery { weakness: vec![] }),
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
            object,
        );
        data.spawn_light(cmds);
        cmds.remove::<Parent>();
        Some(())
    };
    run();
}
