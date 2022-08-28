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
#[cfg(feature = "debug")]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::*;

use crate::collision_groups as groups;

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
    #[cfg_attr(feature = "debug", inspectable(ignore))]
    follows: Entity,
    /// Prevent camera from moving with mouse.
    pub locked: bool,
}

impl OrbitCamera {
    pub(crate) fn horizontal_rotation(&self) -> f32 {
        self.x_rot % TAU
    }
    pub(crate) fn follows(entity: Entity) -> Self {
        OrbitCamera {
            x_rot: 1.48,
            y_rot: 1.101,
            locked: false,
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
    let (camera, mut transform) = match query.get_single_mut() {
        Ok(item) => item,
        Err(_) => return,
    };
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
        QueryFilter::default().groups(groups::CAM.into()),
    );
    transform.translation = if let Some((_, toi)) = collision {
        followed_pos + toi.toi * cam_offset
    } else {
        cam_pos
    };
}

fn camera_movement(
    mut events: EventReader<MouseMotion>,
    gp_axis: Res<Axis<GamepadAxis>>,
    mut query: Query<&mut OrbitCamera, With<Camera>>,
) {
    let mut camera = match query.get_single_mut() {
        Ok(cam) => cam,
        Err(_) => return,
    };
    if camera.locked {
        return;
    }
    let gp_axis_kind = |axis_type| GamepadAxis { gamepad: Gamepad { id: 0 }, axis_type };
    let axis_x = gp_axis_kind(GamepadAxisType::RightStickX);
    let axis_y = gp_axis_kind(GamepadAxisType::RightStickY);
    let gp_y_delta = gp_axis.get(axis_y).map_or(default(), |y| -Vec2::Y * y);
    let gp_x_delta = gp_axis.get(axis_x).map_or(default(), |x| Vec2::X * x);
    let gp_delta = gp_x_delta + gp_y_delta;
    let delta = if gp_delta.length_squared() < 0.01 {
        events.iter().fold(Vec2::ZERO, |acc, m| acc + m.delta)
    } else {
        gp_delta * 2.1
    };
    if delta != Vec2::ZERO {
        let xy = delta * CAM_SPEED;
        camera.x_rot -= xy.x;
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
                .with_system(update_camera_transform.after(camera_movement))
                .before(TransformSystem::TransformPropagate),
        );
    }
}
