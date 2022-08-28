use arrayvec::ArrayVec;
use bevy::prelude::{Plugin as BevyPlugin, *};
use bevy_debug_text_overlay::screen_print;
#[cfg(feature = "debug")]
use bevy_inspector_egui::{egui, Context, Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::{ContactForceEvent, RapierContext, Velocity};
use fastrand::usize as rand_usize;
use serde::Deserialize;

use crate::{
    audio::{AudioAssets, AudioRequest, AudioRequestSystem, ImpactSound, IntroTrack, MusicTrack},
    ball::{BallSystems::FreeFallUpdate, FreeFall, Klod, KlodBall, MAX_KLOD_SPEED},
};

#[cfg(feature = "debug")]
impl Inspectable for NoiseOnHit {
    type Attributes = <ImpactSound as Inspectable>::Attributes;
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        options: Self::Attributes,
        context: &mut Context<'_>,
    ) -> bool {
        let mut changed = false;

        ui.vertical(|ui| {
            let mut to_delete = None;

            let len = self.noises.len();
            for (i, elem) in self.noises.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let button = ui
                        .add(
                            egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED))
                                .frame(false),
                        )
                        .clicked();
                    if button {
                        to_delete = Some(i);
                    }
                    changed |= elem.ui(ui, options, &mut context.with_id(i as u64));
                });

                if i != len - 1 {
                    ui.separator();
                }
            }

            if len != 4 {
                ui.vertical_centered_justified(|ui| {
                    if ui.button("+").clicked() {
                        self.noises.push(ImpactSound::default());
                        changed = true;
                    }
                });
            }

            if let Some(i) = to_delete {
                self.noises.remove(i);
                changed = true;
            }
        });

        changed
    }
}

#[derive(Component)]
pub(crate) struct NoiseOnHit {
    pub(crate) noises: ArrayVec<ImpactSound, 4>,
}
impl NoiseOnHit {
    fn impact(&self) -> Option<ImpactSound> {
        match self.noises.len() {
            0 => None,
            nonzero => Some(self.noises[rand_usize(..nonzero)]),
        }
    }
}

// TODO
#[cfg_attr(feature = "debug", derive(Inspectable))]
#[cfg_attr(feature = "editor", derive(serde::Serialize))]
#[derive(Deserialize, Debug, Clone, Component, Copy, PartialEq, Eq)]
pub(crate) struct MusicTrigger {
    pub(crate) intro: Option<IntroTrack>,
    pub(crate) track: MusicTrack,
}

fn play_impact_sound(
    effects: Query<&NoiseOnHit>,
    audio: Res<AudioAssets>,
    mut collisions: EventReader<ContactForceEvent>,
    mut audio_requests: EventWriter<AudioRequest>,
) {
    for ContactForceEvent { collider1, collider2, total_force_magnitude, .. } in collisions.iter() {
        let effects = match (effects.get(*collider1), effects.get(*collider2)) {
            (Ok(effects), _) => effects,
            (_, Ok(effects)) => effects,
            _ => continue,
        };
        if let Some(to_play) = effects.impact() {
            let magnitude = *total_force_magnitude as f64 / 1000.0;
            let strength = (-1.0 / magnitude) + 1.0;
            if strength >= 0.0 {
                screen_print!(
                    sec: 0.8,
                    col: Color::BLUE,
                    "strength: {strength:.3}, noise: {to_play:?}"
                );
                audio_requests.send(AudioRequest::PlayEffect(audio.impact(to_play), strength));
            }
        }
    }
}
fn play_roll(
    mut audio_requests: EventWriter<AudioRequest>,
    free_fall: Query<(&FreeFall, ChangeTrackers<FreeFall>), With<Klod>>,
    klod: Query<&Velocity, With<Klod>>,
    time: Res<Time>,
) {
    let delta = time.delta_seconds_f64();
    let current_time = time.seconds_since_startup();
    let once_every = |t: f64| current_time % t < delta;

    let (free_falling, must_update) = match free_fall.get_single() {
        Ok((free_falling, changed)) => (free_falling.0, changed.is_changed()),
        Err(_) => return,
    };
    if !once_every(0.3) && !must_update {
        return;
    }
    if let Ok(velocity) = klod.get_single() {
        let magnitude = velocity.linvel.length();
        if magnitude > 1.0 && !free_falling {
            let volume = magnitude as f64 / MAX_KLOD_SPEED as f64;
            screen_print!(sec: 0.3, col: Color::RED, "strength: {volume:.3}, roll");
            audio_requests.send(AudioRequest::Roll(volume.min(1.0)));
        } else {
            audio_requests.send(AudioRequest::StopRoll);
        }
    }
}

fn trigger_music(
    ball: Query<Entity, With<KlodBall>>,
    triggers: Query<&MusicTrigger>,
    rapier_context: Res<RapierContext>,
    audio: Res<AudioAssets>,
    mut audio_requests: EventWriter<AudioRequest>,
    mut current_trigger: Local<Option<MusicTrigger>>,
    time: Res<Time>,
) {
    let delta = time.delta_seconds_f64();
    let current_time = time.seconds_since_startup();
    let once_every = |t: f64| current_time % t < delta;

    if !once_every(0.8) || triggers.is_empty() {
        return;
    }
    let ball = match ball.get_single() {
        Ok(ball) => ball,
        Err(_) => return,
    };
    let not_ball = |e1, e2| (e1 == ball).then(|| e2).unwrap_or(e1);
    let trigger = rapier_context
        .intersections_with(ball)
        .filter_map(|c| c.2.then(|| not_ball(c.0, c.1)))
        .find_map(|t| triggers.get(t).ok());
    if let Some(trigger) = trigger {
        if Some(*trigger) != *current_trigger {
            screen_print!(sec: 3.0, col: Color::LIME_GREEN, "trigger_music: {trigger:?}");
            *current_trigger = Some(*trigger);
            if let Some(intro) = trigger.intro {
                audio_requests.send(AudioRequest::QueueNewTrack(audio.track(intro)));
                audio_requests.send(AudioRequest::QueueMusic(audio.track(trigger.track)));
            } else {
                audio_requests.send(AudioRequest::QueueNewTrack(audio.track(trigger.track)));
            }
        }
    }
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<NoiseOnHit>()
            .register_inspectable::<MusicTrigger>();

        app.add_system(play_impact_sound.before(AudioRequestSystem))
            .add_system(trigger_music.before(AudioRequestSystem))
            .add_system(play_roll.before(AudioRequestSystem).after(FreeFallUpdate));
    }
}
