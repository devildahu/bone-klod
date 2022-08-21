//! Menu and gameover screen ui.
mod common;
mod main_menu;

pub use common::UiAssets as Assets;

use bevy::prelude::{Plugin as BevyPlugin, *};

use crate::GameState;

#[cfg(feature = "debug")]
fn debug_buttons(
    mut ctx: ResMut<bevy_inspector_egui::bevy_egui::EguiContext>,
    mut events: EventWriter<crate::GameOver>,
) {
    use crate::{EndReason, GameOver};
    use bevy_inspector_egui::egui::*;
    Area::new("gameover::debug_buttons")
        .anchor(Align2::RIGHT_BOTTOM, vec2(0., 0.))
        .show(ctx.ctx_mut(), |ui| {
            if ui.button("PlayerWon").clicked() {
                events.send(GameOver(EndReason::Victory));
            }
            if ui.button("PlayerLost").clicked() {
                events.send(GameOver(EndReason::Loss));
            }
        });
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.add_system(debug_buttons);

        app.add_plugin(common::Plugin)
            .add_plugin(main_menu::Plugin(GameState::MainMenu));
    }
}
