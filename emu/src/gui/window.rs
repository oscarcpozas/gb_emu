use font8x8::legacy::BASIC_LEGACY;
use softbuffer::{Context, Surface};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

const WINDOW_WIDTH: usize = 160;
const WINDOW_HEIGHT: usize = 144;
const WINDOW_TITLE: &str = "Gameboy emulator (github.com/oscarcpozas/gb_emu)";
const SCALE: usize = 4;

// DMG palette (0x00RRGGBB — softbuffer ignores the upper byte)
const COLOR_LIGHTEST: u32 = 0x009BBC0F;
const COLOR_DARKEST: u32 = 0x000F380F;
const COLOR_DARK: u32 = 0x00306230;

// ── Public key enum (used by joypad and hardware) ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameBoyKey {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

// ── Public GUI handle ─────────────────────────────────────────────────────────

/// Created on the main thread. The shared Arcs are cloned into `Hardware` so
/// the emulator thread can read/write them concurrently.
pub struct GUI {
    pub escape: Arc<AtomicBool>,
    pub muted: Arc<AtomicBool>,
    pub vram: Arc<Mutex<Vec<u32>>>,
    pub keys_states: Arc<Mutex<HashMap<GameBoyKey, bool>>>,
    /// Set by the app when the user drops or picks a ROM file.
    pub dropped_file: Arc<Mutex<Option<PathBuf>>>,
    event_loop: EventLoop<()>,
}

impl GUI {
    pub fn new() -> Self {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        GUI {
            escape: Arc::new(AtomicBool::new(false)),
            muted: Arc::new(AtomicBool::new(false)),
            vram: Arc::new(Mutex::new(vec![0u32; WINDOW_WIDTH * WINDOW_HEIGHT])),
            keys_states: Arc::new(Mutex::new(Self::init_key_states())),
            dropped_file: Arc::new(Mutex::new(None)),
            event_loop,
        }
    }

    /// Runs the winit event loop (blocks until the window is closed).
    ///
    /// `rom_loaded = true`  → skip splash, go straight to the game loop.
    /// `rom_loaded = false` → show splash; the app will set `dropped_file` and
    ///                        switch to game mode when a ROM is provided.
    pub fn run(self, rom_loaded: bool) {
        let mut app = GbApp {
            escape: self.escape,
            muted: self.muted,
            vram: self.vram,
            keys_states: self.keys_states,
            dropped_file: self.dropped_file,
            rom_loaded,
            window: None,
            context: None,
            surface: None,
            splash_buffer: build_splash_buffer(),
        };
        self.event_loop
            .run_app(&mut app)
            .expect("Event loop error");
    }

    fn init_key_states() -> HashMap<GameBoyKey, bool> {
        [
            GameBoyKey::Right,
            GameBoyKey::Left,
            GameBoyKey::Up,
            GameBoyKey::Down,
            GameBoyKey::A,
            GameBoyKey::B,
            GameBoyKey::Select,
            GameBoyKey::Start,
        ]
        .into_iter()
        .map(|k| (k, false))
        .collect()
    }
}

// ── winit ApplicationHandler ──────────────────────────────────────────────────

type SbContext = Context<Arc<Window>>;
type SbSurface = Surface<Arc<Window>, Arc<Window>>;

struct GbApp {
    // Shared with the emulator thread
    escape: Arc<AtomicBool>,
    muted: Arc<AtomicBool>,
    vram: Arc<Mutex<Vec<u32>>>,
    keys_states: Arc<Mutex<HashMap<GameBoyKey, bool>>>,
    dropped_file: Arc<Mutex<Option<PathBuf>>>,

    rom_loaded: bool,

    // Window / rendering (created in `resumed`)
    window: Option<Arc<Window>>,
    context: Option<SbContext>,
    surface: Option<SbSurface>,
    splash_buffer: Vec<u32>,
}

impl ApplicationHandler for GbApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // already initialised (e.g. after suspend/resume on mobile)
        }

        let attrs = WindowAttributes::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(
                (WINDOW_WIDTH * SCALE) as f64,
                (WINDOW_HEIGHT * SCALE) as f64,
            ))
            .with_resizable(false);

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        // softbuffer: Context owns the display, Surface owns the window.
        let context = Context::new(window.clone()).expect("softbuffer Context");
        self.context = Some(context);
        let surface = Surface::new(self.context.as_ref().unwrap(), window.clone())
            .expect("softbuffer Surface");

        self.window = Some(window);
        self.surface = Some(surface);

        // Kick off the first draw.
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.escape.store(true, Ordering::Relaxed);
                event_loop.exit();
            }

            // ── Drag-and-drop ─────────────────────────────────────────────────
            WindowEvent::DroppedFile(path) => {
                if !self.rom_loaded {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if matches!(ext.as_str(), "gb" | "gbc" | "gba") {
                        *self.dropped_file.lock().unwrap() = Some(path);
                        self.rom_loaded = true;
                    }
                }
            }

            // ── Keyboard ──────────────────────────────────────────────────────
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    self.handle_key(key_code, event.state, event.repeat, event_loop);
                }
            }

            // ── Render ────────────────────────────────────────────────────────
            WindowEvent::RedrawRequested => {
                self.render();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.escape.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }

        if self.rom_loaded {
            // Game running: poll → request redraws as fast as the OS allows.
            event_loop.set_control_flow(ControlFlow::Poll);
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        } else {
            // Splash is static; only redraw on events (saves CPU/battery).
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

impl GbApp {
    fn handle_key(
        &mut self,
        key_code: KeyCode,
        state: ElementState,
        repeat: bool,
        event_loop: &ActiveEventLoop,
    ) {
        // Escape always exits.
        if key_code == KeyCode::Escape && state == ElementState::Pressed {
            self.escape.store(true, Ordering::Relaxed);
            event_loop.exit();
            return;
        }

        // While on the splash, Enter / Space opens the native file picker.
        if !self.rom_loaded && state == ElementState::Pressed && !repeat {
            if matches!(key_code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Game Boy ROM", &["gb", "gbc", "gba"])
                    .set_title("Open Game Boy ROM")
                    .pick_file()
                {
                    *self.dropped_file.lock().unwrap() = Some(path);
                    self.rom_loaded = true;
                }
                return;
            }
        }

        // Mute toggle (M key, no repeat).
        if key_code == KeyCode::KeyM && state == ElementState::Pressed && !repeat {
            let prev = self.muted.load(Ordering::Relaxed);
            self.muted.store(!prev, Ordering::Relaxed);
        }

        // Game Boy buttons — active in both splash and game mode so input is
        // always registered, but joypad only matters when the emulator runs.
        let gb_key = match key_code {
            KeyCode::ArrowRight => Some(GameBoyKey::Right),
            KeyCode::ArrowLeft => Some(GameBoyKey::Left),
            KeyCode::ArrowUp => Some(GameBoyKey::Up),
            KeyCode::ArrowDown => Some(GameBoyKey::Down),
            KeyCode::KeyZ => Some(GameBoyKey::A),
            KeyCode::KeyX => Some(GameBoyKey::B),
            KeyCode::Space => Some(GameBoyKey::Select),
            KeyCode::Enter | KeyCode::NumpadEnter => Some(GameBoyKey::Start),
            _ => None,
        };

        if let Some(key) = gb_key {
            let pressed = state == ElementState::Pressed;
            if let Some(v) = self.keys_states.lock().unwrap().get_mut(&key) {
                *v = pressed;
            }
        }
    }

    fn render(&mut self) {
        let size = match &self.window {
            Some(w) => w.inner_size(),
            None => return,
        };

        let (dw, dh) = (size.width as usize, size.height as usize);
        if dw == 0 || dh == 0 {
            return;
        }

        let surface = match &mut self.surface {
            Some(s) => s,
            None => return,
        };

        surface
            .resize(
                NonZeroU32::new(dw as u32).unwrap(),
                NonZeroU32::new(dh as u32).unwrap(),
            )
            .expect("softbuffer resize");

        let mut screen = surface.buffer_mut().expect("softbuffer buffer_mut");

        if self.rom_loaded {
            let vram = self.vram.lock().unwrap();
            blit_scaled(&vram, WINDOW_WIDTH, WINDOW_HEIGHT, &mut screen, dw, dh);
        } else {
            blit_scaled(
                &self.splash_buffer,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                &mut screen,
                dw,
                dh,
            );
        }

        screen.present().expect("softbuffer present");
    }
}

// ── Rendering helpers ─────────────────────────────────────────────────────────

/// Nearest-neighbour scale from `src` (sw×sh) into `dst` (dw×dh).
/// The upper byte of each pixel is zeroed because softbuffer requires 0x00RRGGBB.
fn blit_scaled(src: &[u32], sw: usize, sh: usize, dst: &mut [u32], dw: usize, dh: usize) {
    for dy in 0..dh {
        let sy = dy * sh / dh;
        let src_row = &src[sy * sw..sy * sw + sw];
        let dst_row = &mut dst[dy * dw..dy * dw + dw];
        for dx in 0..dw {
            let sx = dx * sw / dw;
            dst_row[dx] = src_row[sx] & 0x00FF_FFFF;
        }
    }
}

// ── Splash screen ─────────────────────────────────────────────────────────────

fn build_splash_buffer() -> Vec<u32> {
    let mut buf = vec![COLOR_LIGHTEST; WINDOW_WIDTH * WINDOW_HEIGHT];

    draw_border(&mut buf);

    // Title
    // draw_text_centered(&mut buf, "GAME BOY", 30);
    // draw_text_centered(&mut buf, "EMULATOR", 42);
    //
    // // Separator below title
    // let sep_y = 58usize;
    // let sep_len = "GAME BOY".len() * 8; // 64 px
    // let sep_x = (WINDOW_WIDTH - sep_len) / 2;
    // draw_hline(&mut buf, sep_x, sep_x + sep_len - 1, sep_y, COLOR_DARK);

    // Instructions
    draw_text_centered(&mut buf, "DRAG  &  DROP", 76);
    draw_text_centered(&mut buf, "OR  PRESS  ENTER", 88);
    draw_text_centered(&mut buf, "TO  OPEN  ROM", 100);

    buf
}

fn draw_text_centered(buf: &mut Vec<u32>, text: &str, y: usize) {
    let w = text.len() * 8;
    let x = if w < WINDOW_WIDTH { (WINDOW_WIDTH - w) / 2 } else { 0 };
    draw_text(buf, text, x, y, COLOR_DARKEST);
}

fn draw_text(buf: &mut Vec<u32>, text: &str, x: usize, y: usize, color: u32) {
    for (i, ch) in text.chars().enumerate() {
        let code = ch as usize;
        if code >= 128 {
            continue;
        }
        let glyph = &BASIC_LEGACY[code];
        for (row, &byte) in glyph.iter().enumerate() {
            for col in 0..8usize {
                if byte & (1 << col) != 0 {
                    let px = x + i * 8 + col;
                    let py = y + row;
                    if px < WINDOW_WIDTH && py < WINDOW_HEIGHT {
                        buf[py * WINDOW_WIDTH + px] = color;
                    }
                }
            }
        }
    }
}

fn draw_hline(buf: &mut Vec<u32>, x0: usize, x1: usize, y: usize, color: u32) {
    for x in x0..=x1 {
        if x < WINDOW_WIDTH && y < WINDOW_HEIGHT {
            buf[y * WINDOW_WIDTH + x] = color;
        }
    }
}

fn draw_border(buf: &mut Vec<u32>) {
    let m = 4usize;
    for x in m..(WINDOW_WIDTH - m) {
        buf[m * WINDOW_WIDTH + x] = COLOR_DARK;
        buf[(WINDOW_HEIGHT - m - 1) * WINDOW_WIDTH + x] = COLOR_DARK;
    }
    for y in m..(WINDOW_HEIGHT - m) {
        buf[y * WINDOW_WIDTH + m] = COLOR_DARK;
        buf[y * WINDOW_WIDTH + (WINDOW_WIDTH - m - 1)] = COLOR_DARK;
    }
}
