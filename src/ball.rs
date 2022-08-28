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

use self::anim::KlodVisualElem;
#[cfg(not(feature = "editor"))]
use crate::scene::reset_scene;
use crate::{
    cam::OrbitCamera, collision_groups as groups, powers::Power, prefabs::AggloBundle,
    state::GameState, system_helper::EasySystemSetCtor,
};

const BASE_INPUT_IMPULSE: f32 = 1.0;
const INPUT_WEIGHT_COMP: f32 = 0.5;
const KLOD_INITIAL_WEIGHT: f32 = 4.2;
const KLOD_INITIAL_RADIUS: f32 = 1.0;
pub(crate) const MAX_KLOD_SPEED: f32 = 28.0;

#[derive(SystemLabel)]
pub(crate) enum BallSystems {
    FreeFallUpdate,
    DestroyKlod,
    ResetKlod,
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
    pub(crate) scene: Option<Entity>,
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
fn spawn_ball(cmds: &mut ChildBuilder) {
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
}

fn reset_klod(
    mut cmds: Commands,
    klod_exists: Query<(), With<Klod>>,
    mut klod_entity: Query<(Entity, &mut Klod, &mut Velocity)>,
    cam: Query<Entity, With<KlodCamera>>,
    asset_server: Res<AssetServer>,
    spawn_point: Res<KlodSpawnTransform>,
    other_klod_elems: Query<Entity, Or<(With<KlodElem>, With<KlodVisualElem>)>>,
) -> Option<()> {
    if klod_exists.is_empty() {
        spawn_klod(cmds, klod_exists, cam, asset_server, spawn_point)
    } else {
        let (klod, mut klod_value, mut klod_velocity) = klod_entity.get_single_mut().ok()?;
        klod_value.weight = KLOD_INITIAL_WEIGHT;
        *klod_velocity = default();
        other_klod_elems.for_each(|entity| {
            cmds.entity(entity).despawn_recursive();
        });
        cmds.entity(klod)
            .insert(spawn_point.0)
            .add_children(|cmds| {
                spawn_ball(cmds);
                anim::spawn_klod_visuals(cmds, &asset_server);
            });
    }
    Some(())
}

fn spawn_klod(
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
            spawn_ball(cmds);
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
                .insert_bundle((trans, KlodVisualElem));
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
    gp_axis: Res<Axis<GamepadAxis>>,
    gp_buttons: Res<Input<GamepadButton>>,
    mut klod: Query<(&mut ExternalImpulse, &mut Velocity, &Klod)>,
    camera: Query<&OrbitCamera>,
    time: Res<Time>,
    mut pound_timeout: Local<f64>,
) {
    use KeyCode::{A, D, S, W};

    let (mut impulse, mut velocity, klod) = match klod.get_single_mut() {
        Ok(impulse) => impulse,
        Err(_) => {
            screen_print!(col: Color::RED, "BAD!!!!!!");
            return;
        }
    };
    let gp_axis_kind = |axis_type| GamepadAxis { gamepad: Gamepad { id: 0 }, axis_type };
    let gp_button = |button_type| GamepadButton { gamepad: Gamepad { id: 0 }, button_type };
    let axis_x = gp_axis_kind(GamepadAxisType::LeftStickX);
    let axis_y = gp_axis_kind(GamepadAxisType::LeftStickY);
    let gp_y_force = gp_axis.get(axis_y).map_or(default(), |y| Vec2::Y * y);
    let gp_x_force = gp_axis.get(axis_x).map_or(default(), |x| -Vec2::X * x);
    let gp_force = gp_x_force + gp_y_force;
    let cam_rot = camera.single();
    let vel = velocity.linvel;
    let additional_weight = klod.weight - KLOD_INITIAL_WEIGHT;
    let force = BASE_INPUT_IMPULSE + additional_weight * INPUT_WEIGHT_COMP;
    let force = |key, dir| if keys.pressed(key) { dir * force } else { Vec2::ZERO };
    let force = if gp_force.length_squared() < 0.01 {
        force(W, Vec2::Y) + force(S, -Vec2::Y) + force(A, Vec2::X) + force(D, -Vec2::X)
    } else {
        gp_force * 1.2
    };
    let force = Vec2::from_angle(-cam_rot.horizontal_rotation()).rotate(force);
    let max_more_force = MAX_KLOD_SPEED - vel.y;
    let force = (vel.xz() + force).clamp_length_max(max_more_force) - vel.xz();
    impulse.impulse = Vec3::new(force.x, 0.0, force.y);

    let gp_a = gp_button(GamepadButtonType::South);
    let ground_pound = keys.just_pressed(KeyCode::Space) || gp_buttons.just_pressed(gp_a);
    if ground_pound && time.seconds_since_startup() > *pound_timeout {
        *pound_timeout = time.seconds_since_startup() + 3.0;
        velocity.linvel.y -= 50.0;
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

macro_rules! err_sys {
    ($system:expr) => {
        $system.chain(|_| {})
    };
}

pub(crate) struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<Klod>()
            .register_inspectable::<Agglomerable>();

        // No idea why, but this system crashes the game when editor feature is enabled
        #[cfg(not(feature = "editor"))]
        app.add_system_set(GameState::Playing.on_enter(reset_scene.exclusive_system().at_start()));

        app.init_resource::<KlodSpawnTransform>()
            .add_event::<AgglomerateToKlod>()
            .add_event::<anim::DestroyKlodEvent>()
            .add_startup_system(spawn_camera)
            .add_system_set(GameState::Playing.on_exit(lock_camera))
            .add_system_set(
                GameState::Playing
                    .on_enter(unlock_camera)
                    .with_system(err_sys!(reset_klod).label(BallSystems::ResetKlod)),
            )
            .add_system_set(
                GameState::Playing
                    .on_update(ball_input)
                    .with_system(anim::destroy_klod.label(BallSystems::DestroyKlod))
                    .with_system(set_freefall.label(BallSystems::FreeFallUpdate))
                    .with_system(shlurp_agglomerable)
                    .with_system(agglo_to_klod.after(shlurp_agglomerable)),
            );
    }
}
