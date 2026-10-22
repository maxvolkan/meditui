//! Sounds, all generated in code (no audio files): a singing bowl bell, soft
//! gliding tones when the breath turns, and background noise.

use std::f64::consts::{PI, TAU};
use std::num::NonZero;
use std::time::{Duration, Instant};

use rodio::{ChannelCount, DeviceSinkBuilder, MixerDeviceSink, Player, SampleRate, Source};

use crate::storage::Background;

const RATE: u32 = 44_100;
const MONO: ChannelCount = NonZero::new(1).unwrap();
const SAMPLE_RATE: SampleRate = NonZero::new(RATE).unwrap();

/// How long background sound takes to fade away when it stops.
const FADE_OUT: Duration = Duration::from_secs(4);
/// Overtones above this get softened, so bells stay mellow on headphones.
const MELLOW_HZ: f64 = 700.0;
/// Background sound sits well under the bells.
const BACKGROUND_LEVEL: f32 = 0.35;

pub struct Sound {
    device: MixerDeviceSink,
    background: Option<Player>,
    /// A background that's on its way out: the player, when the fade
    /// started, and how loud it was then.
    fading: Option<(Player, Instant, f32)>,
    background_volume: f32,
}

impl Sound {
    /// Opens the default audio device. `None` if there isn't one; the app
    /// then falls back to the terminal bell.
    pub fn open() -> Option<Self> {
        let mut device = DeviceSinkBuilder::from_default_device()
            .ok()?
            // The default handlers print to the terminal, which would scribble
            // over the app, so stay quiet.
            .with_error_callback(|_| {})
            .open_stream()
            .ok()?;
        device.log_on_drop(false);
        Some(Self {
            device,
            background: None,
            fading: None,
            background_volume: 0.0,
        })
    }

    fn play<S: Source + Send + 'static>(&self, source: S, volume: f32) {
        self.device.mixer().add(source.amplify(volume));
    }

    /// One low singing bowl strike, for the start of a session.
    pub fn bell(&self, volume: f32) {
        self.play(Bowl::new(196.0, 7.0), volume * 0.6);
    }

    /// One long, resonant strike for the end of a session.
    pub fn closing_bell(&self, volume: f32) {
        self.play(Bowl::new(196.0, 11.0), volume * 0.65);
    }

    /// A small high bowl, for when a guided cue changes.
    pub fn chime(&self, volume: f32) {
        self.play(Bowl::new(523.25, 3.0), volume * 0.35);
    }

    /// A soft tone that rises for an in-breath and falls for an out-breath.
    pub fn breath_tone(&self, rising: bool, volume: f32) {
        let (from, to) = if rising {
            (262.0, 349.0)
        } else {
            (349.0, 262.0)
        };
        self.play(Glide::new(from, to, 1.6), volume * 0.18);
    }

    pub fn start_background(&mut self, kind: Background, volume: f32) {
        self.stop_background();
        if kind == Background::Off {
            return;
        }
        let player = Player::connect_new(self.device.mixer());
        self.background_volume = volume * BACKGROUND_LEVEL;
        player.set_volume(self.background_volume);
        player.append(Noise::new(kind).fade_in(Duration::from_secs(3)));
        self.background = Some(player);
    }

    /// Fades the background out over a few seconds instead of cutting it.
    /// `tick` does the fading.
    pub fn stop_background(&mut self) {
        if let Some(player) = self.background.take() {
            if let Some((old, _, _)) = self.fading.take() {
                old.stop();
            }
            self.fading = Some((player, Instant::now(), self.background_volume));
        }
    }

    /// Call every frame: moves any fade-out along.
    pub fn tick(&mut self) {
        if let Some((player, started, from)) = &self.fading {
            let level = fade_level(started.elapsed());
            if level <= 0.0 {
                player.stop();
                self.fading = None;
            } else {
                player.set_volume(from * level);
            }
        }
    }

    pub fn pause_background(&self, paused: bool) {
        if let Some(player) = &self.background {
            if paused {
                player.pause();
            } else {
                player.play();
            }
        }
    }

    pub fn set_background_volume(&mut self, volume: f32) {
        if let Some(player) = &self.background {
            self.background_volume = volume * BACKGROUND_LEVEL;
            player.set_volume(self.background_volume);
        }
    }
}

/// Loudness during a fade-out, from 1.0 down to 0.0. Squared, so it drops
/// the way our ears expect rather than hanging on and then vanishing.
fn fade_level(elapsed: Duration) -> f32 {
    let left = 1.0 - (elapsed.as_secs_f32() / FADE_OUT.as_secs_f32()).min(1.0);
    left * left
}

/// How much of a partial at `freq` gets through a gentle low-pass filter:
/// about 1.0 for low tones, falling off above `MELLOW_HZ`.
fn mellow(freq: f64) -> f64 {
    1.0 / (1.0 + (freq / MELLOW_HZ).powi(2))
}

/// A struck singing bowl: a few slightly out-of-tune partials that ring and
/// slowly fade. The two close partials near the bottom make the "shimmer".
struct Bowl {
    sample: u64,
    length: u64,
    partials: [(f64, f64, f64); 5], // (frequency, loudness, seconds to fade)
}

impl Bowl {
    fn new(freq: f64, seconds: f64) -> Self {
        let d = seconds / 3.0;
        Self {
            sample: 0,
            length: (seconds * RATE as f64) as u64,
            partials: [
                (freq, 1.0, d),
                (freq * 1.006, 0.6, d),
                (freq * 2.76, 0.45, d * 0.6),
                (freq * 5.40, 0.22, d * 0.35),
                (freq * 8.93, 0.10, d * 0.2),
            ]
            .map(|(f, amp, fade)| (f, amp * mellow(f), fade)),
        }
    }
}

impl Iterator for Bowl {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.sample >= self.length {
            return None;
        }
        let t = self.sample as f64 / RATE as f64;
        self.sample += 1;
        let strike = (t / 0.008).min(1.0); // 8ms soft attack, no click
        let ring: f64 = self
            .partials
            .iter()
            .map(|&(freq, amp, fade)| amp * (TAU * freq * t).sin() * (-t / fade).exp())
            .sum();
        Some((ring * strike * 0.4) as rodio::Sample)
    }
}

/// A sine tone that glides from one pitch to another and swells in and out.
struct Glide {
    sample: u64,
    length: u64,
    from: f64,
    to: f64,
    phase: f64,
}

impl Glide {
    fn new(from: f64, to: f64, seconds: f64) -> Self {
        Self {
            sample: 0,
            length: (seconds * RATE as f64) as u64,
            from,
            to,
            phase: 0.0,
        }
    }
}

impl Iterator for Glide {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.sample >= self.length {
            return None;
        }
        let progress = self.sample as f64 / self.length as f64;
        self.sample += 1;
        let eased = (1.0 - (progress * PI).cos()) / 2.0;
        let freq = self.from + (self.to - self.from) * eased;
        self.phase = (self.phase + TAU * freq / RATE as f64) % TAU;
        let swell = (progress * PI).sin().powi(2);
        let tone = self.phase.sin() + 0.2 * (2.0 * self.phase).sin();
        Some((tone * swell * 0.5) as rodio::Sample)
    }
}

/// Endless background noise. Brown noise is a deep, steady rumble; the ocean
/// is the same noise rising and falling in slow, uneven swells.
struct Noise {
    kind: Background,
    sample: u64,
    seed: u64,
    brown: f64,
}

impl Noise {
    fn new(kind: Background) -> Self {
        Self {
            kind,
            sample: 0,
            seed: 0x9E37_79B9_7F4A_7C15,
            brown: 0.0,
        }
    }

    /// White noise in -1.0..1.0 (xorshift: fast, and random enough for sound).
    fn white(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 11) as f64 / (1u64 << 52) as f64 - 1.0
    }
}

impl Iterator for Noise {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let white = self.white();
        // Brown noise: white noise, smoothed by a "leaky" running total.
        self.brown = (self.brown + 0.02 * white) / 1.02;
        let brown = self.brown * 3.5;

        let value = match self.kind {
            Background::Ocean => {
                let t = self.sample as f64 / RATE as f64;
                self.sample += 1;
                // Two slow waves of different lengths, so swells never repeat exactly.
                let swell = 0.5 - 0.5 * (TAU * t / 9.0).cos();
                let tide = 0.5 - 0.5 * (TAU * t / 13.0).cos();
                let level = 0.15 + 0.85 * (0.7 * swell + 0.3 * tide).powi(2);
                brown * level + white * 0.04 * level * level
            }
            Background::BrownNoise | Background::Off => brown,
        };
        Some(value.clamp(-1.0, 1.0) as rodio::Sample)
    }
}

impl Source for Bowl {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        MONO
    }
    fn sample_rate(&self) -> SampleRate {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(self.length as f64 / RATE as f64))
    }
}

impl Source for Glide {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        MONO
    }
    fn sample_rate(&self) -> SampleRate {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(self.length as f64 / RATE as f64))
    }
}

impl Source for Noise {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        MONO
    }
    fn sample_rate(&self) -> SampleRate {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(samples: impl Iterator<Item = rodio::Sample>) -> f32 {
        samples.fold(0.0, |max, s| max.max(s.abs()))
    }

    #[test]
    fn bowl_rings_then_fades_without_clipping() {
        let samples: Vec<_> = Bowl::new(196.0, 11.0).collect();
        assert_eq!(samples.len(), 11 * RATE as usize);
        assert!(samples.iter().all(|s| s.is_finite()));
        let start = peak(samples[..RATE as usize].iter().copied());
        let end = peak(samples[samples.len() - RATE as usize..].iter().copied());
        assert!(start <= 1.0, "clips: {start}");
        assert!(start > 0.3, "too quiet: {start}");
        assert!(end < start * 0.1, "doesn't fade: {start} -> {end}");
    }

    #[test]
    fn high_overtones_are_softened() {
        assert!(mellow(196.0) > 0.9);
        assert!(mellow(700.0) == 0.5);
        assert!(mellow(2800.0) < 0.1);
    }

    #[test]
    fn fade_out_goes_from_full_to_silent() {
        assert_eq!(fade_level(Duration::ZERO), 1.0);
        let halfway = fade_level(FADE_OUT / 2);
        assert!(halfway > 0.2 && halfway < 0.3, "{halfway}");
        assert_eq!(fade_level(FADE_OUT), 0.0);
        assert_eq!(fade_level(FADE_OUT * 2), 0.0);
    }

    #[test]
    fn glide_starts_and_ends_silent() {
        let samples: Vec<_> = Glide::new(262.0, 349.0, 1.6).collect();
        assert!(samples[0].abs() < 1e-3);
        assert!(samples[samples.len() - 1].abs() < 1e-3);
        assert!(peak(samples.into_iter()) <= 1.0);
    }

    #[test]
    fn noise_is_endless_and_in_range() {
        for kind in [Background::BrownNoise, Background::Ocean] {
            let samples: Vec<_> = Noise::new(kind).take(RATE as usize * 20).collect();
            assert_eq!(samples.len(), RATE as usize * 20);
            assert!(samples.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
            assert!(peak(samples.into_iter()) > 0.05, "{kind:?} is silent");
        }
    }

    // Opens the real audio device (silently). Run with:
    //   cargo test -- --ignored
    #[test]
    #[ignore]
    fn can_open_the_audio_device() {
        assert!(Sound::open().is_some());
    }
}
