//! Play music and sound effects.
//!
//! Defines an [`AudioRequest`] event, reads them in [`play_audio`] system
//! using the kira backend for mixing and loudness controls.
use bevy::audio::AudioSink;
use bevy::prelude::{Plugin as BevyPlugin, *};

#[derive(SystemLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct AudioRequestSystem;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AudioChannel {
    Master,
    Sfx,
    Music,
}
struct ChannelVolumes {
    master: f32,
    sfx: f32,
    music: f32,
}
impl Default for ChannelVolumes {
    fn default() -> Self {
        Self { master: 1.0, sfx: 0.5, music: 0.5 }
    }
}

struct AudioAssets {
    wood_clink: Option<Handle<AudioSink>>,
    music: Handle<AudioSink>,
}
impl FromWorld for AudioAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        let sinks = world.resource::<Assets<AudioSink>>();
        let audio = world.resource::<Audio>();
        let mk_loop = |file: &str| {
            sinks.get_handle(audio.play_with_settings(assets.load(file), PlaybackSettings::LOOP))
        };
        Self { music: mk_loop("music.ogg"), wood_clink: None }
    }
}
trait AudioExt {
    fn get_sink(&self, sink: &Handle<AudioSink>) -> Option<&AudioSink>;
    fn pause(&self, sink: &Handle<AudioSink>) {
        if let Some(sink) = self.get_sink(sink) {
            sink.pause();
        }
    }
    fn set_volume(&self, sink: &Handle<AudioSink>, volume: f32) {
        if let Some(sink) = self.get_sink(sink) {
            sink.set_volume(volume);
        }
    }
}
impl AudioExt for Assets<AudioSink> {
    fn get_sink(&self, sink: &Handle<AudioSink>) -> Option<&AudioSink> {
        self.get(sink)
    }
}

pub enum SfxParam {
    StartLoop,
    PlayOnce,
}
pub enum AudioRequest {
    StopSfxLoop,
    PlayWoodClink(SfxParam),
    // StartMusic,
    SetVolume(AudioChannel, f32),
}
fn play_audio(
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
    mut assets: ResMut<AudioAssets>,
    sinks: Res<Assets<AudioSink>>,
    mut volumes: ResMut<ChannelVolumes>,
    mut events: EventReader<AudioRequest>,
) {
    for event in events.iter() {
        match event {
            // AudioRequest::StartMusic => {
            //     sinks.play(&assets.music);
            // }
            AudioRequest::SetVolume(AudioChannel::Sfx, volume) if *volume != volumes.sfx => {
                volumes.sfx = *volume;
                if let Some(clink) = &assets.wood_clink {
                    sinks.set_volume(clink, volume * volumes.master);
                }
            }
            AudioRequest::SetVolume(AudioChannel::Music, volume) if *volume != volumes.music => {
                volumes.music = *volume;
                sinks.set_volume(&assets.music, volume * volumes.master);
            }
            AudioRequest::SetVolume(AudioChannel::Master, volume) if *volume != volumes.master => {
                volumes.master = *volume;
                if let Some(clink) = &assets.wood_clink {
                    sinks.set_volume(clink, volume * volumes.sfx);
                }
                sinks.set_volume(&assets.music, volume * volumes.music);
            }
            // Volume is equal to what it is requested to be changed to
            AudioRequest::SetVolume(_, _) => {}
            AudioRequest::StopSfxLoop => {
                if let Some(clink) = &assets.wood_clink {
                    sinks.pause(clink);
                }
            }
            AudioRequest::PlayWoodClink(SfxParam::StartLoop) => {
                let mk_loop = |file: &str| {
                    sinks.get_handle(
                        audio.play_with_settings(asset_server.load(file), PlaybackSettings::LOOP),
                    )
                };
                assets.wood_clink = Some(mk_loop("wood_clink.ogg"));
            }
            AudioRequest::PlayWoodClink(SfxParam::PlayOnce) => {
                assets.wood_clink =
                    Some(sinks.get_handle(audio.play(asset_server.load("wood_clink.ogg"))));
            }
        }
    }
}

pub struct Plugin;
impl BevyPlugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChannelVolumes>()
            .init_resource::<AudioAssets>()
            .add_event::<AudioRequest>()
            .add_system(play_audio.label(AudioRequestSystem));
    }
}
