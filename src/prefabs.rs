use bevy::{
    ecs::query::{QueryItem, WorldQuery},
    ecs::system::EntityCommands,
    prelude::*,
};
use bevy_rapier3d::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::ball::{Agglomerable, Scenery};

pub(crate) trait Prefab: Serialize + DeserializeOwned {
    type Query: WorldQuery;

    fn from_query(item: &QueryItem<Self::Query>) -> Self;

    fn spawn(self, cmds: &mut EntityCommands, meshes: &mut Assets<Mesh>);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
impl<'a> From<ColliderView<'a>> for SerdeCollider {
    fn from(view: ColliderView<'a>) -> Self {
        match view {
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
            _ => panic!("Cannot handle view type!"),
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

#[derive(Serialize, Debug, Deserialize)]
pub(crate) struct SceneryData {
    collider: SerdeCollider,
    friction: f32,
    restitution: f32,
}
impl SceneryData {
    pub(crate) fn new(collider: SerdeCollider, friction: f32, restitution: f32) -> Self {
        Self { collider, friction, restitution }
    }
}
impl Prefab for SceneryData {
    type Query = (
        &'static Scenery,
        &'static Collider,
        &'static Friction,
        &'static Restitution,
    );

    fn from_query((_, collider, friction, restitution): &QueryItem<Self::Query>) -> Self {
        SceneryData {
            collider: collider.as_typed_shape().into(),
            friction: friction.coefficient,
            restitution: restitution.coefficient,
        }
    }
    fn spawn(self, cmds: &mut EntityCommands, meshes: &mut Assets<Mesh>) {
        cmds.insert_bundle((
            meshes.add(self.collider.clone().into()),
            RigidBody::Fixed,
            Scenery,
            Collider::from(self.collider),
            Friction {
                coefficient: self.friction,
                combine_rule: CoefficientCombineRule::Max,
            },
            Restitution {
                coefficient: self.restitution,
                combine_rule: CoefficientCombineRule::Max,
            },
        ));
    }
}

#[derive(Serialize, Debug, Deserialize)]
pub(crate) struct AggloData {
    mass: f32,
    collider: SerdeCollider,
    friction: f32,
    restitution: f32,
}
impl AggloData {
    pub(crate) fn new(mass: f32, collider: SerdeCollider, friction: f32, restitution: f32) -> Self {
        Self { mass, collider, friction, restitution }
    }
}
#[derive(Bundle)]
pub(crate) struct AggloBundle {
    agglo: Agglomerable,
    active_events: ActiveEvents,
    collider: Collider,
    mass: ColliderMassProperties,
    rigid_body: RigidBody,
    contact_threshold: ContactForceEventThreshold,
    collision_group: CollisionGroups,
    friction: Friction,
    restitution: Restitution,
}
impl AggloBundle {
    pub(crate) fn new(mass: f32, collider: Collider, friction: f32, restitution: f32) -> Self {
        AggloBundle {
            agglo: Agglomerable { weight: mass },
            active_events: ActiveEvents::CONTACT_FORCE_EVENTS,
            collider: collider.into(),
            mass: ColliderMassProperties::Mass(mass),
            rigid_body: RigidBody::Dynamic,
            contact_threshold: ContactForceEventThreshold(mass * 1000.0),
            collision_group: AGGLO_COLLISION_GROUP,
            friction: Friction {
                coefficient: friction,
                combine_rule: CoefficientCombineRule::Max,
            },
            restitution: Restitution {
                coefficient: restitution,
                combine_rule: CoefficientCombineRule::Max,
            },
        }
    }
}

#[derive(Component)]
pub(crate) struct SceneryEmpty;

#[derive(Serialize, Debug, Deserialize)]
pub(crate) struct Empty(pub(crate) SerdeCollider);
impl Prefab for Empty {
    type Query = (&'static Collider, &'static SceneryEmpty);

    fn from_query(item: &QueryItem<Self::Query>) -> Self {
        Empty(item.0.as_typed_shape().into())
    }
    fn spawn(self, cmds: &mut EntityCommands, meshes: &mut Assets<Mesh>) {
        cmds.insert_bundle((
            meshes.add(self.0.clone().into()),
            RigidBody::Fixed,
            SceneryEmpty,
            Collider::from(self.0),
        ));
    }
}

const AGGLO_COLLISION_GROUP: CollisionGroups = CollisionGroups::new(0b1000, !0);

impl Prefab for AggloData {
    type Query = (
        &'static Collider,
        &'static Agglomerable,
        &'static Friction,
        &'static Restitution,
    );

    fn from_query((collider, agglo, friction, restitution): &QueryItem<Self::Query>) -> Self {
        AggloData {
            mass: agglo.weight,
            collider: collider.as_typed_shape().into(),
            friction: friction.coefficient,
            restitution: restitution.coefficient,
        }
    }

    fn spawn(self, cmds: &mut EntityCommands, meshes: &mut Assets<Mesh>) {
        let Self { mass, collider, friction, restitution } = self;
        cmds.insert_bundle(AggloBundle::new(
            mass,
            collider.clone().into(),
            friction,
            restitution,
        ))
        .insert(meshes.add(collider.into()));
    }
}
