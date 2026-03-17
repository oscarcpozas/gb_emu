mod audio_output;
pub mod channel1;
pub mod channel2;
pub mod channel3;
pub mod channel4;

use audio_output::AudioOutput;
use channel1::Channel1;
use channel2::Channel2;
use channel3::Channel3;
use channel4::Channel4;
use ringbuf::traits::Producer as _;

use crate::mmu::{MemHandler, MemRead, MemWrite};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Game Boy CPU clock speed in Hz.
const CPU_FREQ: f64 = 4_194_304.0;
/// Target audio sample rate in Hz.
const SAMPLE_RATE: f64 = 44_100.0;

/// Frame sequencer period in T-cycles (CPU_FREQ / 512 Hz * 4 T/M).
/// 4_194_304 / 512 = 8192 M-cycles = 32768 T-cycles.
const FRAME_SEQ_PERIOD: u32 = 32_768;

/// Samples per T-cycle (used for accumulator).
/// One sample every (CPU_FREQ * 4) / SAMPLE_RATE T-cycles ≈ 380.4.
const T_CYCLES_PER_SAMPLE: f64 = (CPU_FREQ * 4.0) / SAMPLE_RATE;

pub struct Apu {
    ch1: Channel1,
    ch2: Channel2,
    ch3: Channel3,
    ch4: Channel4,

    // NR50 (0xFF24): L/R master volume
    nr50: u8,
    // NR51 (0xFF25): channel panning
    nr51: u8,
    // NR52 (0xFF26): master enable
    master_enable: bool,

    // Frame sequencer
    frame_seq_timer: u32,
    frame_seq_step: u8,

    // Sample accumulator (T-cycles elapsed since last sample push)
    sample_timer: f64,

    // High-pass filter capacitor state (tracks DC offset to cancel it)
    hp_cap: f32,

    // Mute flag shared with the GUI (toggled by pressing M)
    muted: Arc<AtomicBool>,

    // Audio output (None if no audio device available)
    audio: Option<AudioOutput>,
}

impl Apu {
    pub fn new(muted: Arc<AtomicBool>) -> Self {
        let audio = AudioOutput::new();
        if audio.is_none() {
            log::warn!("APU: no audio output device found — running silently");
        }
        Self {
            ch1: Channel1::new(),
            ch2: Channel2::new(),
            ch3: Channel3::new(),
            ch4: Channel4::new(),
            nr50: 0x77,
            nr51: 0xF3,
            master_enable: true,
            frame_seq_timer: 0,
            frame_seq_step: 0,
            sample_timer: 0.0,
            hp_cap: 0.0,
            muted,
            audio,
        }
    }

    /// Advance the APU by `cycles` M-cycles (as returned by `Cpu::fetch_n_execute`).
    pub fn update(&mut self, cycles: usize) {
        if !self.master_enable {
            return;
        }

        let t_cycles = (cycles as u32) * 4;

        // Tick frame sequencer (fires at 512 Hz = every 32768 T-cycles)
        self.frame_seq_timer += t_cycles;
        while self.frame_seq_timer >= FRAME_SEQ_PERIOD {
            self.frame_seq_timer -= FRAME_SEQ_PERIOD;
            self.clock_frame_sequencer();
        }

        // Tick all channels and get their current output (0–15)
        let s1 = self.ch1.tick(t_cycles) as f32;
        let s2 = self.ch2.tick(t_cycles) as f32;
        let s3 = self.ch3.tick(t_cycles) as f32;
        let s4 = self.ch4.tick(t_cycles) as f32;

        // Generate samples at the target sample rate
        self.sample_timer += t_cycles as f64;
        while self.sample_timer >= T_CYCLES_PER_SAMPLE {
            self.sample_timer -= T_CYCLES_PER_SAMPLE;

            // Mix channels (each 0–15), normalise to 0..1
            let raw = (s1 + s2 + s3 + s4) / 60.0;

            // High-pass filter: removes DC offset (same as the GB hardware capacitor).
            // Without this, disabled channels produce constant -1.0 → loud buzz.
            // Charge factor 0.999 → cutoff ~14 Hz, passes all audio frequencies.
            let sample = raw - self.hp_cap;
            self.hp_cap = self.hp_cap * 0.999 + raw * 0.001;

            if let Some(ref mut audio) = self.audio {
                let out = if self.muted.load(Ordering::Relaxed) { 0.0 } else { sample };
                let _ = audio.producer.try_push(out);
            }
        }
    }

    fn clock_frame_sequencer(&mut self) {
        match self.frame_seq_step {
            0 | 4 => {
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch3.clock_length();
                self.ch4.clock_length();
            }
            2 | 6 => {
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch3.clock_length();
                self.ch4.clock_length();
                self.ch1.clock_sweep();
            }
            7 => {
                self.ch1.clock_envelope();
                self.ch2.clock_envelope();
                self.ch4.clock_envelope();
            }
            _ => {}
        }
        self.frame_seq_step = (self.frame_seq_step + 1) & 7;
    }
}

impl MemHandler for Apu {
    fn on_read(&self, addr: u16) -> MemRead {
        // Wave RAM is always readable regardless of master enable
        if (0xFF30..=0xFF3F).contains(&addr) {
            return MemRead::Replace(self.ch3.read_reg(addr));
        }

        if !self.master_enable {
            return match addr {
                0xFF26 => MemRead::Replace(0x70), // master off, channels off
                0xFF10..=0xFF3F => MemRead::Replace(0xFF),
                _ => MemRead::PassThrough,
            };
        }

        let val = match addr {
            0xFF10..=0xFF14 => self.ch1.read_reg(addr),
            0xFF15 => 0xFF,
            0xFF16..=0xFF19 => self.ch2.read_reg(addr),
            0xFF1A..=0xFF1E => self.ch3.read_reg(addr),
            0xFF1F => 0xFF,
            0xFF20..=0xFF23 => self.ch4.read_reg(addr),
            0xFF24 => self.nr50,
            0xFF25 => self.nr51,
            0xFF26 => {
                let mut v: u8 = 0x70; // bits 4-6 always 1
                v |= (self.master_enable as u8) << 7;
                v |= self.ch1.enabled as u8;
                v |= (self.ch2.enabled as u8) << 1;
                v |= (self.ch3.enabled as u8) << 2;
                v |= (self.ch4.enabled as u8) << 3;
                v
            }
            _ => 0xFF,
        };

        MemRead::Replace(val)
    }

    fn on_write(&mut self, addr: u16, val: u8) -> MemWrite {
        // Wave RAM writes always go through regardless of master enable
        if (0xFF30..=0xFF3F).contains(&addr) {
            self.ch3.write_reg(addr, val);
            return MemWrite::Block;
        }

        // When master is off, only NR52 writes are allowed
        if !self.master_enable {
            if addr == 0xFF26 {
                self.master_enable = (val & 0x80) != 0;
            }
            return MemWrite::Block;
        }

        match addr {
            0xFF10..=0xFF14 => self.ch1.write_reg(addr, val),
            0xFF16..=0xFF19 => self.ch2.write_reg(addr, val),
            0xFF1A..=0xFF1E => self.ch3.write_reg(addr, val),
            0xFF20..=0xFF23 => self.ch4.write_reg(addr, val),
            0xFF24 => self.nr50 = val,
            0xFF25 => self.nr51 = val,
            0xFF26 => {
                let was_enabled = self.master_enable;
                self.master_enable = (val & 0x80) != 0;
                if was_enabled && !self.master_enable {
                    // Power off: reset all channels and registers
                    self.ch1 = Channel1::new();
                    self.ch2 = Channel2::new();
                    self.ch3 = Channel3::new();
                    self.ch4 = Channel4::new();
                    self.nr50 = 0;
                    self.nr51 = 0;
                }
            }
            _ => {}
        }

        MemWrite::Block
    }
}
