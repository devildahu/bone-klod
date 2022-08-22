use bevy::prelude::{Plugin as BevyPlugin, *};

use bevy_editor_pls::{prelude::*, EditorEvent};
use bevy_editor_pls_default_windows::cameras::ActiveEditorCamera;
use bevy_mod_picking::{DefaultPickingPlugins, PickingCameraBundle};
use bevy_transform_gizmo::{GizmoPickSource, TransformGizmoPlugin};

use crate::cam::OrbitCamera;

fn _count_active_cameras(cams: Query<(&Camera, &Name)>) {
    let cams: Vec<_> = cams
        .iter()
        .filter_map(|t| t.0.is_active.then(|| t.1))
        .collect();
    println!("{cams:?} active cameras");
}

fn toggle_picking_camera(
    mut cmds: Commands,
    mut events: EventReader<EditorEvent>,
    klod_cam: Query<Entity, With<OrbitCamera>>,
    editor_cam: Query<Entity, With<ActiveEditorCamera>>,
) -> Option<()> {
    for event in events.iter() {
        match event {
            EditorEvent::Toggle { now_active } => {
                println!("toggle {now_active}");
                let (to_enable, to_disable) = if *now_active {
                    (editor_cam.get_single().ok()?, klod_cam.get_single().ok()?)
                } else {
                    (klod_cam.get_single().ok()?, editor_cam.get_single().ok()?)
                };
                println!("changing picking camera");
                cmds.entity(to_enable)
                    .insert_bundle(PickingCameraBundle::default())
                    .insert(GizmoPickSource::default());
                cmds.entity(to_disable)
                    .remove_bundle::<PickingCameraBundle>()
                    .remove::<GizmoPickSource>();
            }
            _ => {}
        }
    }
    Some(())
}
macro_rules! err_sys {
    ($system:expr) => {
        $system.chain(|_| {})
    };
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .add_plugin(TransformGizmoPlugin::default())
            .add_plugin(EditorPlugin)
            // .add_system(count_active_cameras)
            .add_system(err_sys!(toggle_picking_camera));
    }
}
