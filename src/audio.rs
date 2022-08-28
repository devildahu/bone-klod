//! Play music and sound effects.
//!
//! Defines an [`AudioRequest`] event, reads them in [`play_audio`] system
//! using the kira backend for mixing and loudness controls.
use std::collections::VecDeque;

use bevy::prelude::{Plugin as BevyPlugin, *};
use bevy_debug_text_overlay::screen_print;
#[cfg(feature = "debug")]
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_kira_audio::prelude::*;
use enum_map::{enum_map, Enum, EnumMap};
use fastrand::usize as rand_usize;
use serde::{Deserialize, Serialize};

pub(crate) type Sfx = Handle<AudioSource>;

#[derive(SystemLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct AudioRequestSystem;

enum Effects {}
enum Roll {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundChannel {
    Master,
    Effects,
    Music,
}
struct AudioState {
    queue: VecDeque<Sfx>,
    volumes: ChannelVolumes,
    playing: Option<Handle<AudioInstance>>,
    stop_current_track: bool,
    stop_loop_effect: bool,
}
struct ChannelVolumes {
    master: f64,
    effect: f64,
    music: f64,
}
impl Default for AudioState {
    fn default() -> Self {
        AudioState {
            queue: VecDeque::new(),
            volumes: ChannelVolumes { master: 1.0, effect: 0.5, music: 0.5 },
            playing: None,
            stop_current_track: false,
            stop_loop_effect: false,
        }
    }
}

pub(crate) enum AudioRequest {
    PlayEffect(Sfx, f64),
    QueueMusic(Sfx),
    QueueNewTrack(Sfx),
    StopMusic,
    SetVolume(SoundChannel, f64),
    Roll(f64),
    StopRoll,
    LoopEffect,
    StopLoopEffect,
}
fn handle_requests(
    music_channel: Res<Audio>,
    effect_channel: Res<AudioChannel<Effects>>,
    roll_channel: Res<AudioChannel<Roll>>,
    assets: Res<AudioAssets>,
    mut state: ResMut<AudioState>,
    mut events: EventReader<AudioRequest>,
) {
    for event in events.iter() {
        match event {
            AudioRequest::SetVolume(SoundChannel::Effects, volume)
                if *volume != state.volumes.effect =>
            {
                state.volumes.effect = *volume;
                effect_channel.set_volume(*volume * state.volumes.master);
                roll_channel.set_volume(*volume * state.volumes.master);
            }
            AudioRequest::SetVolume(SoundChannel::Music, volume)
                if *volume != state.volumes.music =>
            {
                state.volumes.music = *volume;
                music_channel.set_volume(*volume * state.volumes.master);
            }
            AudioRequest::SetVolume(SoundChannel::Master, volume)
                if *volume != state.volumes.master =>
            {
                state.volumes.master = *volume;
                effect_channel.set_volume(volume * state.volumes.effect);
                roll_channel.set_volume(volume * state.volumes.effect);
                music_channel.set_volume(volume * state.volumes.music);
            }
            // Volume is equal to what it is requested to be changed to
            AudioRequest::SetVolume(_, _) => {}
            AudioRequest::PlayEffect(effect, volume) => {
                if !effect_channel.is_playing_sound() {
                    effect_channel
                        .play(effect.clone_weak())
                        .with_volume(*volume);
                }
            }
            AudioRequest::QueueNewTrack(music) => {
                state.queue.clear();
                state.queue.push_back(music.clone_weak());
                state.stop_current_track = true;
            }
            AudioRequest::QueueMusic(music) => state.queue.push_back(music.clone_weak()),
            AudioRequest::LoopEffect => {
                effect_channel.play(assets.wood_clink.clone_weak()).looped();
            }
            AudioRequest::StopLoopEffect => state.stop_loop_effect = true,
            AudioRequest::Roll(roll_speed) => {
                let volume = state.volumes.master * state.volumes.effect * roll_speed;
                let pitch = 1.0 + *roll_speed * 0.6;
                roll_channel.set_volume(volume);
                roll_channel.set_playback_rate(pitch);
                if !roll_channel.is_playing_sound() {
                    roll_channel.play(assets.roll.clone_weak()).looped();
                }
            }
            AudioRequest::StopRoll => {
                roll_channel.stop();
            }
            AudioRequest::StopMusic => {
                state.stop_current_track = true;
                state.queue.clear();
            }
        }
    }
}
fn play_music(
    mut state: ResMut<AudioState>,
    music_channel: Res<Audio>,
    effect_channel: Res<AudioChannel<Effects>>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    if state.stop_current_track {
        screen_print!("Stopping audoi");
        state.stop_current_track = false;
        if let Some(current) = state.playing.as_ref().and_then(|h| instances.get_mut(h)) {
            current.stop(AudioTween::default());
        }
    }
    let playback_state = state
        .playing
        .as_ref()
        .map(|playing| music_channel.state(playing));
    if matches!(playback_state, Some(PlaybackState::Stopped) | None) {
        if let Some(to_play) = state.queue.front() {
            let to_play = to_play.clone_weak();
            if state.queue.len() > 1 {
                state.queue.pop_front();
            }
            state.playing = Some(music_channel.play(to_play).handle());
        }
    }
    if state.stop_loop_effect {
        effect_channel.stop();
        state.stop_loop_effect = false;
    }
}

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[cfg_attr(feature = "editor", derive(Serialize))]
#[derive(Deserialize, Debug, Clone, Default, Copy)]
pub(crate) enum Pitch {
    High,
    #[default]
    Medium,
    Low,
}
#[cfg_attr(feature = "debug", derive(Inspectable))]
#[cfg_attr(feature = "editor", derive(Serialize))]
#[derive(Deserialize, Debug, Clone, Default, Copy)]
pub(crate) enum ImpactSound {
    Explosion,
    Bell,
    #[default]
    Generic,
    Mining,
    Plank,
    PunchHeavy,
    PunchMedium,
    SoftHeavy,
    SoftMedium,
    GenericMetal,
    Metal(Pitch),
    Glass(Pitch),
    Plate(Pitch),
    Wood(Pitch),
}

#[derive(Debug, Enum)]
enum FullImpactType {
    Metal,
    Glass,
    Plate,
    Wood,
}
#[derive(Debug, Enum)]
enum PartialImpactType {
    Bell,
    Generic,
    GenericMetal,
    Mining,
    Plank,
    PunchHeavy,
    PunchMedium,
    SoftHeavy,
    SoftMedium,
}
struct FullImpact {
    hight: Impact,
    medium: Impact,
    low: Impact,
}
impl FullImpact {
    fn of_weight(&self, weight: Pitch) -> &Impact {
        match weight {
            Pitch::High => &self.hight,
            Pitch::Medium => &self.medium,
            Pitch::Low => &self.low,
        }
    }
    fn from_name(assets: &AssetServer, name: &str) -> Self {
        FullImpact {
            hight: Impact::from_name(assets, &(name.to_owned() + "_light")),
            medium: Impact::from_name(assets, &(name.to_owned() + "_medium")),
            low: Impact::from_name(assets, &(name.to_owned() + "_heavy")),
        }
    }
}
struct Impact(Sfxs);
struct Sfxs([Sfx; 5]);
impl Sfxs {
    fn pick(&self) -> Sfx {
        self.0[rand_usize(..self.0.len())].clone_weak()
    }
    fn from_name(assets: &AssetServer, name: &str) -> Self {
        let name = "sfx/".to_owned() + name;
        Sfxs([
            assets.load(&(name.clone() + "_000.ogg")),
            assets.load(&(name.clone() + "_001.ogg")),
            assets.load(&(name.clone() + "_002.ogg")),
            assets.load(&(name.clone() + "_003.ogg")),
            assets.load(&(name + "_004.ogg")),
        ])
    }
}
impl Impact {
    fn pick(&self) -> Sfx {
        self.0.pick()
    }
    fn from_name(assets: &AssetServer, name: &str) -> Self {
        let name = "impacts/impact".to_owned() + name;
        Impact(Sfxs::from_name(assets, &name))
    }
}

// footstep{_carpet,_concrete,_grass,_snow,_wood,00..09}
pub(crate) struct AudioAssets {
    wood_clink: Sfx,
    full_impacts: EnumMap<FullImpactType, FullImpact>,
    impacts: EnumMap<PartialImpactType, Impact>,
    explosion: Sfxs,
    roll: Sfx,
    tada: Sfx,
    music: EnumMap<MusicTrack, Sfx>,
    intros: EnumMap<IntroTrack, Sfx>,
}
impl AudioAssets {
    pub(crate) fn ui_click(&self) -> Sfx {
        self.wood_clink.clone_weak()
    }
    pub(crate) fn track(&self, track: impl Into<Track>) -> Sfx {
        match track.into() {
            Track::Music(music) => self.music[music].clone_weak(),
            Track::Intro(intro) => self.intros[intro].clone_weak(),
        }
    }
    pub(crate) fn impact(&self, sound: ImpactSound) -> Sfx {
        use FullImpactType as Full;
        use PartialImpactType as Partial;
        match sound {
            ImpactSound::Bell => self.impacts[Partial::Bell].pick(),
            ImpactSound::Plank => self.impacts[Partial::Plank].pick(),
            ImpactSound::Mining => self.impacts[Partial::Mining].pick(),
            ImpactSound::Generic => self.impacts[Partial::Generic].pick(),
            ImpactSound::Explosion => self.explosion.pick(),
            ImpactSound::SoftHeavy => self.impacts[Partial::SoftHeavy].pick(),
            ImpactSound::SoftMedium => self.impacts[Partial::SoftMedium].pick(),
            ImpactSound::PunchHeavy => self.impacts[Partial::PunchHeavy].pick(),
            ImpactSound::PunchMedium => self.impacts[Partial::PunchMedium].pick(),
            ImpactSound::GenericMetal => self.impacts[Partial::GenericMetal].pick(),
            ImpactSound::Wood(weight) => self.full_impacts[Full::Wood].of_weight(weight).pick(),
            ImpactSound::Metal(weight) => self.full_impacts[Full::Metal].of_weight(weight).pick(),
            ImpactSound::Glass(weight) => self.full_impacts[Full::Glass].of_weight(weight).pick(),
            ImpactSound::Plate(weight) => self.full_impacts[Full::Plate].of_weight(weight).pick(),
        }
    }

    pub(crate) fn tada(&self) -> Sfx {
        self.tada.clone_weak()
    }
}
impl FromWorld for AudioAssets {
    fn from_world(world: &mut World) -> Self {
        use FullImpactType::*;
        use IntroTrack as In;
        use MusicTrack as Mu;
        use PartialImpactType::*;
        let assets = world.resource::<AssetServer>();
        AudioAssets {
            wood_clink: assets.load("sfx/wood_clink.ogg"),
            roll: assets.load("sfx/roll.ogg"),
            full_impacts: enum_map! {
                Wood => FullImpact::from_name(&assets, "Wood"),
                Metal => FullImpact::from_name(&assets, "Metal"),
                Glass => FullImpact::from_name(&assets, "Glass"),
                Plate => FullImpact::from_name(&assets, "Plate"),
            },
            impacts: enum_map! {
                Bell => Impact::from_name(&assets, "Bell_heavy"),
                Plank => Impact::from_name(&assets, "Plank_medium"),
                Mining => Impact::from_name(&assets, "Mining"),
                Generic => Impact::from_name(&assets, "Generic_light"),
                PunchHeavy => Impact::from_name(&assets, "Punch_heavy"),
                PunchMedium => Impact::from_name(&assets, "Punch_medium"),
                SoftHeavy => Impact::from_name(&assets, "Soft_heavy"),
                SoftMedium => Impact::from_name(&assets, "Soft_medium"),
                GenericMetal => Impact::from_name(&assets, "Metal"),
            },
            explosion: Sfxs::from_name(&assets, "explosionCrunch"),
            music: enum_map! {
                Mu::Chill => assets.load("music/chill.ogg"),
                Mu::Theremin => assets.load("music/theremin.ogg"),
                Mu::Orchestral => assets.load("music/orchestral.ogg"),
                Mu::OrchestralFinale => assets.load("music/orchestralFinale.ogg"),
            },
            intros: enum_map! {
                In::Chill => assets.load("music/introChill.ogg"),
                In::Theremin => assets.load("music/introTheremin.ogg"),
            },
            tada: assets.load("sfx/tada.ogg"),
        }
    }
}

#[cfg_attr(feature = "debug", derive(Inspectable))]
#[cfg_attr(feature = "editor", derive(Serialize))]
#[derive(Deserialize, Debug, Clone, Default, Copy, PartialEq, Eq, Enum)]
pub(crate) enum MusicTrack {
    #[default]
    Chill,
    Theremin,
    Orchestral,
    OrchestralFinale,
}
#[cfg_attr(feature = "debug", derive(Inspectable))]
#[cfg_attr(feature = "editor", derive(Serialize))]
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, Enum)]
pub(crate) enum IntroTrack {
    #[default]
    Chill,
    Theremin,
}
pub(crate) enum Track {
    Intro(IntroTrack),
    Music(MusicTrack),
}
impl From<MusicTrack> for Track {
    fn from(track: MusicTrack) -> Self {
        Track::Music(track)
    }
}
impl From<IntroTrack> for Track {
    fn from(track: IntroTrack) -> Self {
        Track::Intro(track)
    }
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.register_inspectable::<ImpactSound>()
            .register_inspectable::<MusicTrack>()
            .register_inspectable::<IntroTrack>()
            .register_inspectable::<Pitch>();

        app.add_plugin(AudioPlugin)
            .add_audio_channel::<Effects>()
            .add_audio_channel::<Roll>()
            .init_resource::<AudioState>()
            .init_resource::<AudioAssets>()
            .add_event::<AudioRequest>()
            .add_system(handle_requests.label(AudioRequestSystem))
            .add_system(play_music.after(AudioRequestSystem));
    }
}
