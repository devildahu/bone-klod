//! Animations.

use bevy::prelude::{Plugin as BevyPlugin, *};
#[cfg(feature = "debug")]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) enum Animate {
    /// Moves the thing on the XY plane toward `target` at `speed` unit per second.
    MoveToward {
        target: Vec3,
        speed: f32,
    },
    /// Shake the camera along `direction` until `until` with a forward/backward period of `period`.
    Shake {
        until: f64,
        direction: Vec3,
        period: f64,
    },
    ResizeTo {
        target: Vec3,
        speed: f32,
    },
    #[default]
    None,
}

/// Handles the [`Animate`] component.
fn animate_system(mut animated: Query<(&Animate, &mut Transform)>, time: Res<Time>) {
    let delta = time.delta_seconds();
    let current_time = time.seconds_since_startup();
    for (animate, mut transform) in &mut animated {
        let current = transform.translation;
        match animate {
            Animate::None => {}
            &Animate::MoveToward { target, speed } => {
                let diff = target - current;
                let diff_len = diff.length_squared();
                if diff_len > 0.05 {
                    // move toward target without overshooting it.
                    let distance_traversed = diff_len.sqrt().min(delta * speed);
                    let traversed = distance_traversed * diff.normalize_or_zero();
                    let new_position = current + traversed;
                    transform.translation = new_position;
                }
            }
            &Animate::ResizeTo { target, speed } => {
                if !target.abs_diff_eq(transform.scale, 0.01) {
                    transform.scale = transform.scale.lerp(target, speed * delta);
                }
            }
            &Animate::Shake { until, direction, period } if until > current_time => {
                let sign = current_time % period < period / 2.0;
                let sign = if sign { 1.0 } else { -1.0 };
                let new_position = current + direction * sign;
                transform.translation = new_position;
            }
            Animate::Shake { .. } => {}
        }
    }
}

pub(crate) struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<Animate>();

        app.add_system(animate_system);
    }
}
