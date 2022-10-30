use std::collections::HashMap;
use std::time::Duration;
use minifb::{Scale, Window, WindowOptions};

const WINDOW_WIDTH: usize = 160;
const WINDOW_HEIGHT: usize = 144;

#[derive(Clone, Debug, PartialEq, Eq, Hash)] // TODO: Review this
pub enum GameBoyKey {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start
}

pub struct GUI {
    pub window: Window,
    pub vram: Vec<u32>,
    pub keys_states: HashMap<GameBoyKey, bool>
}

impl GUI {
    pub fn new() -> Self {
        let window_options = WindowOptions {
            resize: false,
            scale: Scale::X4,
            ..WindowOptions::default()
        };

        let mut window =
            match Window::new(
                "Gameboy emulator (github.com/oscarcpozas/gb_emu)",
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                window_options
            ) {
                Ok(win) => win,
                Err(err) => panic!("Error creating window {}", err)
            };

        GUI {
            window,
            keys_states: GUI::new_key_states(),
            vram: vec![0; WINDOW_WIDTH * WINDOW_HEIGHT]
        }
    }

    pub fn run(mut self) {
        while self.window.is_open() && !self.window.is_key_down(minifb::Key::Escape) {
            std::thread::sleep(Duration::from_millis(20));
            self.update_vram();
            self.get_key_update();
        }
    }

    fn new_key_states() -> HashMap<GameBoyKey, bool> {
        let mut keys_states = HashMap::new();
        keys_states.insert(GameBoyKey::Right, false);
        keys_states.insert(GameBoyKey::Left, false);
        keys_states.insert(GameBoyKey::Up, false);
        keys_states.insert(GameBoyKey::Down, false);
        keys_states.insert(GameBoyKey::A, false);
        keys_states.insert(GameBoyKey::B, false);
        keys_states.insert(GameBoyKey::Select, false);
        keys_states.insert(GameBoyKey::Start, false);
        keys_states
    }

    fn update_vram(&mut self) {
        let vram = self.vram.clone();
        self.window
            .update_with_buffer(&vram, WINDOW_WIDTH, WINDOW_HEIGHT)
            .unwrap();
    }

    fn get_key_update(&mut self) {
        for key in self.window.get_keys() {
            let gb_key = match key {
                minifb::Key::Right => GameBoyKey::Right,
                minifb::Key::Left => GameBoyKey::Left,
                minifb::Key::Up => GameBoyKey::Up,
                minifb::Key::Down => GameBoyKey::Down,
                minifb::Key::Z => GameBoyKey::A,
                minifb::Key::X => GameBoyKey::B,
                minifb::Key::Space => GameBoyKey::Select,
                minifb::Key::Enter => GameBoyKey::Start,
                minifb::Key::Escape => {
                    return;
                }
                _ => continue,
            };

            match self.keys_states.get_mut(&gb_key) {
                Some(value) => *value = true,
                None => unreachable!(),
            }
        }
    }
}