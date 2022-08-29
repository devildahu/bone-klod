use bevy::math::EulerRot::XYZ;
use bevy::prelude::{Plugin as BevyPlugin, *};
use bevy_editor_pls::{Editor, EditorState};
use bevy_editor_pls_default_windows::cameras::camera_3d_panorbit::PanOrbitCamera;
use bevy_editor_pls_default_windows::{cameras::ActiveEditorCamera, hierarchy::HierarchyWindow};
use bevy_inspector_egui::Inspectable;
use bevy_mod_picking::{PickingCamera, Primitive3d};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
enum Axis {
    #[default]
    X,
    Y,
    Z,
}
impl Axis {
    fn as_vec3(&self) -> Vec3 {
        use self::Axis::*;
        match self {
            X => Vec3::X,
            Y => Vec3::Y,
            Z => Vec3::Z,
        }
    }
    fn component(&self, vec: Vec3) -> Vec3 {
        use self::Axis::*;
        match self {
            X => Vec3::X * vec.x,
            Y => Vec3::Y * vec.y,
            Z => Vec3::Z * vec.z,
        }
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
enum Component {
    #[default]
    None,
    Rotation,
    Translation,
    PlaneTranslation,
    UniformScale,
    Scale,
}
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
struct EditMod {
    axis: Axis,
    component: Component,
    snap_to_grid: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum EditModEvent {
    MoveToCamera,
    Cancel,
    Apply,
    Start,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct EditModChange;

#[derive(Component, Inspectable)]
struct Editing {
    original: Transform,
}

fn handle_trans_mod(
    input: Res<Input<KeyCode>>,
    mut edit: ResMut<EditMod>,
    mouse: Res<Input<MouseButton>>,
    mut events: EventWriter<EditModEvent>,
    mut changes: EventWriter<EditModChange>,
) {
    use self::Component::UniformScale;
    use KeyCode::{Escape, LControl, LShift, RShift, G, R, S, T, X, Y, Z};

    edit.snap_to_grid = input.pressed(LControl);

    macro_rules! set_on_key {
        (@arm $key:expr => $field:ident = $value:expr ) => {{
            if input.just_pressed($key) && edit.$field != $value {
                edit.$field = $value;
                changes.send(EditModChange);
            }
        }};
        ($($key:expr => $field:ident = $value:expr ,)*) => {
            $( set_on_key!(@arm $key => $field = $value ); )*
        }
    }
    let was_active = edit.component != Component::None;

    if input.just_pressed(T) && was_active {
        events.send(EditModEvent::MoveToCamera);
    }
    if input.just_pressed(Escape) && was_active {
        edit.component = Component::None;
        events.send(EditModEvent::Cancel);
    }
    if mouse.just_pressed(MouseButton::Right) && was_active {
        events.send(EditModEvent::Apply);
        edit.component = Component::None;
    }
    if input.any_just_pressed([G, R, S]) && !was_active {
        events.send(EditModEvent::Start);
    }
    if input.just_pressed(G) {
        let is = |c| edit.component == c;
        if input.any_pressed([LShift, RShift]) && !is(Component::PlaneTranslation) {
            edit.component = Component::PlaneTranslation;
            changes.send(EditModChange);
        } else if !is(Component::Translation) {
            edit.component = Component::Translation;
            changes.send(EditModChange);
        }
    }
    set_on_key! {
        R => component = Component::Rotation,
        S => component = UniformScale,
    };
    if input.just_pressed(X) && (edit.axis != Axis::X || edit.component == UniformScale) {
        edit.axis = Axis::X;
        if edit.component == UniformScale {
            edit.component = Component::Scale;
        }
        changes.send(EditModChange);
    }
    if input.just_pressed(Y) && (edit.axis != Axis::Y || edit.component == UniformScale) {
        edit.axis = Axis::Y;
        if edit.component == UniformScale {
            edit.component = Component::Scale;
        }
        changes.send(EditModChange);
    }
    if input.just_pressed(Z) && (edit.axis != Axis::Z || edit.component == UniformScale) {
        edit.axis = Axis::Z;
        if edit.component == UniformScale {
            edit.component = Component::Scale;
        }
        changes.send(EditModChange);
    }
}
fn manage_editing_component(
    editor: Res<Editor>,
    editor_state: Res<EditorState>,
    cam: Query<&PanOrbitCamera, With<ActiveEditorCamera>>,
    mut events: EventReader<EditModEvent>,
    mut cmds: Commands,
    mut editing: Query<(Entity, &mut Transform, &Editing)>,
    mut windows: ResMut<Windows>,
    transforms: Query<&Transform, Without<Editing>>,
) {
    if !editor_state.active {
        return;
    }
    for event in events.iter() {
        let window_msg = "There is at least one game window open";
        let mut leave_edit_mod = || {
            let window = windows.get_primary_mut().expect(window_msg);
            window.set_cursor_lock_mode(false);
        };
        match event {
            EditModEvent::Cancel => {
                leave_edit_mod();
                for (entity, mut transform, editing) in &mut editing {
                    *transform = editing.original;
                    cmds.entity(entity).remove::<Editing>();
                }
            }
            EditModEvent::Apply => {
                leave_edit_mod();
                for (entity, ..) in &editing {
                    cmds.entity(entity).remove::<Editing>();
                }
            }
            EditModEvent::Start => {
                let window = windows.get_primary_mut().expect(window_msg);
                window.set_cursor_lock_mode(true);

                let to_edit = editor.window_state::<HierarchyWindow>().unwrap();
                for entity in to_edit.selected.iter() {
                    let original = match transforms.get(entity) {
                        Ok(v) => *v,
                        Err(_) => continue,
                    };
                    cmds.entity(entity).insert(Editing { original });
                }
            }
            EditModEvent::MoveToCamera => {
                for (_, mut transform, _) in &mut editing {
                    let camera = match cam.get_single() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    transform.translation = camera.focus;
                }
            }
        }
    }
}
fn transform_editing(
    edit_mod: Res<EditMod>,
    camera: Query<&PickingCamera>,
    mut editing: Query<(&mut Transform, &Editing)>,
    mut changes: EventReader<EditModChange>,
    mut previous_pos: Local<Vec3>,
) -> Option<()> {
    if edit_mod.component == Component::None {
        return Some(());
    }
    let first_transform = editing.iter().next()?.1.original;
    let camera = camera.get_single().ok()?;
    let camera_ray = camera.ray()?;
    let axis = edit_mod.axis.as_vec3();
    let ray = camera_ray.direction();
    let normal = axis.cross(ray).cross(axis).normalize();
    let intersection_plane = Primitive3d::Plane { normal, point: first_transform.translation };
    let intersection = camera.intersect_primitive(intersection_plane)?.position();
    if changes.iter().next().is_some() {
        *previous_pos = intersection;
        return Some(());
    }
    let delta = intersection - *previous_pos;
    *previous_pos = intersection;
    match edit_mod.component {
        Component::None => unreachable!("Returned early when this was met"),
        Component::Rotation => {
            for (mut transform, _) in &mut editing {
                let Vec3 { x, y, z } = edit_mod.axis.component(delta) / 4.0;
                let rot = Quat::from_euler(XYZ, x, y, z);
                transform.rotation *= rot;
            }
        }
        Component::PlaneTranslation => {
            for (mut transform, _) in &mut editing {
                transform.translation += delta;
            }
        }
        Component::Translation => {
            for (mut transform, _) in &mut editing {
                transform.translation += edit_mod.axis.component(delta);
            }
        }
        Component::UniformScale => {
            let Vec3 { x, y, z } = delta / 4.0;
            let magnitude = x + y + z;
            for (mut transform, _) in &mut editing {
                transform.scale -= Vec3::splat(magnitude);
            }
        }
        Component::Scale => {
            let Vec3 { x, y, z } = delta / 4.0;
            let magnitude = x + y + z;
            for (mut transform, _) in &mut editing {
                transform.scale -= edit_mod.axis.as_vec3() * magnitude;
            }
        }
    }
    Some(())
}

macro_rules! err_sys {
    ($system:expr) => {
        $system.chain(|_| {})
    };
}
pub(super) struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditMod>()
            .add_event::<EditModEvent>()
            .add_event::<EditModChange>()
            .add_system(err_sys!(transform_editing).after(handle_trans_mod))
            .add_system(handle_trans_mod)
            .add_system(manage_editing_component.after(handle_trans_mod));
    }
}
