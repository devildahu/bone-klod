mod trans;

use std::path::PathBuf;

use bevy::{
    ecs::system::SystemState,
    hierarchy::despawn_with_children_recursive,
    prelude::{Plugin as BevyPlugin, *},
    ui::FocusPolicy,
};

use bevy_editor_pls::{
    editor_window::{EditorWindow, EditorWindowContext},
    prelude::*,
    EditorEvent,
};
use bevy_editor_pls_default_windows::{
    cameras::ActiveEditorCamera,
    hierarchy::{picking::IgnoreEditorRayCast, HideInEditor, HierarchyState, HierarchyWindow},
};
use bevy_inspector_egui::{egui, options::OptionAttributes, Inspectable};
use bevy_mod_picking::{DefaultPickingPlugins, PickableMesh, PickingCameraBundle, Selection};
use bevy_rapier3d::prelude::{Collider, DebugLinesMesh, RapierConfiguration, Sensor};
use bevy_transform_gizmo::{
    GizmoPickSource, InternalGizmoCamera, PickableGizmo, TransformGizmo, TransformGizmoPlugin,
};

use crate::{
    audio::{ImpactSound, IntroTrack, MusicTrack},
    cam::OrbitCamera,
    collision_groups as groups,
    game_audio::MusicTrigger,
    powers::Power,
    prefabs::{AggloData, Scenery, SerdeCollider},
    scene::{reset_scene, save_scene, KlodScene, ObjectType, PhysicsObject},
    state::GameState,
    system_helper::EasySystemSetCtor,
};

fn toggle_editor_active(
    mut cmds: Commands,
    mut events: EventReader<EditorEvent>,
    mut rapier_config: ResMut<RapierConfiguration>,
    mut orbit_cam: Query<&mut OrbitCamera>,
    editor_cam: Query<Entity, With<ActiveEditorCamera>>,
    mut gizmo_camera: Query<&mut Camera, With<InternalGizmoCamera>>,
    mut game_state: ResMut<State<GameState>>,
) -> Option<()> {
    for event in events.iter() {
        match event {
            EditorEvent::Toggle { now_active } => {
                let mut orbit_cam = orbit_cam.get_single_mut().ok()?;
                orbit_cam.locked = *now_active;
                let cam = editor_cam.get_single().ok()?;
                if *now_active {
                    game_state.push(GameState::Editor).unwrap();
                    rapier_config.physics_pipeline_active = false;
                    cmds.entity(cam)
                        .insert_bundle(PickingCameraBundle::default())
                        .insert(GizmoPickSource::default());
                } else {
                    game_state.pop().unwrap();
                    rapier_config.physics_pipeline_active = true;
                    cmds.entity(cam)
                        .remove_bundle::<PickingCameraBundle>()
                        .remove::<GizmoPickSource>();
                    let mut gizmo_camera = gizmo_camera.get_single_mut().ok()?;
                    gizmo_camera.is_active = false;
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

pub struct SceneWindowState {
    filename: String,
    scene: String,
    name: String,
    spawn_mass: f32,
    spawn_restitution: f32,
    spawn_friction: f32,
    music: MusicTrack,
    music_start: Option<IntroTrack>,
    power: Power,
    scene_save_result: Option<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
}
impl Default for SceneWindowState {
    fn default() -> Self {
        SceneWindowState {
            filename: default(),
            scene: default(),
            name: default(),
            spawn_mass: 0.5,
            spawn_restitution: 0.4,
            spawn_friction: 0.8,
            music: default(),
            music_start: default(),
            power: default(),
            scene_save_result: default(),
        }
    }
}

const DEFAULT_FILENAME: &str = "default.klodlvl";

pub struct SceneWindow;
impl EditorWindow for SceneWindow {
    type State = SceneWindowState;
    const NAME: &'static str = "Level Management";

    fn ui(world: &mut World, mut ctx: EditorWindowContext, ui: &mut egui::Ui) {
        let (state, hierarchy_state) = match ctx.state_mut_pair::<SceneWindow, HierarchyWindow>() {
            (Some(state), Some(hierarchy)) => (state, hierarchy),
            _ => return,
        };
        {
            let input = ui.input();
            if input.key_pressed(egui::Key::D) && input.modifiers.ctrl {
                copy_selected(world, hierarchy_state);
            }
            if input.key_pressed(egui::Key::X) && input.modifiers.ctrl {
                despawn_selected(world, hierarchy_state);
            }
        }
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
                ui.label("Power");
                let selected = state.power.to_string();
                let ret = egui::ComboBox::from_id_source(ui.id())
                    .selected_text(&selected)
                    .show_ui(ui, |ui| {
                        macro_rules! select_menu { ($($name: expr => $value: expr,)*) => {
                            $( if ui.selectable_label($value == state.power, $name).clicked() {
                                state.power = $value;
                            } )*
                        } }
                        select_menu! {
                            "Fire" => Power::Fire,
                            "Water" => Power::Water,
                            "Cat" => Power::Cat,
                            "AmberRod" => Power::AmberRod,
                            "Dig" => Power::Dig,
                            "Saw" => Power::Saw,
                            "None" => Power::None,
                        }
                    });
                ret.response.on_hover_text(
                    "Power granted by Agglomerable OR make a Scenery item destructible, \
                    use the Inspector to set more powers needed to destroy the item.",
                );
                ui.end_row();
                ui.label("Name");
                egui::TextEdit::singleline(&mut state.name)
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
                    spawn_object(world, &state);
                }
                if ui.button("Copy selected").clicked() {
                    copy_selected(world, hierarchy_state);
                }
            });
            ui.vertical(|ui| {
                ui.set_width(160.0);
                state.music.ui_raw(ui, ());
                ui.horizontal(|ui| {
                    ui.label("Intro");
                    state.music_start.ui_raw(
                        ui,
                        OptionAttributes {
                            replacement: Some(IntroTrack::default),
                            ..default()
                        },
                    );
                });
                if ui.button("Spawn music trigger area").clicked() {
                    spawn_music_trigger(world, state.music, state.music_start, state.name.clone());
                }
            });
        });
    }
}

fn spawn_music_trigger(
    world: &mut World,
    track: MusicTrack,
    intro: Option<IntroTrack>,
    name: String,
) {
    let name = if name.is_empty() {
        "Music trigger".to_owned()
    } else {
        name
    };
    let collider = SerdeCollider::Cuboid { half_extents: Vec3::splat(30.0) };
    world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
        world.spawn().insert_bundle((
            Name::new(name),
            MusicTrigger { intro, track },
            Sensor,
            groups::MUSIC,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
            Collider::from(collider.clone()),
            meshes.add(collider.into()),
            PickableMesh::default(),
            Interaction::default(),
            FocusPolicy::default(),
            Selection::default(),
            bevy_transform_gizmo::GizmoTransformable,
        ));
    });
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
fn despawn_selected(world: &mut World, hierarchy: &HierarchyState) {
    for selected in hierarchy.selected.iter() {
        despawn_with_children_recursive(world, selected);
    }
}
fn copy_selected(world: &mut World, hierarchy: &HierarchyState) {
    let to_copy: Vec<_> = hierarchy.selected.iter().collect();
    KlodScene::copy_objects(&to_copy, world);
}
fn spawn_object(
    world: &mut World,
    SceneWindowState {
        scene,
        name,
        spawn_mass,
        spawn_restitution,
        spawn_friction,
        power,
        ..
    }: &SceneWindowState,
) {
    let mut system_state =
        SystemState::<(Commands, Res<AssetServer>, ResMut<Assets<Mesh>>)>::new(world);
    let (mut cmds, assets, mut meshes) = system_state.get_mut(world);
    let data = if &*name == "" || *spawn_mass == 0.0 {
        let power = *power;
        let weakness = if power != Power::None { vec![power] } else { Vec::new() };
        ObjectType::Scenery(Scenery { weakness })
    } else {
        ObjectType::Agglomerable(AggloData::new(*spawn_mass, *power))
    };
    let data = PhysicsObject::new(
        name.clone(),
        Some(scene.clone()),
        default(),
        SerdeCollider::Cuboid { half_extents: Vec3::splat(10.0) },
        *spawn_friction,
        *spawn_restitution,
        vec![ImpactSound::GenericMetal],
        data,
    );
    data.spawn(&mut cmds, &assets, &mut *meshes, true);
    system_state.apply(world);
}

fn ignore_transform_gizmo(
    mut cmds: Commands,
    gizmo_elems: Query<Entity, Or<(Added<PickableGizmo>, Added<TransformGizmo>)>>,
) {
    for added in &gizmo_elems {
        cmds.entity(added).insert(IgnoreEditorRayCast);
    }
}

fn ignore_rapier_wireframes(
    mut cmds: Commands,
    to_ignore: Query<Entity, (With<DebugLinesMesh>, Without<HideInEditor>)>,
) {
    to_ignore.for_each(|entity| {
        cmds.entity(entity).insert(HideInEditor);
    });
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        use bevy_editor_pls::controls::ControlsWindow;
        use bevy_editor_pls_default_windows::inspector::InspectorWindow;
        app.add_plugins(DefaultPickingPlugins)
            .add_plugin(TransformGizmoPlugin::default())
            .add_plugin(EditorPlugin)
            .add_plugin(trans::Plugin)
            .add_editor_window::<SceneWindow>()
            .set_default_panels::<ControlsWindow, SceneWindow, InspectorWindow>()
            .add_system_to_stage(CoreStage::PostUpdate, ignore_transform_gizmo)
            .add_system_set(GameState::Editor.on_enter(reset_scene.exclusive_system().at_end()))
            .add_system_set(GameState::Editor.on_exit(save_scene.exclusive_system().at_end()))
            .add_system(ignore_rapier_wireframes)
            .add_system(err_sys!(toggle_editor_active));
    }
}
