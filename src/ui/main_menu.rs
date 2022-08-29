use super::common::{MenuCursor, UiAssets};
use bevy::prelude::{Plugin as BevyPlugin, *};
use bevy::{app::AppExit, input::mouse::MouseMotion, window::WindowMode};
use bevy_debug_text_overlay::screen_print;
use bevy_ui_build_macros::{build_ui, rect, size, style, unit};
use bevy_ui_navigation::prelude::*;

use crate::audio::{AudioAssets, IntroTrack, MusicTrack};
use crate::{
    audio::{AudioRequest, AudioRequestSystem, SoundChannel},
    cleanup_marked,
    state::GameState,
};
use crate::{LightSwitch, UsesGamepad};

#[derive(Component)]
struct MovingSlider;

#[derive(Component, Clone)]
struct RulesOverlay;

#[derive(Component, Clone)]
struct CreditOverlay;

#[derive(Clone, Component)]
struct MainMenuRoot;

#[derive(Component, Clone, PartialEq)]
enum MainMenuElem {
    Start,
    Exit,
    Credits,
    Rules,
    LockMouse,
    ToggleFullScreen,
    ToggleGamepad,
    Set16_9,
    AudioSlider(SoundChannel, f32),
    ToggleLights,
}

pub struct MenuAssets {
    team_name: Handle<Image>,
    title_image: Handle<Image>,
    slider_handle: Handle<Image>,
    slider_bg: Handle<Image>,
}
impl FromWorld for MenuAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        Self {
            team_name: assets.load("team_logo.png"),
            title_image: assets.load("game_title.png"),
            slider_bg: assets.load("slider_background.png"),
            slider_handle: assets.load("slider_dongle.png"),
        }
    }
}

fn update_sliders(
    mut styles: Query<(Entity, &mut Style, &mut MainMenuElem), With<MovingSlider>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut cmds: Commands,
    mut audio_requests: EventWriter<AudioRequest>,
    mut nav_requests: EventWriter<NavRequest>,
    mut mouse_buttons: ResMut<Input<MouseButton>>,
    gp_axis: Res<Axis<GamepadAxis>>,
    gp_buttons: Res<Input<GamepadButton>>,
    uses_gamepad: Res<UsesGamepad>,
) {
    use MainMenuElem::AudioSlider;
    if let Ok((entity, mut style, mut elem)) = styles.get_single_mut() {
        if let (Val::Percent(left), AudioSlider(channel, strength)) =
            (style.position.left, elem.as_mut())
        {
            let gp_axis_kind = |axis_type| GamepadAxis { gamepad: Gamepad { id: 0 }, axis_type };
            let axis_x = gp_axis_kind(GamepadAxisType::LeftStickX);
            let gp_delta = gp_axis.get(axis_x).unwrap_or_default();
            let gp_delta = if gp_delta.abs() > 0.01 && uses_gamepad.yes {
                gp_delta
            } else {
                0.0
            };
            let mouse_delta: f32 = mouse_motion.iter().map(|m| m.delta.x).sum();
            let delta = mouse_delta + gp_delta;
            let new_left = (left / 0.9 + delta * 0.40).min(100.0).max(0.0);
            *strength = new_left;
            audio_requests.send(AudioRequest::SetVolume(*channel, new_left as f64 / 100.0));
            style.position.left = Val::Percent(new_left * 0.9)
        };
        let gp_button = |button_type| GamepadButton { gamepad: Gamepad { id: 0 }, button_type };
        let gp_a = gp_button(GamepadButtonType::South);
        let gp_b = gp_button(GamepadButtonType::East);
        if mouse_buttons.just_released(MouseButton::Left)
            || gp_buttons.just_pressed(gp_a)
            || gp_buttons.just_pressed(gp_b)
        {
            mouse_buttons.clear_just_released(MouseButton::Left);
            nav_requests.send(NavRequest::Unlock);
            screen_print!("Stop loop effect");
            audio_requests.send(AudioRequest::StopLoopEffect);
            cmds.entity(entity).remove::<MovingSlider>();
        }
    }
}
fn activate_sliders(
    mut audio_requests: EventWriter<AudioRequest>,
    mut events: EventReader<NavEvent>,
    mut nav_requests: EventWriter<NavRequest>,
    mut cmds: Commands,
    elems: Query<&MainMenuElem>,
) {
    let is_slider = |entity| matches!(elems.get(entity), Ok(MainMenuElem::AudioSlider(..)));
    let mut start_moving_slider = |slider| {
        nav_requests.send(NavRequest::Lock);
        cmds.entity(slider).insert(MovingSlider);
        audio_requests.send(AudioRequest::LoopEffect);
    };
    for activated in events.nav_iter().activated() {
        if is_slider(activated) {
            start_moving_slider(activated);
        }
    }
}

fn update_controller_text(
    uses_gamepad: Res<UsesGamepad>,
    mut gamepad_text: Query<(&mut Text, &MainMenuElem)>,
) {
    if uses_gamepad.is_changed() {
        for (mut text, elem) in &mut gamepad_text {
            if matches!(elem, MainMenuElem::ToggleGamepad) {
                let new_text = if uses_gamepad.yes {
                    "Disable Controller input"
                } else {
                    "Enable Controller input"
                };
                text.sections[0].value = new_text.to_owned();
            }
        }
    }
}

fn activate_menu(
    mut events: EventReader<NavEvent>,
    mut nav_requests: EventWriter<NavRequest>,
    mut exit: EventWriter<AppExit>,
    mut audio_requests: EventWriter<AudioRequest>,
    mut windows: ResMut<Windows>,
    mut game_state: ResMut<State<GameState>>,
    mut credit_overlay: Query<&mut Style, With<CreditOverlay>>,
    mut rules_overlay: Query<&mut Style, (Without<CreditOverlay>, With<RulesOverlay>)>,
    mut uses_gamepad: ResMut<UsesGamepad>,
    mut lights: Query<&mut Visibility, With<PointLight>>,
    mut light_switch: ResMut<LightSwitch>,
    mut ambient_light: ResMut<AmbientLight>,
    audio: Res<AudioAssets>,
    elems: Query<&MainMenuElem>,
) {
    let window_msg = "There is at least one game window open";
    for activated in events.nav_iter().activated_in_query(&elems) {
        audio_requests.send(AudioRequest::PlayEffect(audio.ui_click(), 0.05));
        match activated {
            MainMenuElem::Exit => exit.send(AppExit),
            MainMenuElem::Start => {
                screen_print!("Player pressed the start button");
                game_state.set(GameState::Playing).unwrap();
            }
            MainMenuElem::LockMouse => {
                let window = windows.get_primary_mut().expect(window_msg);
                let prev_lock_mode = window.cursor_locked();
                window.set_cursor_lock_mode(!prev_lock_mode);
            }
            MainMenuElem::ToggleFullScreen => {
                use WindowMode::*;
                let window = windows.get_primary_mut().expect(window_msg);
                let new_mode = if window.mode() == BorderlessFullscreen {
                    Windowed
                } else {
                    BorderlessFullscreen
                };
                window.set_mode(new_mode);
            }
            MainMenuElem::Set16_9 => {
                let window = windows.get_primary_mut().expect(window_msg);
                if window.mode() == WindowMode::Windowed {
                    let height = window.height();
                    window.set_resolution(height * 16.0 / 9.0, height);
                }
            }
            MainMenuElem::Credits => {
                let mut style = credit_overlay.single_mut();
                style.display = Display::Flex;
                nav_requests.send(NavRequest::Lock);
            }
            MainMenuElem::Rules => {
                let mut style = rules_overlay.single_mut();
                style.display = Display::Flex;
                nav_requests.send(NavRequest::Lock);
            }
            MainMenuElem::AudioSlider(_, _) => {}
            MainMenuElem::ToggleGamepad => uses_gamepad.yes = !uses_gamepad.yes,
            MainMenuElem::ToggleLights => {
                for mut vis in &mut lights {
                    vis.is_visible = !vis.is_visible;
                }
                light_switch.on = !light_switch.on;
                let ambient_brightness = if light_switch.on { 0.8 } else { 1.0 };
                ambient_light.brightness = ambient_brightness;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn leave_overlay(
    mut overlay: Query<&mut Style, Or<(With<CreditOverlay>, With<RulesOverlay>)>>,
    mut nav_requests: EventWriter<NavRequest>,
    gamepad: Res<Input<GamepadButton>>,
    mouse: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
) {
    if gamepad.get_just_pressed().len() != 0
        || mouse.get_just_pressed().len() != 0
        || keyboard.get_just_pressed().len() != 0
    {
        for mut style in overlay.iter_mut() {
            if style.display == Display::Flex {
                style.display = Display::None;
                nav_requests.send(NavRequest::Unlock)
            }
        }
    }
}

/// Spawns the UI tree
fn setup_main_menu(mut cmds: Commands, menu_assets: Res<MenuAssets>, ui_assets: Res<UiAssets>) {
    use FlexDirection as FD;
    use MainMenuElem::*;
    use PositionType as PT;

    let text_bundle = |content: &str, font_size: f32| ui_assets.text_bundle(content, font_size);
    let large_text = |content| ui_assets.large_text(content);
    let focusable = Focusable::default();
    let image =
        |image: &Handle<Image>| ImageBundle { image: image.clone().into(), ..Default::default() };
    let node = NodeBundle {
        color: Color::NONE.into(),
        style: style! {
            display: Display::Flex,
            flex_direction: FD::ColumnReverse,
            align_items: AlignItems::Center,
        },
        ..Default::default()
    };
    let mut slider = |name: &str, channel: SoundChannel, strength: f32| {
        let volume_name = name.to_string() + " volume";
        let handle_name = Name::new(name.to_string() + " volume slider handle");
        let slider_name = Name::new(name.to_string() + " volume slider");
        let position = UiRect {
            bottom: Val::Px(-10.0),
            left: Val::Percent(strength * 0.9),
            ..Default::default()
        };
        build_ui! {
            #[cmd(cmds)]
            node { flex_direction: FD::Row }[; slider_name](
                node[text_bundle(&volume_name, 30.0); style! { margin: rect!(10 px), }],
                node(
                    entity[
                        image(&menu_assets.slider_bg);
                        style! { size: size!( 200 px, 20 px), }
                    ],
                    entity[
                        image(&menu_assets.slider_handle);
                        focusable,
                        MainMenuElem::AudioSlider(channel, strength),
                        handle_name,
                        style! {
                            size: size!( 20 px, 40 px),
                            position_type: PT::Absolute,
                            position: position,
                        }
                    ]
                )
            )
        }
        .id()
    };
    let master_slider = slider("Master", SoundChannel::Master, 100.0);
    let sfx_slider = slider("Sfx", SoundChannel::Effects, 50.0);
    let music_slider = slider("Music", SoundChannel::Music, 50.0);
    let cursor = MenuCursor::spawn_ui_element(&mut cmds);

    build_ui! {
        #[cmd(cmds)]
        node{
            min_size: size!(100 pct, 100 pct),
            flex_direction: FD::ColumnReverse,
            justify_content: JustifyContent::FlexStart,
            padding: rect!(0 px, 0 px, 0 px, 10 pct,)
        }[; Name::new("Main menu root node"), MainMenuRoot](
            id(cursor),
            entity[
                image(&menu_assets.title_image);
                Name::new("Title card"),
                style! { size: size!(auto, 40 pct), }
            ],
            node{ flex_direction: FD::Row }[; Name::new("Menu columns")](
                node[; Name::new("Menu node")](
                    node[large_text("Start"); Focusable::new().prioritized(), Name::new("Start"), Start],
                    node[large_text("Credits"); focusable, Name::new("Credits"), Credits],
                    node[large_text("How to play"); focusable, Name::new("Rules"), Rules],
                    if (!cfg!(target_arch = "wasm32")) {
                        node[large_text("Exit"); focusable, Name::new("Exit"), Exit]
                    },
                ),
                node{ align_items: AlignItems::FlexEnd, margin: rect!(50 px) }[; Name::new("Audio settings")](
                    id(master_slider),
                    id(music_slider),
                    id(sfx_slider),
                ),
                node[; Name::new("Graphics column")](
                    if (!cfg!(target_arch = "wasm32")) {
                        node[large_text("Lock mouse cursor"); focusable, LockMouse],
                        node[large_text("Fit window to 16:9"); focusable, Set16_9],
                    },
                    node[large_text("Toggle Full screen"); focusable, ToggleFullScreen],
                    node[text_bundle("Disable Controller input", 34.0); focusable, ToggleGamepad],
                    node[text_bundle("Toggle lights (performance)", 34.0); focusable, ToggleLights],
                )
            ),
            node{
                position_type: PT::Absolute,
                position: rect!(10 pct),
                display: Display::None,
                justify_content: JustifyContent::Center
            }[; UiColor(Color::rgb(0.1, 0.1, 0.1)), Name::new("Rules overlay"), RulesOverlay](
                node {
                align_items: AlignItems::FlexStart
                }(
                    node[text_bundle("Story", 30.0);],
                    node[text_bundle("The infamous warlock Hieronymous Bonechill", 25.0);],
                    node[text_bundle("has started the ritual of bones! You are his minion", 25.0);],
                    node[text_bundle("and serve him... until the end of the ritual.", 25.0);],
                    node[text_bundle("Game", 30.0);],
                    node[text_bundle("Collect as many bones as possible and get to the end!", 25.0);],
                    node[text_bundle("You have two minutes until the end of the ritual.", 25.0);],
                    node[text_bundle("You must collect 1000 mana to win the game. Mana is equal", 25.0);],
                    node[text_bundle("to remaining time × bones collected. Heaver bones yield more", 25.0);],
                    node[text_bundle("mana, but make it harder to navigate the level.", 25.0);],
                    node[text_bundle("Doors", 30.0);],
                    node[text_bundle("You can collect more than just bones, some items", 25.0);],
                    node[text_bundle("let you open doors to secret rooms.", 25.0);],
                    node[text_bundle("Ground pound", 30.0);],
                    node[text_bundle("Press SPACE or gampead A to execute a ground pound,", 25.0);],
                    node[text_bundle("it has a 3 seconds cooldown.", 25.0);],
                    node[text_bundle("Reset", 30.0);],
                    node[text_bundle("If you get stuck or fall off the map, hold down R for 1 second", 25.0);],
                    node[text_bundle("to insta-lose. Use start on controller.", 25.0);],
                    node[text_bundle("Controls", 30.0);],
                    node[text_bundle("WASD/arrow keys/gamepad analog sticks", 25.0);],
                )
            ),
            node{
                position_type: PT::Absolute,
                position: rect!(10 pct),
                padding: rect!(10 px),
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween
            }[; UiColor(Color::rgb(0.1, 0.1, 0.1)), Name::new("Credits overlay"), CreditOverlay](
                node[
                    image(&menu_assets.team_name);
                    Name::new("Team name"),
                    style! { size: size!(auto, 45 pct), }
                ],
                node {
                align_items: AlignItems::FlexStart
                }(
                    node[large_text("music, code: Gibonus");],
                    node[large_text("sfx: Kenney (www.kenney.nl)");],
                    node[large_text("graphics: Xolotl");],
                    node[large_text("Thanks to the bevy community");],
                    node[text_bundle("(Click anywhere to exit)", 30.0);]
                )
            )
        )
    };
}

fn play_chill_music(mut requests: EventWriter<AudioRequest>, audio: Res<AudioAssets>) {
    requests.send(AudioRequest::QueueNewTrack(audio.track(IntroTrack::Chill)));
    requests.send(AudioRequest::QueueMusic(audio.track(MusicTrack::Chill)));
}

pub struct Plugin(pub GameState);
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        use crate::system_helper::EasySystemSetCtor;
        app.init_resource::<MenuAssets>()
            .add_system_set(self.0.on_enter(setup_main_menu))
            .add_system_set(self.0.on_enter(play_chill_music.before(AudioRequestSystem)))
            .add_system_set(self.0.on_exit(cleanup_marked::<MainMenuRoot>))
            .add_system_set(
                SystemSet::on_update(self.0)
                    .with_system(
                        update_sliders
                            .before(NavRequestSystem)
                            .before(AudioRequestSystem),
                    )
                    .with_system(
                        activate_sliders
                            .before(NavRequestSystem)
                            .before(AudioRequestSystem),
                    )
                    .with_system(update_controller_text.after(activate_menu))
                    .with_system(leave_overlay.before(NavRequestSystem))
                    .with_system(activate_menu.after(NavRequestSystem)),
            );
    }
}
