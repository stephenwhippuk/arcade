use crate::events::GameEvent;
use macroquad::audio::{
    load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioSettings {
    pub volume: u8,
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { volume: 60, muted: false }
    }
}

impl AudioSettings {
    pub fn gain(self) -> f32 {
        if self.muted { 0.0 } else { f32::from(self.volume) / 100.0 }
    }

    pub fn adjust(&mut self, increase: bool) {
        self.volume = if increase {
            self.volume.saturating_add(10).min(100)
        } else {
            self.volume.saturating_sub(10)
        };
    }
}

pub struct Audio {
    sounds: Vec<(GameEvent, Sound)>,
    settings: AudioSettings,
}

impl Audio {
    pub async fn load() -> Result<Self, macroquad::Error> {
        let mut sounds = Vec::new();
        for (event, notes) in [
            (GameEvent::Collect, vec![880.0, 1320.0]),
            (GameEvent::Shoot, vec![700.0, 350.0]),
            (GameEvent::Jump, vec![300.0, 450.0, 600.0]),
            (GameEvent::Hit, vec![180.0, 120.0]),
            (GameEvent::Win, vec![523.25, 659.25, 783.99, 1046.5]),
            (GameEvent::Lose, vec![400.0, 300.0, 200.0, 100.0]),
            (GameEvent::Menu, vec![660.0]),
        ] {
            let bytes = synthesize(&notes);
            sounds.push((event, load_sound_from_bytes(&bytes).await?));
        }
        Ok(Self { sounds, settings: AudioSettings::default() })
    }

    pub fn set_settings(&mut self, settings: AudioSettings) {
        if settings == self.settings {
            return;
        }
        self.settings = settings;
        for (_, sound) in &self.sounds {
            if settings.gain() == 0.0 {
                stop_sound(sound);
            } else {
                set_sound_volume(sound, settings.gain());
            }
        }
    }

    pub fn play(&self, event: GameEvent) {
        if self.settings.gain() == 0.0 {
            return;
        }
        let (_, sound) = self.sounds.iter().find(|(kind, _)| *kind == event)
            .expect("Every game event must have a sound");
        play_sound(sound, PlaySoundParams {
            looped: false,
            volume: self.settings.gain(),
        });
    }
}

fn synthesize(notes: &[f32]) -> Vec<u8> {
    const RATE: u32 = 22050;
    const SAMPLES_PER_NOTE: usize = 1543;
    let data_size = (notes.len() * SAMPLES_PER_NOTE * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for &frequency in notes {
        for sample in 0..SAMPLES_PER_NOTE {
            let progress = sample as f32 / SAMPLES_PER_NOTE as f32;
            let envelope = (progress * 20.0).min(1.0) * (1.0 - progress);
            let phase = std::f32::consts::TAU * frequency * sample as f32 / RATE as f32;
            let value = (phase.sin() * envelope * 8000.0) as i16;
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_clamp_volume_and_mute_without_losing_level() {
        let mut settings = AudioSettings::default();
        for _ in 0..20 { settings.adjust(true); }
        assert_eq!(settings.volume, 100);
        settings.muted = true;
        assert_eq!(settings.gain(), 0.0);
        settings.muted = false;
        assert_eq!(settings.gain(), 1.0);
        for _ in 0..20 { settings.adjust(false); }
        assert_eq!(settings.volume, 0);
    }

    #[test]
    fn generated_audio_is_non_silent_pcm_with_correct_lengths() {
        let bytes = synthesize(&[440.0, 880.0]);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(&bytes[36..40], b"data");
        let size = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
        assert_eq!(bytes.len(), 44 + size);
        assert!(bytes[44..].iter().any(|&byte| byte != 0));
    }
}
