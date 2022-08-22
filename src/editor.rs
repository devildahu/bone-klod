use bevy::{
    ecs::system::SystemState,
    prelude::{Plugin as BevyPlugin, *},
};

use bevy_editor_pls::{
    editor_window::{EditorWindow, EditorWindowContext},
    prelude::*,
    EditorEvent,
};
use bevy_editor_pls_default_windows::{
    cameras::ActiveEditorCamera, hierarchy::picking::IgnoreEditorRayCast,
};
use bevy_inspector_egui::egui;
use bevy_mod_picking::{DefaultPickingPlugins, PickingCameraBundle};
use bevy_transform_gizmo::{GizmoPickSource, PickableGizmo, TransformGizmoPlugin};

use crate::{
    prefabs::{AggloData, SceneryData, SerdeCollider},
    scene::{KlodScene, ObjectType, PhysicsObject},
};

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
    editor_cam: Query<Entity, With<ActiveEditorCamera>>,
) -> Option<()> {
    for event in events.iter() {
        match event {
            EditorEvent::Toggle { now_active } => {
                println!("toggle {now_active}");
                let cam = editor_cam.get_single().ok()?;
                if *now_active {
                    cmds.entity(cam)
                        .insert_bundle(PickingCameraBundle::default())
                        .insert(GizmoPickSource::default());
                } else {
                    cmds.entity(cam)
                        .remove_bundle::<PickingCameraBundle>()
                        .remove::<GizmoPickSource>();
                };
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

#[derive(Default)]
pub struct SceneWindowState {
    filename: String,
    scene: String,
    spawn_name: String,
    spawn_mass: f32,
    spawn_restitution: f32,
    spawn_friction: f32,
    scene_save_result: Option<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
}

const DEFAULT_FILENAME: &str = "scene.scn.ron";
const DEFAULT_SCENE: &str = "untilted.glb";
pub struct SceneWindow;
impl EditorWindow for SceneWindow {
    type State = SceneWindowState;
    const NAME: &'static str = "Level Management";

    fn ui(world: &mut World, mut cx: EditorWindowContext, ui: &mut egui::Ui) {
        let state = cx.state_mut::<SceneWindow>().unwrap();

        let res = egui::TextEdit::singleline(&mut state.filename)
            .hint_text(DEFAULT_FILENAME)
            .desired_width(120.0)
            .show(ui);

        if res.response.changed() {
            state.scene_save_result = None;
        }

        ui.horizontal(|ui| {
            let filename = if state.filename.is_empty() {
                DEFAULT_FILENAME
            } else {
                &state.filename
            };
            if ui.button("Save").clicked() {
                state.scene_save_result = Some(KlodScene::save(world, filename));
            }
            if ui.button("Load").clicked() {
                state.scene_save_result = Some(KlodScene::load(world, filename));
            }
        });

        if let Some(status) = &state.scene_save_result {
            match status {
                Ok(()) => {
                    ui.label(egui::RichText::new("Success!").color(egui::Color32::GREEN));
                }
                Err(error) => {
                    ui.label(egui::RichText::new(error.to_string()).color(egui::Color32::RED));
                }
            }
        }

        ui.separator();

        ui.horizontal(|ui| {
            egui::TextEdit::singleline(&mut state.spawn_name)
                .hint_text("Physical Object")
                .desired_width(120.0)
                .show(ui);
        });
        ui.horizontal(|ui| {
            ui.label("Mass");
            let res =
                ui.add(egui::DragValue::new(&mut state.spawn_mass).clamp_range(0.0..=100_000.0));
            res.on_hover_text("Set to 0 for a static landscape collider");
        });
        ui.horizontal(|ui| {
            ui.label("Friction");
            ui.add(
                egui::DragValue::new(&mut state.spawn_friction)
                    .speed(0.05)
                    .clamp_range(0.0..=2.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Restitution");
            ui.add(
                egui::DragValue::new(&mut state.spawn_restitution)
                    .speed(0.05)
                    .clamp_range(0.0..=2.0),
            );
        });
        egui::TextEdit::singleline(&mut state.scene)
            .hint_text(DEFAULT_SCENE)
            .desired_width(120.0)
            .show(ui);
        if ui.button("Add new prop").clicked() {
            let mut system_state =
                SystemState::<(Commands, Res<AssetServer>, ResMut<Assets<Mesh>>)>::new(world);
            let (mut cmds, assets, mut meshes) = system_state.get_mut(world);
            let mass = state.spawn_mass;
            let data = if mass == 0.0 {
                ObjectType::Scenery(SceneryData::new(
                    SerdeCollider::Cuboid { half_extents: Vec3::new(10.0, 10.0, 10.0) },
                    state.spawn_friction,
                    state.spawn_restitution,
                ))
            } else {
                ObjectType::Agglomerable(AggloData::new(
                    mass,
                    SerdeCollider::Cuboid { half_extents: Vec3::new(10.0, 10.0, 10.0) },
                    state.spawn_friction,
                    state.spawn_restitution,
                ))
            };
            let data = PhysicsObject::new(
                state.spawn_name.clone(),
                state.scene.clone(),
                default(),
                data,
            );
            data.spawn(&mut cmds, &assets, &mut *meshes);
            system_state.apply(world);
        }
        if ui.button("Clone Selected").clicked() {
            todo!()
        }
    }
}

fn ignore_transform_gizmo(mut cmds: Commands, gizmo_elems: Query<Entity, Added<PickableGizmo>>) {
    for added in &gizmo_elems {
        cmds.entity(added).insert(IgnoreEditorRayCast);
    }
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .add_plugin(TransformGizmoPlugin::default())
            .add_plugin(EditorPlugin)
            .add_editor_window::<SceneWindow>()
            .add_system_to_stage(CoreStage::PostUpdate, ignore_transform_gizmo)
            .add_system(err_sys!(toggle_picking_camera));
    }
}
