use crate::gui::window::{GameBoyKey, GUI};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Hardware {
    escape: Arc<AtomicBool>,
    muted: Arc<AtomicBool>,
    vram: Arc<Mutex<Vec<u32>>>,
    keys_states: Arc<Mutex<HashMap<GameBoyKey, bool>>>,
}

impl Hardware {
    pub fn new(gui: &GUI) -> Self {
        Self {
            escape: gui.escape.clone(),
            muted: gui.muted.clone(),
            vram: gui.vram.clone(),
            keys_states: gui.keys_states.clone(),
        }
    }

    pub fn get_gui_is_alive(&self) -> bool {
        !self.escape.load(Ordering::Relaxed)
    }

    pub fn get_vram(&self) -> Arc<Mutex<Vec<u32>>> {
        self.vram.clone()
    }

    pub fn get_keys_states(&self) -> Arc<Mutex<HashMap<GameBoyKey, bool>>> {
        self.keys_states.clone()
    }

    pub fn get_muted(&self) -> Arc<AtomicBool> {
        self.muted.clone()
    }
}
