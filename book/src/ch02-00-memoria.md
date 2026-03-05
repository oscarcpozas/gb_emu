# El mapa de memoria

Antes de hablar de cualquier otro componente, necesitamos entender cómo la Gameboy organiza su memoria. Todo en esta consola —gráficos, sonido, controles, código del juego— pasa por un único espacio de direcciones de 16 bits. Eso significa que hay exactamente **65.536 posiciones de memoria** (de `0x0000` a `0xFFFF`), y cada componente del sistema tiene su propio trozo reservado.

## El mapa completo

```
0x0000 ┬──────────────────────────────────────┐
       │  Boot ROM / ROM del cartucho banco 0 │  16 KB
0x3FFF ┼──────────────────────────────────────┤
       │  ROM del cartucho banco N (switchable)│  16 KB
0x7FFF ┼──────────────────────────────────────┤
       │  Video RAM (VRAM)                    │   8 KB
0x9FFF ┼──────────────────────────────────────┤
       │  RAM del cartucho (save data)        │   8 KB
0xBFFF ┼──────────────────────────────────────┤
       │  Work RAM (WRAM)                     │   8 KB
0xDFFF ┼──────────────────────────────────────┤
       │  Echo RAM (espejo de WRAM)           │
0xFDFF ┼──────────────────────────────────────┤
       │  OAM (datos de sprites)              │  160 bytes
0xFE9F ┼──────────────────────────────────────┤
       │  (no usable)                         │
0xFEFF ┼──────────────────────────────────────┤
       │  Registros de I/O (hardware)         │
0xFF7F ┼──────────────────────────────────────┤
       │  High RAM (HRAM)                     │  127 bytes
0xFFFE ┼──────────────────────────────────────┤
       │  Registro IE (interrupciones)        │   1 byte
0xFFFF └──────────────────────────────────────┘
```

Cuando la CPU quiere leer o escribir algo, siempre usa una dirección de este mapa. El componente encargado de decidir quién responde a cada dirección es la **MMU** (Memory Management Unit, o Unidad de Gestión de Memoria).

## La MMU: el árbitro del tráfico

La MMU es básicamente un policía de tráfico. Cuando la CPU dice "dame el byte de la dirección `0xFF40`", la MMU mira quién está registrado para esa dirección y le pregunta. En nuestro caso, `0xFF40` es el registro de control de la PPU, así que la MMU se lo pregunta a la PPU.

### MemHandlers: los "inquilinos" de la memoria

Cada componente del emulador implementa el trait `MemHandler`, que tiene solo dos funciones:

```rust
pub trait MemHandler {
    fn on_read(&self, addr: u16) -> MemRead;
    fn on_write(&mut self, addr: u16, value: u8) -> MemWrite;
}
```

Y cada función devuelve una decisión:

```rust
pub enum MemRead {
    Replace(u8),   // "yo respondo con este valor"
    PassThrough,   // "no soy yo, pregunta al siguiente"
}

pub enum MemWrite {
    Replace(u8),   // "escribe este valor modificado"
    PassThrough,   // "escríbelo en la RAM normal"
    Block,         // "yo lo gestiono, no toques la RAM"
}
```

### Registrar los handlers

En `emu.rs`, al arrancar el emulador, registramos qué componente gestiona qué rango de direcciones:

```rust
mmu.add_handler((0x0000, 0x00FF), boot_rom_handler);   // Boot ROM
mmu.add_handler((0x0000, 0x7FFF), cartridge_handler);  // ROM del juego
mmu.add_handler((0x8000, 0x9FFF), ppu_handler);        // VRAM
mmu.add_handler((0xFE00, 0xFE9F), ppu_handler);        // OAM (sprites)
mmu.add_handler((0xFF40, 0xFF4B), ppu_handler);        // Registros LCD
mmu.add_handler((0xFF0F, 0xFF0F), interrupt_handler);  // Registro IF
mmu.add_handler((0xFFFF, 0xFFFF), interrupt_handler);  // Registro IE
mmu.add_handler((0xFF00, 0xFF00), joypad_handler);     // Controles
mmu.add_handler((0xFF04, 0xFF07), timer_handler);      // Temporizadores
mmu.add_handler((0xFF10, 0xFF3F), apu_handler);        // Audio
```

### Prioridad de handlers

Un detalle importante: **pueden registrarse varios handlers para la misma dirección**. La MMU los consulta en orden. El primero que responda con `Replace` o `Block` gana; si devuelve `PassThrough`, se pregunta al siguiente.

Esto es crucial para la Boot ROM. Las primeras 256 posiciones (`0x0000-0x00FF`) las gestiona la Boot ROM durante el arranque, pero luego las cede al cartucho. Por eso la Boot ROM se registra **antes** que el cartucho:

```rust
// Boot ROM primero → tiene prioridad para 0x0000-0x00FF
mmu.add_handler((0x0000, 0x00FF), boot_rom_handler);
// Cartucho después → cubre todo 0x0000-0x7FFF
mmu.add_handler((0x0000, 0x7FFF), cartridge_handler);
```

Cuando la Boot ROM termina, escribe un `1` en `0xFF50`, y a partir de ese momento devuelve `PassThrough` para `0x0000-0x00FF`, dejando que el cartucho responda.

### La Echo RAM

Hay un pequeño detalle peculiar: las direcciones `0xE000-0xFDFF` son un espejo exacto de `0xC000-0xDDFF` (la Work RAM). Esto se llama Echo RAM y es una consecuencia del diseño del hardware original. En nuestro emulador lo gestionamos directamente en la MMU:

```rust
fn is_echo_ram(&self, addr: u16) -> bool {
    addr >= 0xE000 && addr <= 0xFDFF
}
// Si es Echo RAM, simplemente restamos 0x2000 para apuntar a la WRAM real
```
