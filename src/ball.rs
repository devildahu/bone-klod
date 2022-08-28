pub(crate) mod anim;

use bevy::{
    ecs::system::EntityCommands,
    math::Vec3Swizzles,
    prelude::{Plugin as BevyPlugin, *},
};
use bevy_debug_text_overlay::screen_print;
#[cfg(feature = "debug")]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::*;

use crate::{
    cam::OrbitCamera, collision_groups as groups, powers::Power, prefabs::AggloBundle,
    state::GameState, system_helper::EasySystemSetCtor,
};

const BASE_INPUT_IMPULSE: f32 = 1.0;
const KLOD_INITIAL_WEIGHT: f32 = 4.2;
const KLOD_INITIAL_RADIUS: f32 = 1.0;
pub(crate) const MAX_KLOD_SPEED: f32 = 28.0;

#[derive(SystemLabel)]
pub(crate) enum BallSystems {
    FreeFallUpdate,
    DestroyKlod,
}

#[derive(Component)]
pub(crate) struct KlodCamera;

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component)]
pub(crate) struct Klod {
    weight: f32,
}
impl Klod {
    fn within_radius(&self, distance: f32) -> bool {
        let max_distance = self.weight / KLOD_INITIAL_WEIGHT;
        let can_slurp = distance < max_distance;
        let color = if can_slurp { Color::GREEN } else { Color::RED };
        screen_print!(col: color, "slurp dist: {distance:.3} <? {max_distance:.3}");
        can_slurp
    }
    fn can_slurp(&self, weight: f32, velocity: Vec3) -> bool {
        let speed_bonus = (velocity.length() * 1.2 / MAX_KLOD_SPEED).max(0.5);
        let weight_limit = self.weight / 10.0;
        let can_slurp = weight < speed_bonus * weight_limit;
        let color = if can_slurp { Color::GREEN } else { Color::RED };
        screen_print!(
            col: color,
            "slurp: {weight:.3} <? {speed_bonus:.3} * {weight_limit:.3}"
        );
        can_slurp
    }

    pub(crate) fn weight(&self) -> f32 {
        (self.weight - KLOD_INITIAL_WEIGHT) * 10.0
    }
}
#[derive(Component)]
pub(crate) struct KlodBall;

#[derive(Component)]
pub(crate) struct KlodElem {
    klod: Entity,
    scene: Option<Entity>,
}

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component)]
pub(crate) struct FreeFall(pub(crate) bool);

#[derive(Default)]
pub(crate) struct KlodSpawnTransform(pub(crate) Transform);

fn spawn_klod_elem<'w, 's, 'a>(
    cmds: &'a mut ChildBuilder<'w, 's, '_>,
    name: String,
    klod_elem: KlodElem,
    mass: f32,
    collider: Collider,
    transform: Transform,
    friction: Friction,
    restitution: Restitution,
    power: Power,
) -> EntityCommands<'w, 's, 'a> {
    cmds.spawn_bundle((
        klod_elem,
        Name::new(name),
        groups::KLOD,
        ActiveEvents::CONTACT_FORCE_EVENTS,
        ContactForceEventThreshold(1000.0),
        collider,
        ColliderMassProperties::Mass(mass),
        friction,
        restitution,
        transform,
        GlobalTransform::default(),
        power,
    ))
}

pub(crate) fn spawn_klod(
    mut cmds: Commands,
    klod_exists: Query<(), With<Klod>>,
    cam: Query<Entity, With<KlodCamera>>,
    asset_server: Res<AssetServer>,
    spawn_point: Res<KlodSpawnTransform>,
) {
    if !klod_exists.is_empty() {
        return;
    }
    let cam = match cam.get_single() {
        Ok(cam) => cam,
        Err(_) => return,
    };
    let klod = cmds
        .spawn_bundle((
            Klod { weight: KLOD_INITIAL_WEIGHT },
            FreeFall(true),
            RigidBody::Dynamic,
            ExternalImpulse::default(),
            Velocity::default(),
            Name::new("Klod"),
            groups::KLOD,
        ))
        .insert_bundle(SpatialBundle::from_transform(spawn_point.0))
        .with_children(|cmds| {
            let klod = cmds.parent_entity();
            let mut ball = spawn_klod_elem(
                cmds,
                "Klod ball".to_owned(),
                KlodElem { klod, scene: None },
                KLOD_INITIAL_WEIGHT,
                Collider::ball(KLOD_INITIAL_RADIUS),
                default(),
                Friction {
                    coefficient: 0.0,
                    combine_rule: CoefficientCombineRule::Max,
                },
                Restitution {
                    coefficient: 0.0,
                    combine_rule: CoefficientCombineRule::Max,
                },
                Power::None,
            );
            ball.insert(KlodBall);
            anim::spawn_klod_visuals(cmds, &asset_server);
        })
        .id();
    cmds.entity(cam).insert(OrbitCamera::follows(klod));
}

struct AgglomerateToKlod {
    klod: Entity,
    agglo: Entity,
    agglo_weight: f32,
}

/// Thing that can be klodded.
#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component)]
pub(crate) struct Agglomerable {
    pub(crate) weight: f32,
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
            &Power,
            &Friction,
            &Restitution,
            Option<&Name>,
        ),
        With<Agglomerable>,
    >,
    mut klod_query: Query<(&mut Klod, &Velocity)>,
    transforms: Query<&GlobalTransform>,
) {
    for &AgglomerateToKlod { klod, agglo, agglo_weight } in events.iter() {
        if let Ok((mut klod_data, klod_velocity)) = klod_query.get_mut(klod) {
            let klod_trans = transforms.get(klod).unwrap();
            let (coll, agglo_trans, power, friction, restitution, name) =
                match agglo_query.get(agglo) {
                    Ok(item) => item,
                    _ => continue,
                };
            let mut trans = transform_relative_to(agglo_trans, klod_trans);
            trans.translation = trans.translation * 0.8;
            let within_radius = || {
                let distance_to_center = trans.translation.length();
                klod_data.within_radius(distance_to_center)
            };
            let can_slurp = || klod_data.can_slurp(agglo_weight, klod_velocity.linvel);
            if !within_radius() || !can_slurp() {
                continue;
            }
            cmds.entity(agglo)
                .remove_bundle::<AggloBundle>()
                .remove_bundle::<(Collider, Friction, Restitution)>()
                .insert(trans);
            cmds.entity(klod).add_child(agglo);
            klod_data.weight += agglo_weight;

            let name = name.map_or("Klod elem".to_owned(), |name| name.to_string() + " elem");
            screen_print!("added {name} to klod {klod:?}");
            cmds.entity(klod).add_children(|cmds| {
                spawn_klod_elem(
                    cmds,
                    name,
                    KlodElem { klod, scene: Some(agglo) },
                    agglo_weight,
                    coll.clone(),
                    trans,
                    *friction,
                    *restitution,
                    *power,
                );
            });
        }
    }
}
fn shlurp_agglomerable(
    klod: Query<&KlodElem>,
    agglo: Query<&Agglomerable>,
    mut events: EventWriter<AgglomerateToKlod>,
    mut collisions: EventReader<ContactForceEvent>,
) {
    for ContactForceEvent { collider1, collider2, .. } in collisions.iter() {
        let (klod, agglo_entity) = match (klod.get(*collider1), klod.get(*collider2)) {
            (Ok(elem), _) => (elem.klod, *collider2),
            (_, Ok(elem)) => (elem.klod, *collider1),
            _ => continue,
        };
        if let Ok(agglo) = agglo.get(agglo_entity) {
            events.send(AgglomerateToKlod {
                klod,
                agglo_weight: agglo.weight,
                agglo: agglo_entity,
            });
        }
    }
}

fn ball_input(
    keys: Res<Input<KeyCode>>,
    mut default_klod_position: ResMut<KlodSpawnTransform>,
    mut klod: Query<(&mut Transform, &mut ExternalImpulse, &mut Velocity, &Klod)>,
    camera: Query<&OrbitCamera>,
) {
    use KeyCode::{A, D, S, W};

    let (mut transform, mut impulse, mut velocity, klod) = match klod.get_single_mut() {
        Ok(impulse) => impulse,
        Err(_) => {
            screen_print!(col: Color::RED, "BAD!!!!!!");
            return;
        }
    };
    let cam_rot = camera.single();
    let vel = velocity.linvel;
    let additional_weight = klod.weight - KLOD_INITIAL_WEIGHT;
    let force = BASE_INPUT_IMPULSE + additional_weight * 0.1;
    let force = |key, dir| if keys.pressed(key) { dir * force } else { Vec2::ZERO };
    let force = force(W, Vec2::Y) + force(S, -Vec2::Y) + force(A, Vec2::X) + force(D, -Vec2::X);
    let force = Vec2::from_angle(-cam_rot.horizontal_rotation()).rotate(force);
    let max_more_force = MAX_KLOD_SPEED - vel.y;
    let force = (vel.xz() + force).clamp_length_max(max_more_force) - vel.xz();
    impulse.impulse = Vec3::new(force.x, 0.0, force.y);

    if keys.just_pressed(KeyCode::Space) {
        *transform = default_klod_position.0;
        *velocity = Velocity::default();
    }
    if keys.just_pressed(KeyCode::R) {
        default_klod_position.0 = *transform;
    }
}

fn set_freefall(
    klod_elems: Query<Entity, With<KlodElem>>,
    mut klod: Query<&mut FreeFall, With<Klod>>,
    rapier_context: Res<RapierContext>,
) {
    let free_falling = |elem| {
        rapier_context
            .contacts_with(elem)
            .filter(|c| c.has_any_active_contacts())
            .next()
            .is_none()
    };
    let free_falling = klod_elems.iter().all(free_falling);
    if let Ok(mut component) = klod.get_single_mut() {
        if component.0 != free_falling {
            component.0 = free_falling;
        }
    }
}

fn lock_camera(mut query: Query<&mut OrbitCamera>) {
    for mut cam in &mut query {
        cam.locked = true;
    }
}
fn unlock_camera(mut query: Query<&mut OrbitCamera>) {
    for mut cam in &mut query {
        cam.locked = false;
    }
}
fn spawn_camera(
    klod_spawn: Res<KlodSpawnTransform>,
    mut cmds: Commands,
    existing_cam: Query<(), With<KlodCamera>>,
) {
    use bevy::math::EulerRot::XYZ;
    if !existing_cam.is_empty() {
        return;
    }
    cmds.spawn_bundle(Camera3dBundle {
        transform: Transform {
            translation: klod_spawn.0.translation + Vec3::new(-12.713, 6.149, -0.646),
            rotation: Quat::from_euler(XYZ, -1.676, -1.118, -1.687),
            scale: Vec3::ONE,
        },
        ..default()
    })
    .insert_bundle((Name::new("Klod Camera"), KlodCamera));
}

pub(crate) struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<Klod>()
            .register_inspectable::<Agglomerable>();

        app.init_resource::<KlodSpawnTransform>()
            .add_event::<AgglomerateToKlod>()
            .add_event::<anim::DestroyKlodEvent>()
            .add_startup_system(spawn_camera)
            .add_system_set(GameState::Playing.on_exit(lock_camera))
            .add_system_set(GameState::Playing.on_enter(unlock_camera))
            .add_system_set(GameState::Playing.on_enter(spawn_klod))
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    .with_system(ball_input)
                    .with_system(anim::destroy_klod.label(BallSystems::DestroyKlod))
                    .with_system(set_freefall.label(BallSystems::FreeFallUpdate))
                    .with_system(shlurp_agglomerable)
                    .with_system(agglo_to_klod.after(shlurp_agglomerable)),
            );
    }
}
