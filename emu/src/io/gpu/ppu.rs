use crate::io::interrupt::{INT_LCD_STAT, INT_VBLANK};
use crate::mmu::{MemHandler, MemRead, MemWrite};
use std::sync::{Arc, Mutex};

// LCD Control Register (0xFF40)
const LCDC_BG_ENABLE: u8 = 0x01;
const LCDC_OBJ_ENABLE: u8 = 0x02;
const LCDC_OBJ_SIZE: u8 = 0x04;
const LCDC_BG_MAP: u8 = 0x08;
const LCDC_TILE_DATA: u8 = 0x10;
const LCDC_WINDOW_ENABLE: u8 = 0x20;
const LCDC_WINDOW_MAP: u8 = 0x40;
const LCDC_DISPLAY_ENABLE: u8 = 0x80;

// LCD Status Register (0xFF41)
const STAT_MODE: u8 = 0x03;
const STAT_LYC_EQUAL: u8 = 0x04;
const STAT_HBLANK_INT: u8 = 0x08;
const STAT_VBLANK_INT: u8 = 0x10;
const STAT_OAM_INT: u8 = 0x20;
const STAT_LYC_INT: u8 = 0x40;

// LCD Modes
const MODE_HBLANK: u8 = 0x00;
const MODE_VBLANK: u8 = 0x01;
const MODE_OAM: u8 = 0x02;
const MODE_TRANSFER: u8 = 0x03;

// Screen dimensions
const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

// Game Boy color palette (4 shades of green)
const COLOR_WHITE: u32 = 0xFFFFFFFF;
const COLOR_LIGHT_GREEN: u32 = 0xFFADD794;
const COLOR_DARK_GREEN: u32 = 0xFF306230;
const COLOR_BLACK: u32 = 0xFF0F380F;

/// Picture Processing Unit (PPU)
/// Responsible for rendering graphics to the screen
pub struct Ppu {
    /// Video RAM (8KB)
    vram: Vec<u8>,
    /// Object Attribute Memory (OAM) - for sprites
    oam: Vec<u8>,
    /// LCD Control Register (0xFF40)
    lcdc: u8,
    /// LCD Status Register (0xFF41)
    stat: u8,
    /// Scroll Y (0xFF42)
    scy: u8,
    /// Scroll X (0xFF43)
    scx: u8,
    /// LCD Y Coordinate (0xFF44) - current scanline
    ly: u8,
    /// LY Compare (0xFF45) - scanline to compare for STAT interrupt
    lyc: u8,
    /// Window Y Position (0xFF4A)
    wy: u8,
    /// Window X Position minus 7 (0xFF4B)
    wx: u8,
    /// Background Palette (0xFF47)
    bgp: u8,
    /// Object Palette 0 (0xFF48)
    obp0: u8,
    /// Object Palette 1 (0xFF49)
    obp1: u8,
    /// Current PPU mode
    mode: u8,
    /// Cycles until next mode
    mode_cycles: usize,
    /// Internal window line counter (independent of LY)
    window_line: u8,
    /// Pending OAM DMA source page (written to 0xFF46)
    pub pending_dma: Option<u8>,
    /// Frame buffer - pixel data to be displayed
    frame_buffer: Arc<Mutex<Vec<u32>>>,
}

impl Ppu {
    pub fn new(frame_buffer: Arc<Mutex<Vec<u32>>>) -> Self {
        Self {
            vram: vec![0; 0x2000],
            oam: vec![0; 0xA0],
            lcdc: 0x91,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            wy: 0,
            wx: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            mode: MODE_OAM,
            mode_cycles: 0,
            window_line: 0,
            pending_dma: None,
            frame_buffer,
        }
    }

    /// Update the PPU state.
    /// Returns a bitmask of interrupts to be requested (INT_VBLANK, INT_LCD_STAT).
    pub fn update(&mut self, cycles: usize) -> u8 {
        let mut interrupts: u8 = 0;

        // If LCD is disabled, reset and return
        if self.lcdc & LCDC_DISPLAY_ENABLE == 0 {
            self.mode = MODE_HBLANK;
            self.ly = 0;
            self.mode_cycles = 0;
            self.window_line = 0;
            return 0;
        }

        self.mode_cycles += cycles;

        match self.mode {
            MODE_OAM => {
                if self.mode_cycles >= 80 {
                    self.mode_cycles -= 80;
                    self.mode = MODE_TRANSFER;
                    self.stat = (self.stat & 0xFC) | MODE_TRANSFER;
                }
            }
            MODE_TRANSFER => {
                if self.mode_cycles >= 172 {
                    self.mode_cycles -= 172;
                    self.mode = MODE_HBLANK;
                    self.stat = (self.stat & 0xFC) | MODE_HBLANK;
                    self.render_scanline();

                    // STAT interrupt on HBlank
                    if self.stat & STAT_HBLANK_INT != 0 {
                        interrupts |= INT_LCD_STAT;
                    }
                }
            }
            MODE_HBLANK => {
                if self.mode_cycles >= 204 {
                    self.mode_cycles -= 204;
                    self.ly += 1;

                    if self.ly == 144 {
                        self.mode = MODE_VBLANK;
                        self.stat = (self.stat & 0xFC) | MODE_VBLANK;
                        // VBlank interrupt
                        interrupts |= INT_VBLANK;
                        // STAT interrupt on VBlank
                        if self.stat & STAT_VBLANK_INT != 0 {
                            interrupts |= INT_LCD_STAT;
                        }
                    } else {
                        self.mode = MODE_OAM;
                        self.stat = (self.stat & 0xFC) | MODE_OAM;
                        // STAT interrupt on OAM
                        if self.stat & STAT_OAM_INT != 0 {
                            interrupts |= INT_LCD_STAT;
                        }
                    }
                }
            }
            MODE_VBLANK => {
                if self.mode_cycles >= 456 {
                    self.mode_cycles -= 456;
                    self.ly += 1;

                    if self.ly > 153 {
                        self.ly = 0;
                        self.window_line = 0;
                        self.mode = MODE_OAM;
                        self.stat = (self.stat & 0xFC) | MODE_OAM;
                        // STAT interrupt on OAM
                        if self.stat & STAT_OAM_INT != 0 {
                            interrupts |= INT_LCD_STAT;
                        }
                    }
                }
            }
            _ => unreachable!("Invalid PPU mode"),
        }

        // Update LYC=LY flag and trigger STAT interrupt if enabled
        if self.ly == self.lyc {
            if self.stat & STAT_LYC_EQUAL == 0 {
                // LY just matched LYC
                self.stat |= STAT_LYC_EQUAL;
                if self.stat & STAT_LYC_INT != 0 {
                    interrupts |= INT_LCD_STAT;
                }
            }
        } else {
            self.stat &= !STAT_LYC_EQUAL;
        }

        interrupts
    }

    /// Render a single scanline into the shared frame buffer.
    fn render_scanline(&mut self) {
        if self.ly >= SCREEN_HEIGHT as u8 {
            return;
        }

        // Per-pixel working data for this scanline.
        // color_index: palette color (0-3) for the final pixel.
        // bg_opaque: true when BG/Window pixel is color 1-3 (used for sprite priority).
        let mut color_index = [0u8; SCREEN_WIDTH];
        let mut bg_opaque = [false; SCREEN_WIDTH];

        if self.lcdc & LCDC_BG_ENABLE != 0 {
            self.render_background(&mut color_index, &mut bg_opaque);
        }

        let window_drawn = if self.lcdc & LCDC_WINDOW_ENABLE != 0 && self.wy <= self.ly {
            self.render_window(&mut color_index, &mut bg_opaque)
        } else {
            false
        };

        if self.lcdc & LCDC_OBJ_ENABLE != 0 {
            self.render_sprites(&mut color_index, &bg_opaque);
        }

        // Advance the window internal line counter when a window line was drawn.
        if window_drawn {
            self.window_line += 1;
        }

        // Write final scanline to the frame buffer.
        let mut frame_buffer = self.frame_buffer.lock().unwrap();
        let base = self.ly as usize * SCREEN_WIDTH;
        for x in 0..SCREEN_WIDTH {
            frame_buffer[base + x] = Self::dmg_color(color_index[x]);
        }
    }

    /// Map a 2-bit DMG palette color index to an ARGB pixel value.
    fn dmg_color(index: u8) -> u32 {
        match index {
            0 => COLOR_WHITE,
            1 => COLOR_LIGHT_GREEN,
            2 => COLOR_DARK_GREEN,
            3 => COLOR_BLACK,
            _ => unreachable!(),
        }
    }

    /// Look up a tile pixel color using the shared tile-data addressing logic.
    /// Returns the raw 2-bit color index (before palette mapping).
    fn tile_color(&self, tile_index: u8, tile_sub_x: usize, tile_sub_y: usize, signed: bool) -> u8 {
        // Unsigned mode (LCDC bit 4 = 1): tiles 0-255 at VRAM 0x0000-0x0FF0 (GB 0x8000-0x8FF0)
        // Signed mode  (LCDC bit 4 = 0): tiles -128..127 centred at VRAM 0x1000 (GB 0x9000)
        let tile_addr = if signed {
            (0x1000i32 + (tile_index as i8 as i32) * 16) as usize
        } else {
            tile_index as usize * 16
        };
        let lo = self.vram[tile_addr + tile_sub_y * 2];
        let hi = self.vram[tile_addr + tile_sub_y * 2 + 1];
        let bit = 7 - tile_sub_x;
        (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1)
    }

    /// Apply a DMG palette register to a raw 2-bit color index.
    fn apply_palette(palette: u8, color_id: u8) -> u8 {
        (palette >> (color_id * 2)) & 0x03
    }

    /// Render the background layer into the scanline buffers.
    fn render_background(&self, color_index: &mut [u8; SCREEN_WIDTH], bg_opaque: &mut [bool; SCREEN_WIDTH]) {
        let map_base = if self.lcdc & LCDC_BG_MAP != 0 { 0x1C00 } else { 0x1800 };
        let signed = self.lcdc & LCDC_TILE_DATA == 0;

        let py = self.ly.wrapping_add(self.scy) as usize;
        let tile_row = py / 8;
        let sub_y = py % 8;

        for x in 0..SCREEN_WIDTH {
            let px = (x as u8).wrapping_add(self.scx) as usize;
            let tile_col = px / 8;
            let sub_x = px % 8;

            let tile_idx = self.vram[map_base + tile_row * 32 + tile_col];
            let raw = self.tile_color(tile_idx, sub_x, sub_y, signed);
            let color = Self::apply_palette(self.bgp, raw);

            color_index[x] = color;
            bg_opaque[x] = raw != 0;
        }
    }

    /// Render the window layer into the scanline buffers.
    /// Returns true if any window pixels were drawn (used to advance window_line).
    fn render_window(&self, color_index: &mut [u8; SCREEN_WIDTH], bg_opaque: &mut [bool; SCREEN_WIDTH]) -> bool {
        // WX is the screen X position + 7; values < 7 clip off the left edge.
        let wx = self.wx as i16 - 7;
        let map_base = if self.lcdc & LCDC_WINDOW_MAP != 0 { 0x1C00 } else { 0x1800 };
        let signed = self.lcdc & LCDC_TILE_DATA == 0;
        let sub_y = self.window_line as usize % 8;
        let tile_row = self.window_line as usize / 8;

        let mut drawn = false;
        for x in 0..SCREEN_WIDTH {
            let win_x = x as i16 - wx;
            if win_x < 0 {
                continue;
            }
            let tile_col = win_x as usize / 8;
            let sub_x = win_x as usize % 8;

            let tile_idx = self.vram[map_base + tile_row * 32 + tile_col];
            let raw = self.tile_color(tile_idx, sub_x, sub_y, signed);
            let color = Self::apply_palette(self.bgp, raw);

            color_index[x] = color;
            bg_opaque[x] = raw != 0;
            drawn = true;
        }
        drawn
    }

    /// Render sprites for this scanline into the color buffer.
    fn render_sprites(&self, color_index: &mut [u8; SCREEN_WIDTH], bg_opaque: &[bool; SCREEN_WIDTH]) {
        let tall = self.lcdc & LCDC_OBJ_SIZE != 0;
        let sprite_height: i32 = if tall { 16 } else { 8 };
        let ly = self.ly as i32;

        // Collect up to 10 visible sprites in OAM order.
        // Each entry: (sx, sy, tile_index, attributes)
        let mut visible: Vec<(i32, i32, u8, u8)> = Vec::with_capacity(10);
        for i in 0..40usize {
            let base = i * 4;
            let sy = self.oam[base] as i32 - 16;
            let sx = self.oam[base + 1] as i32 - 8;
            let tile = if tall { self.oam[base + 2] & 0xFE } else { self.oam[base + 2] };
            let attrs = self.oam[base + 3];

            if ly >= sy && ly < sy + sprite_height {
                visible.push((sx, sy, tile, attrs));
                if visible.len() == 10 {
                    break;
                }
            }
        }

        // Draw in reverse order: lower OAM index = higher priority (drawn on top last).
        for &(sx, sy, tile, attrs) in visible.iter().rev() {
            let behind_bg = attrs & 0x80 != 0;
            let y_flip    = attrs & 0x40 != 0;
            let x_flip    = attrs & 0x20 != 0;
            let palette   = if attrs & 0x10 != 0 { self.obp1 } else { self.obp0 };

            let mut row = (ly - sy) as usize;
            if y_flip {
                row = sprite_height as usize - 1 - row;
            }

            // In 8×16 mode the bottom half uses the next tile.
            let tile_idx = if tall && row >= 8 { tile + 1 } else { tile };
            let tile_row = row % 8;

            for col in 0..8i32 {
                let screen_x = sx + col;
                if screen_x < 0 || screen_x >= SCREEN_WIDTH as i32 {
                    continue;
                }
                let x = screen_x as usize;
                let tile_col = if x_flip { 7 - col as usize } else { col as usize };

                // Sprites always use unsigned (0x8000-based) tile addressing.
                let raw = self.tile_color(tile_idx, tile_col, tile_row, false);
                if raw == 0 {
                    continue; // color 0 is transparent
                }
                if behind_bg && bg_opaque[x] {
                    continue; // sprite is behind BG color 1-3
                }

                color_index[x] = Self::apply_palette(palette, raw);
            }
        }
    }

    /// Execute a pending OAM DMA transfer. Called from emu.rs with a full MMU read slice.
    /// Copies 160 bytes from (page * 0x100) into OAM.
    pub fn execute_dma(&mut self, src: &[u8]) {
        let len = src.len().min(0xA0);
        self.oam[..len].copy_from_slice(&src[..len]);
    }

    /// Get a byte from VRAM
    pub fn get_vram(&self, addr: u16) -> u8 {
        self.vram[(addr & 0x1FFF) as usize]
    }

    /// Set a byte in VRAM
    pub fn set_vram(&mut self, addr: u16, value: u8) {
        self.vram[(addr & 0x1FFF) as usize] = value;
    }

    /// Get a byte from OAM
    pub fn get_oam(&self, addr: u16) -> u8 {
        self.oam[(addr & 0xFF) as usize]
    }

    /// Set a byte in OAM
    pub fn set_oam(&mut self, addr: u16, value: u8) {
        self.oam[(addr & 0xFF) as usize] = value;
    }
}

impl MemHandler for Ppu {
    fn on_read(&self, addr: u16) -> MemRead {
        match addr {
            // VRAM (0x8000-0x9FFF)
            0x8000..=0x9FFF => MemRead::Replace(self.get_vram(addr)),

            // OAM (0xFE00-0xFE9F)
            0xFE00..=0xFE9F => MemRead::Replace(self.get_oam(addr)),

            // LCD Control Register (0xFF40)
            0xFF40 => MemRead::Replace(self.lcdc),

            // LCD Status Register (0xFF41)
            0xFF41 => MemRead::Replace(self.stat),

            // Scroll Y (0xFF42)
            0xFF42 => MemRead::Replace(self.scy),

            // Scroll X (0xFF43)
            0xFF43 => MemRead::Replace(self.scx),

            // LCD Y Coordinate (0xFF44)
            0xFF44 => MemRead::Replace(self.ly),

            // LY Compare (0xFF45)
            0xFF45 => MemRead::Replace(self.lyc),

            // Background Palette (0xFF47)
            0xFF47 => MemRead::Replace(self.bgp),

            // Object Palette 0 (0xFF48)
            0xFF48 => MemRead::Replace(self.obp0),

            // Object Palette 1 (0xFF49)
            0xFF49 => MemRead::Replace(self.obp1),

            // Window Y Position (0xFF4A)
            0xFF4A => MemRead::Replace(self.wy),

            // Window X Position minus 7 (0xFF4B)
            0xFF4B => MemRead::Replace(self.wx),

            // DMA register (0xFF46) — write-only, reads as 0xFF
            0xFF46 => MemRead::Replace(0xFF),

            // Not a PPU register
            _ => MemRead::PassThrough,
        }
    }

    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite {
        match addr {
            // VRAM (0x8000-0x9FFF)
            0x8000..=0x9FFF => {
                self.set_vram(addr, value);
                MemWrite::Block
            }

            // OAM (0xFE00-0xFE9F)
            0xFE00..=0xFE9F => {
                self.set_oam(addr, value);
                MemWrite::Block
            }

            // LCD Control Register (0xFF40)
            0xFF40 => {
                self.lcdc = value;
                MemWrite::Block
            }

            // LCD Status Register (0xFF41)
            0xFF41 => {
                // Only bits 3-6 are writable
                self.stat = (self.stat & 0x07) | (value & 0xF8);
                MemWrite::Block
            }

            // Scroll Y (0xFF42)
            0xFF42 => {
                self.scy = value;
                MemWrite::Block
            }

            // Scroll X (0xFF43)
            0xFF43 => {
                self.scx = value;
                MemWrite::Block
            }

            // LCD Y Coordinate (0xFF44) - read-only
            0xFF44 => MemWrite::Block,

            // LY Compare (0xFF45)
            0xFF45 => {
                self.lyc = value;
                MemWrite::Block
            }

            // Background Palette (0xFF47)
            0xFF47 => {
                self.bgp = value;
                MemWrite::Block
            }

            // Object Palette 0 (0xFF48)
            0xFF48 => {
                self.obp0 = value;
                MemWrite::Block
            }

            // Object Palette 1 (0xFF49)
            0xFF49 => {
                self.obp1 = value;
                MemWrite::Block
            }

            // Window Y Position (0xFF4A)
            0xFF4A => {
                self.wy = value;
                MemWrite::Block
            }

            // Window X Position minus 7 (0xFF4B)
            0xFF4B => {
                self.wx = value;
                MemWrite::Block
            }

            // OAM DMA transfer (0xFF46): writing triggers a copy from (value * 0x100) to OAM.
            // The actual copy is performed in emu.rs where the MMU is accessible.
            0xFF46 => {
                self.pending_dma = Some(value);
                MemWrite::Block
            }

            // Not a PPU register
            _ => MemWrite::PassThrough,
        }
    }
}
