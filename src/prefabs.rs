use std::ops::{Div, Mul};

use bevy::{
    ecs::query::{QueryItem, WorldQuery},
    ecs::system::EntityCommands,
    prelude::*,
    ui::FocusPolicy,
};
#[cfg(feature = "debug")]
use bevy_inspector_egui::Inspectable;
use bevy_rapier3d::prelude::*;
use serde::Deserialize;

use crate::{
    ball::Agglomerable,
    collision_groups as groups,
    game_audio::MusicTrigger,
    powers::{ElementalObstacle, Power},
};

pub(crate) trait Prefab {
    type Query: WorldQuery;

    fn from_query(item: QueryItem<Self::Query>) -> Self;

    fn spawn(self, cmds: &mut EntityCommands);
}

#[cfg_attr(feature = "editor", derive(serde::Serialize))]
#[derive(Debug, Deserialize, Copy, Clone)]
pub(crate) struct SerdeTransform {
    pub(crate) rotation: Quat,
    pub(crate) scale: Vec3,
    pub(crate) translation: Vec3,
}
impl Default for SerdeTransform {
    fn default() -> Self {
        SerdeTransform {
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            translation: Vec3::ZERO,
        }
    }
}
impl From<Transform> for SerdeTransform {
    fn from(item: Transform) -> Self {
        SerdeTransform {
            rotation: item.rotation,
            scale: item.scale,
            translation: item.translation,
        }
    }
}
impl From<SerdeTransform> for Transform {
    fn from(item: SerdeTransform) -> Self {
        Transform {
            rotation: item.rotation,
            scale: item.scale,
            translation: item.translation,
        }
    }
}

#[cfg_attr(feature = "editor", derive(serde::Serialize))]
#[derive(Deserialize, Debug, Clone)]
pub(crate) enum SerdeCollider {
    Ball {
        radius: f32,
    },
    Cuboid {
        half_extents: Vec3,
    },
    Capsule {
        a: Vec3,
        b: Vec3,
        radius: f32,
    },
    Cylinder {
        half_height: f32,
        radius: f32,
    },
    Cone {
        half_height: f32,
        radius: f32,
    },
    RoundCuboid {
        half_extents: Vec3,
        border_radius: f32,
    },
    RoundCylinder {
        half_height: f32,
        radius: f32,
        border_radius: f32,
    },
    RoundCone {
        half_height: f32,
        radius: f32,
        border_radius: f32,
    },
}
impl Div<Vec3> for SerdeCollider {
    type Output = SerdeCollider;

    fn div(self, rhs: Vec3) -> Self::Output {
        let rhs = 1.0 / rhs;
        self * rhs
    }
}
impl Mul<Vec3> for SerdeCollider {
    type Output = SerdeCollider;
    fn mul(self, rhs: Vec3) -> Self::Output {
        use SerdeCollider::*;
        let Vec3 { x, y, z } = rhs;
        let avg_mul = (x + y + z) / 3.0;
        match self {
            SerdeCollider::Ball { radius } => Ball { radius: radius * avg_mul },
            SerdeCollider::Cuboid { half_extents } => Cuboid { half_extents: half_extents * rhs },
            SerdeCollider::Capsule { a, b, radius } => {
                Capsule { a: a * rhs, b: b * rhs, radius: radius * avg_mul }
            }
            SerdeCollider::Cylinder { half_height, radius } => Cylinder {
                half_height: half_height * avg_mul,
                radius: radius * avg_mul,
            },
            SerdeCollider::Cone { half_height, radius } => Cone {
                half_height: half_height * avg_mul,
                radius: radius * avg_mul,
            },
            SerdeCollider::RoundCuboid { half_extents, border_radius } => RoundCuboid {
                half_extents: half_extents * rhs,
                border_radius: border_radius * avg_mul,
            },
            SerdeCollider::RoundCylinder { half_height, radius, border_radius } => RoundCylinder {
                half_height: half_height * avg_mul,
                radius: radius * avg_mul,
                border_radius: border_radius * avg_mul,
            },
            SerdeCollider::RoundCone { half_height, radius, border_radius } => RoundCone {
                half_height: half_height * avg_mul,
                radius: radius * avg_mul,
                border_radius: border_radius * avg_mul,
            },
        }
    }
}
impl From<SerdeCollider> for Mesh {
    fn from(collider: SerdeCollider) -> Self {
        match collider {
            SerdeCollider::Ball { radius } => shape::Icosphere { radius, subdivisions: 3 }.into(),
            SerdeCollider::Cuboid { half_extents: Vec3 { x, y, z } } => {
                shape::Box::new(x * 2.0, y * 2.0, z * 2.0).into()
            }
            SerdeCollider::Capsule { a, b, radius } => shape::Capsule {
                radius,
                depth: a.distance(b),
                latitudes: 8,
                rings: 0,
                longitudes: 8,
                uv_profile: default(),
            }
            .into(),
            SerdeCollider::Cylinder { half_height, radius } => {
                shape::Box::new(radius * 2.0, half_height * 2.0, radius * 2.0).into()
            }
            SerdeCollider::Cone { half_height, radius } => {
                shape::Box::new(radius * 2.0, half_height * 2.0, radius * 2.0).into()
            }

            SerdeCollider::RoundCuboid { half_extents: Vec3 { x, y, z }, border_radius } => {
                shape::Box::new(
                    (border_radius + x) * 2.0,
                    (border_radius + y) * 2.0,
                    (border_radius + z) * 2.0,
                )
                .into()
            }
            SerdeCollider::RoundCylinder { half_height, radius, border_radius } => shape::Box::new(
                (border_radius + radius) * 2.0,
                (border_radius + half_height) * 2.0,
                (border_radius + radius) * 2.0,
            )
            .into(),
            SerdeCollider::RoundCone { half_height, radius, border_radius } => shape::Box::new(
                (border_radius + radius) * 2.0,
                (border_radius + half_height) * 2.0,
                (border_radius + radius) * 2.0,
            )
            .into(),
        }
    }
}
impl<'a> From<&'a Collider> for SerdeCollider {
    fn from(collider: &'a Collider) -> Self {
        match collider.as_unscaled_typed_shape() {
            ColliderView::Ball(view) => SerdeCollider::Ball { radius: view.radius() },
            ColliderView::Cuboid(view) => {
                SerdeCollider::Cuboid { half_extents: view.half_extents() }
            }
            ColliderView::Capsule(view) => SerdeCollider::Capsule {
                a: Vec3::new(
                    view.raw.segment.a.x,
                    view.raw.segment.a.y,
                    view.raw.segment.a.z,
                ),
                b: Vec3::new(
                    view.raw.segment.b.x,
                    view.raw.segment.b.y,
                    view.raw.segment.b.z,
                ),
                radius: view.radius(),
            },
            ColliderView::Cylinder(view) => SerdeCollider::Cylinder {
                half_height: view.half_height(),
                radius: view.radius(),
            },
            ColliderView::Cone(view) => SerdeCollider::Cone {
                half_height: view.half_height(),
                radius: view.radius(),
            },
            ColliderView::RoundCuboid(view) => SerdeCollider::RoundCuboid {
                half_extents: view.inner_shape().half_extents(),
                border_radius: view.border_radius(),
            },
            ColliderView::RoundCylinder(view) => SerdeCollider::RoundCylinder {
                half_height: view.inner_shape().half_height(),
                radius: view.inner_shape().radius(),
                border_radius: view.border_radius(),
            },
            ColliderView::RoundCone(view) => SerdeCollider::RoundCone {
                half_height: view.inner_shape().half_height(),
                radius: view.inner_shape().radius(),
                border_radius: view.border_radius(),
            },
            _ => {
                let aabb = collider.raw.compute_local_aabb();
                SerdeCollider::Cuboid { half_extents: aabb.half_extents().into() }
            }
        }
    }
}
impl From<SerdeCollider> for Collider {
    fn from(data: SerdeCollider) -> Self {
        match data {
            SerdeCollider::Ball { radius } => Collider::ball(radius),
            SerdeCollider::Cuboid { half_extents: Vec3 { x, y, z } } => Collider::cuboid(x, y, z),
            SerdeCollider::Capsule { a, b, radius } => Collider::capsule(a, b, radius),
            SerdeCollider::Cylinder { half_height, radius } => {
                Collider::cylinder(half_height, radius)
            }
            SerdeCollider::Cone { half_height, radius } => Collider::cone(half_height, radius),
            SerdeCollider::RoundCuboid { half_extents: Vec3 { x, y, z }, border_radius } => {
                Collider::round_cuboid(x, y, z, border_radius)
            }
            SerdeCollider::RoundCylinder { half_height, radius, border_radius } => {
                Collider::round_cylinder(half_height, radius, border_radius)
            }
            SerdeCollider::RoundCone { half_height, radius, border_radius } => {
                Collider::round_cone(half_height, radius, border_radius)
            }
        }
    }
}

/// Static physic objects
#[cfg_attr(feature = "editor", derive(serde::Serialize))]
#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Debug, Deserialize, Component, Clone)]
pub(crate) struct Scenery {
    pub(crate) weakness: Vec<Power>,
}
impl Prefab for Scenery {
    type Query = (&'static Scenery, Option<&'static ElementalObstacle>);

    fn from_query((_, powers): QueryItem<Self::Query>) -> Self {
        let non_empty = powers.filter(|p| !p.required_powers.is_empty());
        Scenery {
            weakness: non_empty.map_or(Vec::new(), |p| p.required_powers.clone()),
        }
    }
    fn spawn(self, cmds: &mut EntityCommands) {
        if !self.weakness.is_empty() {
            cmds.insert(ElementalObstacle { required_powers: self.weakness.clone() });
        }
        cmds.insert_bundle((RigidBody::Fixed, self));
    }
}

#[cfg_attr(feature = "editor", derive(serde::Serialize))]
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AggloData {
    mass: f32,
    power: Power,
}
impl AggloData {
    pub(crate) fn new(mass: f32, power: Power) -> Self {
        Self { mass, power }
    }
}
#[derive(Bundle)]
pub(crate) struct AggloBundle {
    agglo: Agglomerable,
    active_events: ActiveEvents,
    mass: ColliderMassProperties,
    rigid_body: RigidBody,
    contact_threshold: ContactForceEventThreshold,
    collision_group: CollisionGroups,
    power: Power,
}
impl AggloBundle {
    pub(crate) fn new(mass: f32, power: Power) -> Self {
        AggloBundle {
            power,
            agglo: Agglomerable { weight: mass },
            active_events: ActiveEvents::CONTACT_FORCE_EVENTS,
            contact_threshold: ContactForceEventThreshold(mass * 10.0),
            mass: ColliderMassProperties::Mass(mass),
            rigid_body: RigidBody::Dynamic,
            collision_group: groups::AGGLO,
        }
    }
}

impl Prefab for AggloData {
    type Query = (&'static Agglomerable, &'static Power);

    fn from_query((agglo, power): QueryItem<Self::Query>) -> Self {
        AggloData { mass: agglo.weight, power: *power }
    }

    fn spawn(self, cmds: &mut EntityCommands) {
        let Self { mass, power } = self;
        cmds.insert_bundle(AggloBundle::new(mass, power));
    }
}

#[cfg_attr(feature = "editor", derive(serde::Serialize))]
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct MusicTriggerData {
    name: String,
    trigger: MusicTrigger,
    pub(crate) collider: SerdeCollider,
    transform: SerdeTransform,
}
impl MusicTriggerData {
    pub(crate) fn new(name: String, trigger: MusicTrigger, collider: &Collider) -> Self {
        Self {
            name,
            trigger,
            collider: collider.into(),
            transform: default(),
        }
    }
}
impl Prefab for MusicTriggerData {
    type Query = (
        &'static MusicTrigger,
        &'static Collider,
        &'static Transform,
        &'static Name,
    );

    fn from_query((trigger, collider, transform, name): QueryItem<Self::Query>) -> Self {
        MusicTriggerData {
            name: name.to_string(),
            trigger: *trigger,
            collider: collider.into(),
            transform: (*transform).into(),
        }
    }
    fn spawn(self, cmds: &mut EntityCommands) {
        cmds.insert_bundle((
            Name::new(self.name),
            self.trigger,
            Sensor,
            groups::MUSIC,
            Transform::from(self.transform),
            GlobalTransform::default(),
            Collider::from(self.collider),
        ));
        #[cfg(feature = "editor")]
        cmds.insert_bundle((
            Visibility::default(),
            ComputedVisibility::default(),
            bevy_mod_picking::PickableMesh::default(),
            Interaction::default(),
            FocusPolicy::default(),
            bevy_mod_picking::Selection::default(),
            bevy_transform_gizmo::GizmoTransformable,
        ));
    }
}
