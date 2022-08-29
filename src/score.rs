use std::time::Duration;

use bevy::prelude::{Plugin as BevyPlugin, *};
use bevy_debug_text_overlay::screen_print;
use bevy_rapier3d::prelude::RapierContext;
use bevy_ui_build_macros::{build_ui, rect, size, style, unit};
use bevy_ui_navigation::prelude::{Focusable, NavEvent, NavEventReaderExt};

use crate::{
    audio::{AudioAssets, AudioRequest, AudioRequestSystem},
    ball::{anim::DestroyKlodEvent, BallSystems, Klod, KlodBall},
    cleanup_marked,
    state::GameState,
    system_helper::EasySystemSetCtor,
    ui::{self, MenuCursor},
};

struct Score {
    bone_mass: f32,
    time_remaining: f32,
    required_mana: f32,
}
impl Score {
    fn mana(&self) -> f32 {
        self.bone_mass * self.time_remaining
    }
    fn won(&self) -> bool {
        self.mana() > self.required_mana
    }

    fn hint(&self) -> &'static str {
        if self.time_remaining <= 0.0 {
            "Ran out of time"
        } else if !self.won() {
            "Not enough mana generated"
        } else {
            "Congratulations!"
        }
    }
    fn time_label(&self) -> String {
        format!("Time left: {:.0} seconds", self.time_remaining)
    }
    fn bone_mass_label(&self) -> String {
        format!("Bone mass: {:.0}", self.bone_mass)
    }
    fn mana_label(&self) -> String {
        format!("Mana generated: {:.0}", self.mana())
    }
}

pub(crate) struct GameData {
    main_timer: Timer,
    pub(crate) time: f32,
    pub(crate) required_score: f32,
}
impl GameData {
    pub(crate) fn new(time: f32, required_score: f32) -> Self {
        Self {
            time,
            main_timer: Timer::from_seconds(time, false),
            required_score,
        }
    }
    fn remaining(&self) -> f32 {
        self.time - self.main_timer.elapsed_secs()
    }
}

#[derive(Component)]
pub(crate) struct FinishLine;

fn init_timer(mut timer: ResMut<GameData>) {
    timer.main_timer = Timer::from_seconds(timer.time, false);
}

/// This system controls ticking the timer within the countdown resource and
/// handling its state.
fn countdown(
    time: Res<Time>,
    mut timer: ResMut<GameData>,
    mut destroy: EventWriter<DestroyKlodEvent>,
    mut state: ResMut<State<GameState>>,
    mut held_down: Local<f32>,
    gp_buttons: Res<Input<GamepadButton>>,
    keys: Res<Input<KeyCode>>,
) {
    timer.main_timer.tick(time.delta());
    screen_print!("Time remaining: {:.0}", timer.remaining());
    let gp_button = |button_type| GamepadButton { gamepad: Gamepad { id: 0 }, button_type };
    let gp_start = gp_button(GamepadButtonType::Start);
    let reset = keys.pressed(KeyCode::R) || gp_buttons.pressed(gp_start);
    if reset {
        *held_down += time.delta_seconds();
    } else {
        *held_down = 0.0;
    }
    if *held_down >= 1.0 {
        timer.main_timer.set_elapsed(Duration::from_secs(10_000));
    }
    if timer.main_timer.finished() {
        destroy.send(DestroyKlodEvent);
        state.set(GameState::TimeUp).unwrap();
    }
}

fn handle_finish(
    mut state: ResMut<State<GameState>>,
    finish_lines: Query<Entity, With<FinishLine>>,
    klods_query: Query<Entity, With<KlodBall>>,
    rapier_context: Res<RapierContext>,
    mut klods: Local<Vec<Entity>>,
) {
    klods.extend(&klods_query);
    for finish_line in &finish_lines {
        let not_line = |e1, e2| (e1 == finish_line).then(|| e2).unwrap_or(e1);
        let klod_at_finish = rapier_context
            .intersections_with(finish_line)
            .any(|(e1, e2, colliding)| colliding && klods.contains(&not_line(e1, e2)));
        if klod_at_finish {
            screen_print!("Reached finish line");
            state.set(GameState::GameComplete).unwrap();
        }
    }
    klods.clear();
}

#[derive(Component, Copy, Clone, Debug)]
struct ScoreboardUi;

#[derive(Component, Copy, Clone, Debug)]
enum ScoreboardElem {
    MainMenu,
    Retry,
}
fn setup_scoreboard(
    timer: Res<GameData>,
    klod: Query<&Klod>,
    mut cmds: Commands,
    ui_assets: Res<ui::Assets>,
) {
    use FlexDirection as FD;
    use ScoreboardElem::*;

    let bone_mass = match klod.get_single() {
        Ok(klod) => klod.weight(),
        Err(_) => return,
    };
    let score = Score {
        bone_mass,
        time_remaining: timer.remaining(),
        required_mana: timer.required_score,
    };

    let text_bundle = |content: &str, color: Color, font_size: f32| {
        let style = TextStyle {
            color,
            font: ui_assets.font.clone_weak(),
            font_size,
        };
        let text = Text::from_section(content, style);
        TextBundle { text, ..Default::default() }
    };
    let text = |content: &str| text_bundle(content, Color::ANTIQUE_WHITE, 30.0);
    let title_text = if score.won() {
        text_bundle("Ritual Completed!", Color::rgb_u8(0x63, 0x89, 0x61), 60.0)
    } else {
        text_bundle("The bones die again", Color::rgb_u8(0xc6, 0x18, 0x11), 60.0)
    };
    let hint_text = text(score.hint());

    let focusable = Focusable::new();

    let node = NodeBundle {
        color: Color::NONE.into(),
        style: bevy_ui_build_macros::style! {
            display: Display::Flex,
            flex_direction: FD::ColumnReverse,
            align_items: AlignItems::Center,
        },
        ..Default::default()
    };
    let cursor = MenuCursor::spawn_ui_element(&mut cmds);
    let name = Name::new;

    build_ui! {
        #[cmd(cmds)]
        node {
            min_size: size!(100 pct, 100 pct),
            flex_direction: FD::ColumnReverse,
            justify_content: JustifyContent::Center
        }[; name("Scoreboard root node"), ScoreboardUi ](
            id(cursor),
            node[title_text; name("Game status")],
            node[hint_text; name("Game Hints")],
            node {
                align_items: AlignItems::FlexStart,
                padding: rect!(40 px)
            }[; name("Scores container")](
                node[text(&score.bone_mass_label()); name("Bone Mass")],
                node[text(&score.time_label()); name("Time left")],
                node[text(&score.mana_label()); name("Mana")]
            ),
            node[text("Retry"); focusable, name("Retry"), Retry],
            node[text("Main menu"); focusable, name("Mainmenu"), MainMenu]
        )
    };
}

fn activate_scoreboard(
    mut events: EventReader<NavEvent>,
    mut state: ResMut<State<GameState>>,
    elems: Query<&ScoreboardElem>,
) {
    for activated in events.nav_iter().activated_in_query(&elems) {
        match activated {
            ScoreboardElem::MainMenu => state.set(GameState::MainMenu).unwrap(),
            ScoreboardElem::Retry => state.set(GameState::Playing).unwrap(),
        }
    }
}

fn tada(mut requests: EventWriter<AudioRequest>, audio: Res<AudioAssets>) {
    screen_print!("Tada!");
    requests.send(AudioRequest::StopMusic);
    requests.send(AudioRequest::PlayEffect(audio.tada(), 1.0));
}
fn times_up(mut state: ResMut<State<GameState>>) {
    state.set(GameState::GameComplete).unwrap();
}

pub(crate) struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(GameState::Playing.on_enter(init_timer))
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    .with_system(countdown.before(BallSystems::DestroyKlod).before(times_up))
                    .with_system(handle_finish),
            )
            .add_system_set(GameState::TimeUp.on_update(times_up.before(tada)))
            .add_system_set(GameState::GameComplete.on_enter(setup_scoreboard))
            .add_system_set(GameState::GameComplete.on_enter(tada.before(AudioRequestSystem)))
            .add_system_set(GameState::GameComplete.on_update(activate_scoreboard))
            .add_system_set(GameState::GameComplete.on_exit(cleanup_marked::<ScoreboardUi>))
        // This comment is here to prevent rustfmt from putting the semicolon up there
        ;
    }
}
