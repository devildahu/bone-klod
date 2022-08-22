use bevy::prelude::*;

mod animate;
mod audio;
mod cam;
mod ball;
#[cfg(feature = "editor")]
mod editor;
mod state;
mod system_helper;
mod ui;

use bevy_rapier3d::{render::RapierDebugRenderPlugin, prelude::{RapierPhysicsPlugin, NoUserData}};
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
    use system_helper::EasySystemSetCtor;

    let mut app = App::new();

    app.insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            #[cfg(target_os = "linux")]
            // workaround for https://github.com/bevyengine/bevy/issues/1908 (seems to be Mesa bug with X11 + Vulkan)
            present_mode: bevy::window::PresentMode::Immediate, 
            ..default()
        })
        .add_state(GameState::MainMenu)
        .add_plugins(DefaultPlugins);

    app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());

    #[cfg(all(feature = "debug", not(feature = "editor")))]
    app.add_plugin(bevy_inspector_egui::WorldInspectorPlugin::new());

    #[cfg(feature = "debug")]
    app.add_plugin(RapierDebugRenderPlugin::default())
        // .add_plugin(bevy_inspector_egui_rapier::InspectableRapierPlugin)
        .add_plugin(bevy::pbr::wireframe::WireframePlugin)
        .insert_resource(bevy::render::settings::WgpuSettings {
            features: bevy::render::render_resource::WgpuFeatures::POLYGON_MODE_LINE,
            ..default()
        });
    
    #[cfg(feature="editor")]
    app.add_plugin(editor::Plugin);

    app.insert_resource(ClearColor(Color::rgb(0.293, 0.3828, 0.4023)))
        .add_plugin(bevy_scene_hook::HookPlugin)
        .add_plugin(bevy_debug_text_overlay::OverlayPlugin::default())
        .add_plugin(animate::Plugin)
        .add_plugin(cam::Plugin)
        .add_plugin(ball::Plugin)
        .add_plugin(audio::Plugin)
        .add_plugin(ui::Plugin)
        .add_event::<GameOver>()
        .add_system_set(GameState::WaitLoaded.on_exit(cleanup_marked::<WaitRoot>))
        .add_startup_system(setup);

    app.run();
}

pub fn cleanup_marked<T: Component>(mut cmds: Commands, query: Query<Entity, With<T>>) {
    use bevy_debug_text_overlay::screen_print;
    screen_print!(sec: 3.0, "Cleaned up Something (can't show)");
    for entity in query.iter() {
        cmds.entity(entity).despawn_recursive();
    }
}

fn setup(
    mut ambiant_light: ResMut<AmbientLight>,
    // mut cmds: Commands,
    // mut audio_events: EventWriter<audio::AudioRequest>,
) {
    *ambiant_light = AmbientLight { color: Color::WHITE, brightness: 1.0 };
    // audio_events.send(audio::AudioRequest::StartMusic);
}
