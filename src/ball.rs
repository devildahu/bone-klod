use bevy::prelude::{Plugin as BevyPlugin, *};
use bevy_debug_text_overlay::screen_print;
#[cfg(feature = "debug")]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::*;

use crate::{cam::OrbitCamera, state::GameState};

const INPUT_IMPULSE: f32 = 3.0;
const KLOD_COLLISION_GROUP: CollisionGroups = CollisionGroups::new(0b0100, !0b0100);
const AGGLO_COLLISION_GROUP: CollisionGroups = CollisionGroups::new(0b1000, !0);

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component)]
struct Klod {
    weight: f32,
}

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component)]
struct KlodElem {
    klod: Entity,
}

#[derive(Bundle)]
struct KlodElemBundle {
    elem: KlodElem,
    collision_group: CollisionGroups,
    collider: Collider,
    mass: ColliderMassProperties,
    friction: Friction,
    restitution: Restitution,
    transform: Transform,
}
impl KlodElemBundle {
    fn new(
        klod: Entity,
        mass: f32,
        collider: Collider,
        transform: Transform,
        friction: Friction,
        restitution: Restitution,
    ) -> Self {
        Self {
            elem: KlodElem { klod },
            collision_group: KLOD_COLLISION_GROUP,
            collider,
            mass: ColliderMassProperties::Mass(mass),
            friction,
            restitution,
            transform,
        }
    }
}

pub(crate) fn spawn_klod(cmds: &mut Commands, position: Vec3) -> Entity {
    let transform = Transform::from_translation(position);
    cmds.spawn_bundle((
        Klod { weight: 10.0 },
        RigidBody::Dynamic,
        ExternalImpulse::default(),
        Velocity::default(),
        Name::new("Klod"),
        KLOD_COLLISION_GROUP,
    ))
    .insert_bundle(SpatialBundle::from_transform(transform))
    .with_children(|cmds| {
        let klod = cmds.parent_entity();
        cmds.spawn_bundle(KlodElemBundle::new(
            klod,
            10.0,
            Collider::ball(3.0),
            default(),
            Friction {
                coefficient: 0.9,
                combine_rule: CoefficientCombineRule::Min,
            },
            Restitution {
                coefficient: 0.9,
                combine_rule: CoefficientCombineRule::Min,
            },
        ));
    })
    .id()
}

struct AgglomerateToKlod {
    klod: Entity,
    agglo: Entity,
    agglo_weight: f32,
}

#[cfg_attr(feature = "debug", derive(Inspectable))]
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
    collision_group: CollisionGroups,
}
impl AggloBundle {
    pub(crate) fn new(mass: f32, collider: Collider) -> Self {
        AggloBundle {
            agglo: Agglomerable { weight: mass },
            active_events: ActiveEvents::CONTACT_FORCE_EVENTS,
            collider,
            mass: ColliderMassProperties::Mass(mass),
            rigid_body: RigidBody::Dynamic,
            contact_threshold: ContactForceEventThreshold(mass * 1000.0),
            collision_group: AGGLO_COLLISION_GROUP,
        }
    }
}
fn transform_relative_to(point: &GlobalTransform, reference: &GlobalTransform) -> Transform {
    let relative_affine = reference.affine().inverse() * point.affine();
    let (scale, rotation, translation) = relative_affine.to_scale_rotation_translation();
    Transform { translation, rotation, scale }
}

fn agglo_to_klod(
    mut cmds: Commands,
    mut events: EventReader<AgglomerateToKlod>,
    agglo_query: Query<
        (
            &Collider,
            &GlobalTransform,
            Option<&Friction>,
            Option<&Restitution>,
        ),
        With<Agglomerable>,
    >,
    mut klod_query: Query<&mut Klod>,
    transforms: Query<&GlobalTransform>,
) {
    for &AgglomerateToKlod { klod, agglo, agglo_weight } in events.iter() {
        let klod_trans = transforms.get(klod).unwrap();
        let (coll, agglo_trans, friction, restitution) = match agglo_query.get(agglo) {
            Ok(item) => item,
            _ => continue,
        };
        let trans = transform_relative_to(agglo_trans, klod_trans);
        cmds.entity(agglo)
            .insert_bundle((KlodElem { klod }, trans))
            .remove_bundle::<AggloBundle>();
        cmds.entity(klod).add_child(agglo);
        screen_print!("added {agglo:?} to klod {klod:?}");
        if let Ok(mut klod_component) = klod_query.get_mut(klod) {
            klod_component.weight += agglo_weight;

            cmds.entity(klod).add_children(|cmds| {
                cmds.spawn_bundle(KlodElemBundle::new(
                    klod,
                    agglo_weight,
                    coll.clone(),
                    trans,
                    friction.cloned().unwrap_or_default(),
                    restitution.cloned().unwrap_or_default(),
                ));
            });
        }
    }
}
fn shlurp_agglomerable(
    klod: Query<&KlodElem>,
    agglo: Query<(&Agglomerable, Option<&Name>)>,
    mut events: EventWriter<AgglomerateToKlod>,
    mut collisions: EventReader<ContactForceEvent>,
) {
    for ContactForceEvent { collider1, collider2, .. } in collisions.iter() {
        screen_print!(sec: 1.0, "detected collision between {collider1:?} and {collider2:?}");
        let (klod, agglo_entity) = match (klod.get(*collider1), klod.get(*collider2)) {
            (Ok(elem), _) => (elem.klod, *collider2),
            (_, Ok(elem)) => (elem.klod, *collider1),
            _ => continue,
        };
        if let Ok((agglo, name)) = agglo.get(agglo_entity) {
            let name = name.map_or("something, certainly".to_owned(), |s| s.to_string());
            screen_print!("Shlurped {name}");
            events.send(AgglomerateToKlod {
                klod,
                agglo_weight: agglo.weight,
                agglo: agglo_entity,
            });
        }
    }
}

fn spawn_debug_scene(
    mut cmds: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    // Plane
    cmds.spawn_bundle(PbrBundle {
        material: mats.add(StandardMaterial {
            base_color_texture: Some(assets.load("garbage.png")),
            perceptual_roughness: 0.6,
            metallic: 0.6,
            ..default()
        }),
        mesh: meshes.add(shape::Box::new(200.0, 2.0, 200.0).into()),
        ..default()
    })
    .insert(Name::new("Plane"))
    .insert_bundle((RigidBody::Fixed, Collider::cuboid(100.0, 1.0, 100.0)));

    // Ball
    let ball = cmds
        .spawn_bundle(PbrBundle {
            material: mats.add(StandardMaterial { base_color: Color::RED, ..default() }),
            mesh: meshes.add(shape::Icosphere::default().into()),
            transform: Transform::from_xyz(-5.0, 3.0, -5.0),
            ..default()
        })
        .insert(Name::new("Red Ball"))
        .insert_bundle(AggloBundle::new(200.0, Collider::ball(1.0)))
        .id();

    // Cube
    let cube = cmds
        .spawn_bundle(PbrBundle {
            material: mats.add(StandardMaterial { base_color: Color::GREEN, ..default() }),
            mesh: meshes.add(shape::Box::new(2.0, 2.0, 2.0).into()),
            transform: Transform::from_xyz(5.0, 3.0, 5.0),
            ..default()
        })
        .insert(Name::new("Green Cube"))
        .insert_bundle(AggloBundle::new(2.0, Collider::cuboid(1.0, 1.0, 1.0)))
        .id();
    screen_print!("{ball:?} and {cube:?}");
    println!("{ball:?} and {cube:?}");

    let klod = spawn_klod(&mut cmds, Vec3::new(0.0, 3.0, 0.0));

    // Camera
    cmds.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-10.0, 2.5, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    })
    .insert(OrbitCamera::follows(klod));
}

fn ball_input(
    keyboard: Res<Input<KeyCode>>,
    mut klod: Query<(&mut Transform, &mut ExternalImpulse, &mut Velocity), With<Klod>>,
    camera: Query<&OrbitCamera>,
) {
    use KeyCode::{A, D, S, W};

    let (mut transform, mut impulse, mut velocity) = match klod.get_single_mut() {
        Ok(impulse) => impulse,
        Err(_) => {
            screen_print!(col: Color::RED, "BAD!!!!!!");
            return;
        }
    };
    let cam_rot = camera.single();
    let force = |key_code, dir| {
        if keyboard.pressed(key_code) {
            dir * INPUT_IMPULSE
        } else {
            Vec2::ZERO
        }
    };
    let force = force(W, Vec2::Y) + force(S, -Vec2::Y) + force(A, Vec2::X) + force(D, -Vec2::X);
    let force = Vec2::from_angle(-cam_rot.horizontal_rotation()).rotate(force);
    impulse.impulse = Vec3::new(force.x, 0.0, force.y);
    // TODO: Q => lock camera movement
    if keyboard.just_pressed(KeyCode::Space) {
        *transform = Transform::from_xyz(0.0, 3.0, 0.0);
        *velocity = Velocity::default();
    }
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<Klod>()
            .register_inspectable::<KlodElem>()
            .register_inspectable::<Agglomerable>();

        app.add_event::<AgglomerateToKlod>()
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    .with_system(ball_input)
                    .with_system(shlurp_agglomerable)
                    .with_system(agglo_to_klod.after(shlurp_agglomerable)),
            )
            .add_startup_system(spawn_debug_scene);
    }
}
