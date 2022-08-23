use std::path::PathBuf;

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
    cameras::ActiveEditorCamera,
    hierarchy::{picking::IgnoreEditorRayCast, HierarchyState, HierarchyWindow},
};
use bevy_inspector_egui::egui;
use bevy_mod_picking::{DefaultPickingPlugins, PickingCameraBundle};
use bevy_rapier3d::prelude::RapierConfiguration;
use bevy_transform_gizmo::{GizmoPickSource, PickableGizmo, TransformGizmoPlugin};

use crate::{
    cam::OrbitCamera,
    prefabs::{AggloData, Empty, SceneryData, SerdeCollider},
    scene::{KlodScene, ObjectType, PhysicsObject},
};

fn _count_active_cameras(cams: Query<(&Camera, &Name)>) {
    let cams: Vec<_> = cams
        .iter()
        .filter_map(|t| t.0.is_active.then(|| t.1))
        .collect();
    println!("{cams:?} active cameras");
}

fn toggle_editor_active(
    mut cmds: Commands,
    mut events: EventReader<EditorEvent>,
    mut rapier_config: ResMut<RapierConfiguration>,
    mut orbit_cam: Query<&mut OrbitCamera>,
    editor_cam: Query<Entity, With<ActiveEditorCamera>>,
) -> Option<()> {
    for event in events.iter() {
        match event {
            EditorEvent::Toggle { now_active } => {
                println!("toggle {now_active}");
                let mut orbit_cam = orbit_cam.get_single_mut().ok()?;
                orbit_cam.locked = *now_active;
                let cam = editor_cam.get_single().ok()?;
                if *now_active {
                    rapier_config.physics_pipeline_active = false;
                    cmds.entity(cam)
                        .insert_bundle(PickingCameraBundle::default())
                        .insert(GizmoPickSource::default());
                } else {
                    rapier_config.physics_pipeline_active = true;
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

const DEFAULT_FILENAME: &str = "default.klodlvl";

pub struct SceneWindow;
impl EditorWindow for SceneWindow {
    type State = SceneWindowState;
    const NAME: &'static str = "Level Management";

    fn ui(world: &mut World, mut cx: EditorWindowContext, ui: &mut egui::Ui) {
        let (state, hierarchy_state) = match cx.state_mut_pair::<SceneWindow, HierarchyWindow>() {
            (Some(state), Some(hierarchy)) => (state, hierarchy),
            _ => return,
        };

        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                ui.set_width(140.0);
                let res = egui::TextEdit::singleline(&mut state.filename)
                    .hint_text(DEFAULT_FILENAME)
                    .desired_width(140.0)
                    .show(ui);

                egui::Grid::new("Level Loader").show(ui, |ui| {
                    if res.response.changed() {
                        state.scene_save_result = None;
                    }

                    let filename = file_name(&state.filename);
                    if ui.button("Save").clicked() {
                        state.scene_save_result = Some(KlodScene::save(world, &filename));
                    }
                    if ui.button("Load").clicked() {
                        state.scene_save_result = Some(KlodScene::load(world, &filename));
                    }
                    ui.end_row();

                    match &state.scene_save_result {
                        Some(Ok(())) => {
                            ui.label(egui::RichText::new("Success!").color(egui::Color32::GREEN));
                        }
                        Some(Err(error)) => {
                            ui.label(
                                egui::RichText::new(error.to_string()).color(egui::Color32::RED),
                            );
                        }
                        None => {}
                    }
                });
            });
            egui::Grid::new("Props management physics data").show(ui, |ui| {
                ui.set_width(140.0);
                ui.label("Name");
                egui::TextEdit::singleline(&mut state.spawn_name)
                    .hint_text("Physical Object")
                    .desired_width(120.0)
                    .show(ui);
                ui.end_row();
                ui.label("Mass");
                let res = ui
                    .add(egui::DragValue::new(&mut state.spawn_mass).clamp_range(0.0..=100_000.0));
                res.on_hover_text("Set to 0 for a static landscape collider");
                ui.end_row();
                ui.label("Friction");
                ui.add(
                    egui::DragValue::new(&mut state.spawn_friction)
                        .speed(0.05)
                        .clamp_range(0.0..=2.0),
                );
                ui.end_row();
                ui.label("Bouncy");
                ui.add(
                    egui::DragValue::new(&mut state.spawn_restitution)
                        .speed(0.05)
                        .clamp_range(0.0..=2.0),
                );
                ui.end_row();
            });
            ui.vertical(|ui| {
                ui.set_width(160.0);
                let res = ui.add(egui::TextEdit::singleline(&mut state.scene).desired_width(220.0));
                res.on_hover_text("Should end with #Scene0, leave empty to load an empty");

                if ui.button("Add new prop").clicked() {
                    load_data(world, &state);
                }
                if ui.button("Copy selected").clicked() {
                    copy_selected(world, hierarchy_state);
                }
            });
        });
    }
}

fn file_name(filename: &str) -> PathBuf {
    let root = crate::scene::get_base_path();
    let filename = if filename.is_empty() {
        DEFAULT_FILENAME
    } else {
        &filename
    };
    root.join(filename)
}
fn copy_selected(world: &mut World, hierarchy: &HierarchyState) {
    let to_copy: Vec<_> = hierarchy.selected.iter().collect();
    KlodScene::copy_objects(&to_copy, world);
}
fn load_data(
    world: &mut World,
    SceneWindowState {
        scene,
        spawn_name,
        spawn_mass,
        spawn_restitution,
        spawn_friction,
        ..
    }: &SceneWindowState,
) {
    let mut system_state =
        SystemState::<(Commands, Res<AssetServer>, ResMut<Assets<Mesh>>)>::new(world);
    let (mut cmds, assets, mut meshes) = system_state.get_mut(world);
    let data = if &*spawn_name == "" {
        let empty = Empty(SerdeCollider::Cuboid { half_extents: Vec3::new(10.0, 10.0, 10.0) });
        ObjectType::Empty(empty)
    } else if *spawn_mass == 0.0 {
        ObjectType::Scenery(SceneryData::new(
            SerdeCollider::Cuboid { half_extents: Vec3::new(10.0, 10.0, 10.0) },
            *spawn_friction,
            *spawn_restitution,
        ))
    } else {
        ObjectType::Agglomerable(AggloData::new(
            *spawn_mass,
            SerdeCollider::Cuboid { half_extents: Vec3::new(10.0, 10.0, 10.0) },
            *spawn_friction,
            *spawn_restitution,
        ))
    };
    let data = PhysicsObject::new(spawn_name.clone(), scene.clone(), default(), data);
    data.spawn(&mut cmds, &assets, &mut *meshes, true);
    system_state.apply(world);
}

fn ignore_transform_gizmo(mut cmds: Commands, gizmo_elems: Query<Entity, Added<PickableGizmo>>) {
    for added in &gizmo_elems {
        cmds.entity(added).insert(IgnoreEditorRayCast);
    }
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        use bevy_editor_pls::controls::ControlsWindow;
        use bevy_editor_pls_default_windows::inspector::InspectorWindow;
        app.add_plugins(DefaultPickingPlugins)
            .add_plugin(TransformGizmoPlugin::default())
            .add_plugin(EditorPlugin)
            .add_editor_window::<SceneWindow>()
            .set_default_panels::<ControlsWindow, SceneWindow, InspectorWindow>()
            .add_system_to_stage(CoreStage::PostUpdate, ignore_transform_gizmo)
            .add_system(err_sys!(toggle_editor_active));
    }
}
