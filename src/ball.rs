use bevy::{
    math::Vec3Swizzles,
    prelude::{Plugin as BevyPlugin, *},
};
use bevy_debug_text_overlay::screen_print;
#[cfg(feature = "debug")]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::*;

use crate::{cam::OrbitCamera, powers::Power, prefabs::AggloBundle, state::GameState};

const INPUT_IMPULSE: f32 = 6.0;
const KLOD_COLLISION_GROUP: CollisionGroups = CollisionGroups::new(0b0100, !0b0100);
pub(crate) const MAX_KLOD_SPEED: f32 = 30.0;

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component)]
pub(crate) struct Klod {
    weight: f32,
}

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component)]
pub(crate) struct KlodElem {
    klod: Entity,
}

fn spawn_klod_elem(
    cmds: &mut ChildBuilder,
    klod: Entity,
    mass: f32,
    collider: Collider,
    transform: Transform,
    friction: Friction,
    restitution: Restitution,
    power: Power,
) {
    cmds.spawn_bundle((
        KlodElem { klod },
        KLOD_COLLISION_GROUP,
        ActiveEvents::CONTACT_FORCE_EVENTS,
        ContactForceEventThreshold(1000.0),
        collider,
        ColliderMassProperties::Mass(mass),
        friction,
        restitution,
        transform,
        power,
    ));
}

pub(crate) fn spawn_klod(cmds: &mut Commands, asset_server: &AssetServer) -> Entity {
    cmds.spawn_bundle((
        Klod { weight: 10.0 },
        RigidBody::Dynamic,
        ExternalImpulse::default(),
        Velocity::default(),
        Name::new("Klod"),
        KLOD_COLLISION_GROUP,
    ))
    .insert_bundle(SpatialBundle::default())
    .with_children(|cmds| {
        let klod = cmds.parent_entity();
        spawn_klod_elem(
            cmds,
            klod,
            90.0,
            Collider::ball(3.0),
            default(),
            Friction {
                coefficient: 0.9,
                combine_rule: CoefficientCombineRule::Max,
            },
            Restitution {
                coefficient: 0.4,
                combine_rule: CoefficientCombineRule::Max,
            },
            Power::None,
        );
        cmds.spawn_bundle(SceneBundle {
            scene: asset_server.load("klod.glb#Scene0"),
            transform: Transform::from_scale(Vec3::splat(3.2)),
            ..Default::default()
        });
    })
    .id()
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
        let (coll, agglo_trans, power, friction, restitution) = match agglo_query.get(agglo) {
            Ok(item) => item,
            _ => continue,
        };
        let trans = transform_relative_to(agglo_trans, klod_trans);
        cmds.entity(agglo)
            .remove_bundle::<AggloBundle>()
            .remove_bundle::<(Collider, Friction, Restitution)>()
            .insert_bundle((KlodElem { klod }, trans, *power));
        cmds.entity(klod).add_child(agglo);
        screen_print!("added {agglo:?} to klod {klod:?}");
        if let Ok(mut klod_component) = klod_query.get_mut(klod) {
            klod_component.weight += agglo_weight;

            cmds.entity(klod).add_children(|cmds| {
                spawn_klod_elem(
                    cmds,
                    klod,
                    agglo_weight,
                    coll.clone(),
                    trans,
                    friction.cloned().unwrap_or_default(),
                    restitution.cloned().unwrap_or_default(),
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
    default_klod_position: Res<KlodSpawnTransform>,
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
    let force = INPUT_IMPULSE;
    let force = |key, dir| if keys.pressed(key) { dir * force } else { Vec2::ZERO };
    let force = force(W, Vec2::Y) + force(S, -Vec2::Y) + force(A, Vec2::X) + force(D, -Vec2::X);
    let force = Vec2::from_angle(-cam_rot.horizontal_rotation()).rotate(force);
    let vel = velocity.linvel;
    let force = (vel.xz() + force).clamp_length_max(MAX_KLOD_SPEED) - vel.xz();
    impulse.impulse = Vec3::new(force.x, 0.0, force.y);

    if keys.just_pressed(KeyCode::Space) {
        *transform = default_klod_position.0;
        *velocity = Velocity::default();
    }
}

#[derive(Default)]
pub(crate) struct KlodSpawnTransform(pub(crate) Transform);

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<Klod>()
            .register_inspectable::<KlodElem>()
            .register_inspectable::<Agglomerable>();

        app.init_resource::<KlodSpawnTransform>()
            .add_event::<AgglomerateToKlod>()
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    .with_system(ball_input)
                    .with_system(shlurp_agglomerable)
                    .with_system(agglo_to_klod.after(shlurp_agglomerable)),
            );
    }
}
