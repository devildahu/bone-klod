//! A camera orbiting around the origin of its transform
//!
//! Making it a sibbling of an object (such as the player) will act very
//! similarly to a 3D plateformer
// Derived from https://github.com/iMplode-nZ/bevy-orbit-controls
// Licensed under ISC
use std::f32::consts::TAU;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::{Plugin as BevyPlugin, *};
use bevy::transform::TransformSystem;
use bevy_debug_text_overlay::screen_print;
#[cfg(feature = "debug")]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::*;

const CAM_COLLISION_GROUP: InteractionGroups = InteractionGroups::new(0b0100, !0b0100);
const CAM_SPEED: f32 = 0.01;
const CAM_DIST: f32 = 20.0;
const CAM_Y_MAX: f32 = TAU / 4.0;
const CAM_Y_MIN: f32 = 0.3;

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component)]
pub(crate) struct OrbitCamera {
    distance: f32,
    /// In radians, the horizontal angle of the camera
    x_rot: f32,
    /// In radians, the vertical angle of the camera
    y_rot: f32,
    follows: Entity,
}

impl OrbitCamera {
    pub(crate) fn horizontal_rotation(&self) -> f32 {
        self.x_rot % TAU
    }
    pub(crate) fn follows(entity: Entity) -> Self {
        OrbitCamera {
            x_rot: 0.0,
            y_rot: std::f32::consts::FRAC_PI_2,
            distance: CAM_DIST,
            follows: entity,
        }
    }
}

fn update_camera_transform(
    mut query: Query<(&OrbitCamera, &mut Transform)>,
    phys: Res<RapierContext>,
    followed: Query<&Transform, Without<OrbitCamera>>,
) {
    let (camera, mut transform) = query.get_single_mut().unwrap();
    let followed = followed.get(camera.follows).unwrap();
    let followed_pos = followed.translation;
    // This is actually the crux of the orbit camera, this enables the
    // camera to rotate on a sphere around the Origin of the Transform
    let rot = Quat::from_axis_angle(Vec3::Y, camera.x_rot)
        * Quat::from_axis_angle(-Vec3::X, camera.y_rot);
    // Cast a cone shape, the base of which is oriented toward the origin
    let cam_offset = rot * Vec3::Y * camera.distance;
    let cam_pos = followed_pos + cam_offset;
    let shape = Collider::cone(0.2, 0.2);
    let looking_at_followed = Transform::from_translation(cam_pos)
        .looking_at(followed_pos, Vec3::Y)
        .rotation;
    transform.rotation = looking_at_followed;
    let cast_rot = looking_at_followed * Quat::from_rotation_x(TAU / 4.0);
    let collision = phys.cast_shape(
        followed_pos,
        cast_rot,
        cam_offset,
        &shape,
        1.0,
        QueryFilter::default().groups(CAM_COLLISION_GROUP),
    );
    transform.translation = if let Some((_, toi)) = collision {
        followed_pos + toi.toi * cam_offset
    } else {
        cam_pos
    };
}

fn camera_movement(
    // time: Res<Time>,
    mut events: EventReader<MouseMotion>,
    mut query: Query<&mut OrbitCamera, With<Camera>>,
) {
    let mut camera = match query.get_single_mut() {
        Ok(cam) => cam,
        Err(msg) => {
            screen_print!("error: {msg:?}");
            return;
        }
    };
    let delta = events.iter().fold(Vec2::ZERO, |acc, m| acc + m.delta);
    if delta != Vec2::ZERO {
        let xy = delta * CAM_SPEED;
        camera.x_rot -= xy.x;
        // The max(MIN) is counterintuitive, but correct
        camera.y_rot = (camera.y_rot - xy.y).max(CAM_Y_MIN).min(CAM_Y_MAX);
    }
}

pub(crate) struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<OrbitCamera>();

        app.add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(camera_movement)
                .with_system(update_camera_transform)
                .before(TransformSystem::TransformPropagate),
        );
    }
}
