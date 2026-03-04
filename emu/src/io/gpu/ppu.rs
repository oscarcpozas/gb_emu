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
    /// Frame buffer - pixel data to be displayed
    frame_buffer: Arc<Mutex<Vec<u32>>>,
}

impl Ppu {
    pub fn new(frame_buffer: Arc<Mutex<Vec<u32>>>) -> Self {
        Self {
            vram: vec![0; 0x2000],
            oam: vec![0; 0xA0],
            lcdc: 0x91, // Display enabled by default
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            wy: 0,
            wx: 0,
            bgp: 0xFC, // Default palette: 11 10 01 00 (black, dark, light, white)
            obp0: 0xFF,
            obp1: 0xFF,
            mode: MODE_OAM,
            mode_cycles: 0,
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

    /// Render a single scanline
    fn render_scanline(&mut self) {
        if self.ly >= SCREEN_HEIGHT as u8 {
            return;
        }

        // Render background
        if self.lcdc & LCDC_BG_ENABLE != 0 {
            self.render_background();
        }

        // Render window
        if self.lcdc & LCDC_WINDOW_ENABLE != 0 && self.wy <= self.ly {
            self.render_window();
        }

        // Render sprites
        if self.lcdc & LCDC_OBJ_ENABLE != 0 {
            self.render_sprites();
        }
    }

    /// Render the background for the current scanline
    fn render_background(&mut self) {
        let tile_map_addr = if self.lcdc & LCDC_BG_MAP != 0 {
            0x1C00
        } else {
            0x1800
        };
        let tile_data_addr = if self.lcdc & LCDC_TILE_DATA != 0 {
            0x0000
        } else {
            0x1000
        };
        let signed_addressing = (self.lcdc & LCDC_TILE_DATA) == 0;

        let y = (self.ly.wrapping_add(self.scy)) as usize;
        let tile_y = y / 8;
        let tile_sub_y = y % 8;

        let mut frame_buffer = self.frame_buffer.lock().unwrap();

        for x in 0..SCREEN_WIDTH {
            let scrolled_x = (x as u8).wrapping_add(self.scx) as usize;
            let tile_x = scrolled_x / 8;
            let tile_sub_x = 7 - (scrolled_x % 8); // Bits are reversed

            // Get tile index from the tile map
            let tile_map_offset = tile_y * 32 + tile_x;
            let tile_index = self.vram[tile_map_addr + tile_map_offset];

            // Get tile data address
            let tile_addr = if signed_addressing {
                // Signed addressing (0x8800-0x97FF)
                tile_data_addr + ((tile_index as i8 as i16 + 128) as usize * 16)
            } else {
                // Unsigned addressing (0x8000-0x8FFF)
                tile_data_addr + (tile_index as usize * 16)
            };

            // Get tile data for the current row
            let tile_data_low = self.vram[tile_addr + tile_sub_y * 2];
            let tile_data_high = self.vram[tile_addr + tile_sub_y * 2 + 1];

            // Get color bit
            let color_bit = ((tile_data_high >> tile_sub_x) & 0x01) << 1
                | ((tile_data_low >> tile_sub_x) & 0x01);

            // Get color from palette
            let color = (self.bgp >> (color_bit * 2)) & 0x03;

            // Set pixel in frame buffer
            let pixel_addr = self.ly as usize * SCREEN_WIDTH + x;
            frame_buffer[pixel_addr] = match color {
                0 => COLOR_WHITE,
                1 => COLOR_LIGHT_GREEN,
                2 => COLOR_DARK_GREEN,
                3 => COLOR_BLACK,
                _ => unreachable!("Invalid color"),
            };
        }
    }

    /// Render the window for the current scanline
    fn render_window(&mut self) {
        // TODO: Implement window rendering
    }

    /// Render sprites for the current scanline
    fn render_sprites(&mut self) {
        // TODO: Implement sprite rendering
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

            // Not a PPU register
            _ => MemWrite::PassThrough,
        }
    }
}
