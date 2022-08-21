use bevy::prelude::{Plugin as BevyPlugin, *};
use bevy_rapier3d::prelude::*;

use crate::state::GameState;

#[derive(Component)]
struct Klod {
    weight: f32,
}

#[derive(Component)]
struct KlodElem {
    klod: Entity,
}

pub(crate) fn spawn_klod(cmds: &mut Commands, position: Vec3) {
    cmds.spawn_bundle((Klod { weight: 10.0 }, RigidBody::Dynamic, Name::new("Klod")))
        .insert_bundle(SpatialBundle::from_transform(Transform::from_translation(
            position,
        )))
        .with_children(|cmds| {
            cmds.spawn_bundle(SpatialBundle::default())
                .insert_bundle((Collider::ball(3.0), ColliderMassProperties::Mass(10.0)));
        });
}

struct AgglomerateToKlod {
    klod: Entity,
    agglo: Entity,
    agglo_weight: f32,
}

#[derive(Component)]
struct Agglomerable {
    weight: f32,
}

#[derive(Bundle)]
struct AggloBundle {
    agglo: Agglomerable,
    active_events: ActiveEvents,
    collider: Collider,
    mass: ColliderMassProperties,
    rigid_body: RigidBody,
    contact_threshold: ContactForceEventThreshold,
}
impl AggloBundle {
    pub(crate) fn new(mass: f32, collider: Collider) -> Self {
        AggloBundle {
            agglo: Agglomerable { weight: mass },
            active_events: ActiveEvents::CONTACT_FORCE_EVENTS,
            collider,
            mass: ColliderMassProperties::Mass(mass),
            rigid_body: RigidBody::Dynamic,
            contact_threshold: ContactForceEventThreshold(mass),
        }
    }
}
fn transform_relative_to(point: &GlobalTransform, reference: &GlobalTransform) -> Transform {
    let relative_affine = point.affine() * reference.affine().inverse();
    let (scale, rotation, translation) = relative_affine.to_scale_rotation_translation();
    Transform { translation, rotation, scale }
}

fn agglo_to_klod(
    mut cmds: Commands,
    mut events: EventReader<AgglomerateToKlod>,
    mut klod_query: Query<&mut Klod>,
    transforms: Query<&GlobalTransform>,
) {
    for &AgglomerateToKlod { klod, agglo, agglo_weight } in events.iter() {
        let agglo_trans = transforms.get(agglo).unwrap();
        let klod_trans = transforms.get(klod).unwrap();
        cmds.entity(agglo)
            .insert(transform_relative_to(agglo_trans, klod_trans))
            .remove_bundle::<(
                Agglomerable,
                RigidBody,
                ActiveEvents,
                ContactForceEventThreshold,
            )>();
        cmds.entity(klod).add_child(agglo);
        if let Ok(mut klod_component) = klod_query.get_mut(klod) {
            klod_component.weight += agglo_weight;
        }
    }
}
fn shlurp_agglomerable(
    klod: Query<AnyOf<(&KlodElem, &Klod)>>,
    agglo: Query<&Agglomerable>,
    mut events: EventWriter<AgglomerateToKlod>,
    mut collisions: EventReader<ContactForceEvent>,
) {
    for ContactForceEvent { collider1, collider2, .. } in collisions.iter() {
        let klod = match klod.get(*collider2) {
            Ok((Some(elem), None)) => elem.klod,
            Ok((None, Some(_))) => *collider2,
            _ => continue,
        };
        let agglo_weight = agglo.get(*collider1).unwrap().weight;
        events.send(AgglomerateToKlod { klod, agglo_weight, agglo: *collider1 });
    }
}

fn spawn_debug_scene(
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    // Plane
    cmds.spawn_bundle(PbrBundle {
        material: mats.add(StandardMaterial { base_color: Color::WHITE, ..default() }),
        mesh: meshes.add(shape::Box::new(200.0, 2.0, 200.0).into()),
        ..default()
    })
    .insert(Name::new("Plane"))
    .insert_bundle((RigidBody::Fixed, Collider::cuboid(100.0, 1.0, 100.0)));

    // Ball
    cmds.spawn_bundle(PbrBundle {
        material: mats.add(StandardMaterial { base_color: Color::RED, ..default() }),
        mesh: meshes.add(shape::Icosphere::default().into()),
        transform: Transform::from_xyz(-5.0, 3.0, -5.0),
        ..default()
    })
    .insert(Name::new("Red Ball"))
    .insert_bundle(AggloBundle::new(2.0, Collider::ball(1.0)));

    // Cube
    cmds.spawn_bundle(PbrBundle {
        material: mats.add(StandardMaterial { base_color: Color::GREEN, ..default() }),
        mesh: meshes.add(shape::Box::new(2.0, 2.0, 2.0).into()),
        transform: Transform::from_xyz(5.0, 3.0, 5.0),
        ..default()
    })
    .insert(Name::new("Green Cube"))
    .insert_bundle(AggloBundle::new(2.0, Collider::cuboid(1.0, 1.0, 1.0)));

    // Klod
    spawn_klod(&mut cmds, Vec3::new(0.0, 3.0, 0.0));

    // Camera
    cmds.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-10.0, 2.5, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn ball_input(keyboard: Res<Input<KeyCode>>) {}
pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(shlurp_agglomerable)
                .with_system(agglo_to_klod.after(shlurp_agglomerable)),
        )
        .add_startup_system(spawn_debug_scene);
    }
}
