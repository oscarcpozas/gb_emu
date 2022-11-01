use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use minifb::{Scale, Window, WindowOptions};

const WINDOW_WIDTH: usize = 160;
const WINDOW_HEIGHT: usize = 144;
const WINDOW_TITLE: &str = "Gameboy emulator (github.com/oscarcpozas/gb_emu)";

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
    pub vram: Arc<Mutex<Vec<u32>>>,
    pub keys_states: Arc<Mutex<HashMap<GameBoyKey, bool>>>,
}

impl GUI {
    pub fn new() -> Self {
        GUI {
            keys_states: Arc::new(Mutex::new(GUI::new_key_states())),
            vram: Arc::new(Mutex::new(vec![0; WINDOW_WIDTH * WINDOW_HEIGHT]))
        }
    }

    pub fn run(mut self) {
        let window_options = WindowOptions {
            resize: false,
            scale: Scale::X4,
            ..WindowOptions::default()
        };
        let mut window =
            match Window::new(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT, window_options) {
                Ok(win) => win,
                Err(err) => panic!("Error creating window {}", err)
            };


        while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
            std::thread::sleep(Duration::from_millis(10));
            self.update_vram(&mut window);
            self.get_key_update(&window);
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

    fn update_vram(&mut self, window: &mut Window) {
        let vram = self.vram.lock().unwrap().clone();
        window.update_with_buffer(&vram, WINDOW_WIDTH, WINDOW_HEIGHT).unwrap();
    }

    fn get_key_update(&mut self, window: &Window) {
        for key in window.get_keys() {
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

            match self.keys_states.lock().unwrap().get_mut(&gb_key) {
                Some(value) => *value = true,
                None => unreachable!(),
            }
        }
    }
}