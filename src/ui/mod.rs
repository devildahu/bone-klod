//! Menu and gameover screen ui.
mod common;
mod main_menu;

pub use common::UiAssets as Assets;

use bevy::prelude::{Plugin as BevyPlugin, *};

use crate::GameState;

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(common::Plugin)
            .add_plugin(main_menu::Plugin(GameState::MainMenu));
    }
}
