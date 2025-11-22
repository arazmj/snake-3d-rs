use web_sys::{AudioContext, OscillatorType};
use wasm_bindgen::JsValue;

pub struct AudioPlayer {
    context: Option<AudioContext>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let context = AudioContext::new().ok();
        Self { context }
    }

    pub fn resume_context(&self) {
        if let Some(ctx) = &self.context {
            if ctx.state() == web_sys::AudioContextState::Suspended {
                let _ = ctx.resume();
            }
        }
    }

    pub fn play_sound(&self, freq: f32, duration: f64) {
        if let Some(ctx) = &self.context {
            // Create oscillator and gain node
            let oscillator = match ctx.create_oscillator() {
                Ok(o) => o,
                Err(_) => return,
            };
            let gain_node = match ctx.create_gain() {
                Ok(g) => g,
                Err(_) => return,
            };

            // Connect oscillator -> gain -> destination
            let _ = oscillator.connect_with_audio_node(&gain_node);
            let _ = gain_node.connect_with_audio_node(&ctx.destination());

            // Set frequency
            oscillator.frequency().set_value(freq);
            oscillator.set_type(OscillatorType::Sine);

            // Volume envelope
            let now = ctx.current_time();
            let _ = gain_node.gain().set_value_at_time(0.1, now);
            let _ = gain_node.gain().exponential_ramp_to_value_at_time(0.001, now + duration);

            // Start and stop
            let _ = oscillator.start_with_when(now);
            let _ = oscillator.stop_with_when(now + duration);
        }
    }

    pub fn play_eat(&self) {
        self.play_sound(600.0, 0.1);
        // Small delay for a "coin" sound effect?
        // Since we can't easily delay without closures/futures in this simple struct,
        // we'll just play one tone or multiple at once.
        // self.play_sound(800.0, 0.1); // Playing immediately just overlays them.
    }

    pub fn play_prize(&self) {
        if let Some(ctx) = &self.context {
            let now = ctx.current_time();
            self.play_tone(ctx, 600.0, now, 0.1);
            self.play_tone(ctx, 900.0, now + 0.1, 0.1);
            self.play_tone(ctx, 1200.0, now + 0.2, 0.2);
        }
    }

    pub fn play_game_over(&self) {
        if let Some(ctx) = &self.context {
            let now = ctx.current_time();
            self.play_tone(ctx, 300.0, now, 0.2);
            self.play_tone(ctx, 200.0, now + 0.2, 0.2);
            self.play_tone(ctx, 100.0, now + 0.4, 0.4);
        }
    }

    fn play_tone(&self, ctx: &AudioContext, freq: f32, start_time: f64, duration: f64) {
         let oscillator = match ctx.create_oscillator() {
                Ok(o) => o,
                Err(_) => return,
            };
            let gain_node = match ctx.create_gain() {
                Ok(g) => g,
                Err(_) => return,
            };

            let _ = oscillator.connect_with_audio_node(&gain_node);
            let _ = gain_node.connect_with_audio_node(&ctx.destination());

            oscillator.frequency().set_value(freq);
            oscillator.set_type(OscillatorType::Square);

            let _ = gain_node.gain().set_value_at_time(0.1, start_time);
            let _ = gain_node.gain().exponential_ramp_to_value_at_time(0.001, start_time + duration);

            let _ = oscillator.start_with_when(start_time);
            let _ = oscillator.stop_with_when(start_time + duration);
    }
}
