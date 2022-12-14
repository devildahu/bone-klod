mod animate;
mod audio;
mod ball;
#[cfg(feature = "editor")]
mod box_scene;
mod cam;
#[cfg(feature = "editor")]
mod editor;
mod game_audio;
mod powers;
mod prefabs;
mod scene;
mod score;
mod state;
mod system_helper;
mod ui;

#[cfg(feature = "editor")]
use std::env;

use bevy::{
    log::{Level, LogSettings},
    prelude::*,
};
use bevy_debug_text_overlay::screen_print;
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};
use scene::KlodScene;
use state::GameState;

/// Event to trigger a game over.
#[derive(Debug)]
pub struct GameOver(pub EndReason);

/// What triggered the game over.
#[derive(Debug)]
pub enum EndReason {
    Victory,
    Loss,
}

#[derive(Component, Clone)]
struct WaitRoot;

fn main() {
    let mut app = App::new();

    let initial_state = if cfg!(feature = "editor") {
        GameState::Playing
    } else {
        GameState::MainMenu
    };

    app.insert_resource(Msaa { samples: 4 })
        .insert_resource(LogSettings {
            level: Level::INFO,
            filter:
                "wgpu_core::device=warn,wgpu_hal=error,symphonia_core=warn,symphonia_format_ogg=warn"
                    .to_owned(),
        })
        .insert_resource(WindowDescriptor {
            #[cfg(target_os = "linux")]
            // workaround for https://github.com/bevyengine/bevy/issues/1908 (seems to be Mesa bug with X11 + Vulkan)
            present_mode: bevy::window::PresentMode::Immediate,
            ..default()
        })
        .add_state(initial_state)
        .add_plugins(DefaultPlugins);

    app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());

    #[cfg(all(feature = "debug", not(feature = "editor")))]
    app.add_plugin(bevy_inspector_egui::WorldInspectorPlugin::new());

    #[cfg(feature = "debug")]
    app.add_plugin(bevy_rapier3d::render::RapierDebugRenderPlugin::default())
        .add_plugin(bevy_inspector_egui_rapier::InspectableRapierPlugin)
        .add_plugin(bevy::pbr::wireframe::WireframePlugin)
        .insert_resource(bevy::render::settings::WgpuSettings {
            features: bevy::render::render_resource::WgpuFeatures::POLYGON_MODE_LINE,
            ..default()
        });

    #[cfg(feature = "editor")]
    app.add_plugin(bevy_scene_hook::HookPlugin)
        .add_plugin(editor::Plugin);

    #[cfg(feature = "editor")]
    if env::args().nth(1).as_deref() == Some("--load-greybox") {
        app.add_startup_system(box_scene::load_box_level)
            .add_system(box_scene::save_box_level.exclusive_system().at_start());
    }
    app.insert_resource(ClearColor(Color::rgb(0.293, 0.3828, 0.4023)))
        .init_resource::<LightSwitch>()
        .add_plugin(bevy_debug_text_overlay::OverlayPlugin { font_size: 24.0, ..default() })
        .add_plugin(scene::Plugin)
        .add_plugin(animate::Plugin)
        .add_plugin(powers::Plugin)
        .add_plugin(score::Plugin)
        .add_plugin(audio::Plugin)
        .add_plugin(game_audio::Plugin)
        .add_plugin(cam::Plugin)
        .add_plugin(ball::Plugin)
        .add_plugin(ui::Plugin)
        .add_event::<GameOver>()
        .add_startup_system(|| {
            screen_print!(sec: 10_000_000_000.0, "");
        })
        .add_startup_system(setup.exclusive_system().at_start());

    app.run();
}

pub fn cleanup_marked<T: Component>(mut cmds: Commands, query: Query<Entity, With<T>>) {
    screen_print!(sec: 3.0, "Cleaned up Something (can't show)");
    for entity in query.iter() {
        cmds.entity(entity).despawn_recursive();
    }
}

fn setup(world: &mut World) {
    let mut ambiant_light = world.resource_mut::<AmbientLight>();
    ambiant_light.color = Color::WHITE;
    ambiant_light.brightness = 0.8;
    #[cfg(not(target_family = "wasm"))]
    {
        let root = scene::get_base_path();
        KlodScene::load(world, root.join("default.klodlvl"));
    }
    #[cfg(target_family = "wasm")]
    {
        KlodScene::load_data(world, include_bytes!("../assets/default.klodlvl"));
    }
}

pub(crate) struct UsesGamepad {
    pub(crate) yes: bool,
}
pub(crate) struct LightSwitch {
    pub(crate) on: bool,
}
impl Default for UsesGamepad {
    fn default() -> Self {
        Self { yes: true }
    }
}
impl Default for LightSwitch {
    fn default() -> Self {
        Self { on: true }
    }
}

pub(crate) mod collision_groups {
    use bevy_rapier3d::prelude::CollisionGroups;

    pub(crate) const KLOD: CollisionGroups = CollisionGroups::new(0b00000001, 0b110110);
    pub(crate) const AGGLO: CollisionGroups = CollisionGroups::new(0b0000010, 0b001011);
    pub(crate) const SEEAG: CollisionGroups = CollisionGroups::new(0b0000010, 0b000011);
    pub(crate) const MUSIC: CollisionGroups = CollisionGroups::new(0b0000100, 0b000001);
    pub(crate) const CAM: CollisionGroups = CollisionGroups::new(0b000001000, 0b000011);
}
