# PPU: la unidad de procesamiento de gráficos

La **PPU** (Picture Processing Unit) es el componente que se encarga de dibujar todo lo que ves en pantalla. La pantalla de la Gameboy es de **160×144 píxeles**, en escala de grises con 4 tonos (blanco, gris claro, gris oscuro y negro). En nuestro emulador los mapeamos a una paleta de verdes para imitar el aspecto de la pantalla original.

## El sistema de tiles

La Gameboy no dibuja píxeles sueltos. Todo se construye a partir de **tiles**: pequeños bloques de **8×8 píxeles**. Cada píxel de un tile ocupa 2 bits (para representar los 4 tonos), así que cada tile pesa `8 × 8 × 2 bits = 128 bits = 16 bytes`.

Los tiles se almacenan en la **VRAM** (`0x8000-0x97FF`), que tiene espacio para 384 tiles. El juego carga sus gráficos aquí antes de que se muestren en pantalla.

```
Un tile (8×8 píxeles, 2 bits por píxel):

Byte 0 (bit alto):  0 1 1 0 0 1 1 0
Byte 1 (bit bajo):  0 0 1 1 1 1 0 0
                    ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓
Color index:        0 1 2 1 1 2 1 0  →  ░▒██▒░  (traducido a tonos)
```

Los dos bytes se combinan bit a bit: el bit del byte 0 es el bit alto del color, el del byte 1 es el bajo. El resultado es un número de 0 a 3 que se pasa por la paleta para obtener el color final.

## El mapa de tiles (Tile Map)

Tener los tiles en VRAM no es suficiente; la PPU necesita saber qué tile va en cada posición de la pantalla. Para eso existen los **Tile Maps**: dos grids de 32×32 posiciones (cada posición tiene el índice del tile a mostrar) guardadas en `0x9800-0x9BFF` y `0x9C00-0x9FFF`.

La pantalla visible es de 20×18 tiles (160÷8 × 144÷8), pero el Tile Map es de 32×32. Eso significa que hay **un mundo de 256×256 píxeles** del que la pantalla solo muestra una ventana. Los registros **SCX** y **SCY** (Scroll X e Y) controlan qué parte del mundo se ve, permitiendo el scroll.

```
Mundo de 256×256 (32×32 tiles)
┌────────────────────────────────┐
│                                │
│     ┌──────────────┐           │
│     │  Pantalla    │           │
│     │  160×144     │  ← viewport
│     │  (SCX, SCY)  │           │
│     └──────────────┘           │
│                                │
└────────────────────────────────┘
```

## Los modos de la PPU

La PPU no dibuja todo el frame de golpe. Procesa línea a línea (scanline), y para cada línea pasa por cuatro modos en orden:

```
Tiempo →

Scanline 0:   [OAM: 80c][Transfer: 172c][HBlank: 204c]  = 456 ciclos
Scanline 1:   [OAM: 80c][Transfer: 172c][HBlank: 204c]
...
Scanline 143: [OAM: 80c][Transfer: 172c][HBlank: 204c]
─────────────────────────────────────────────────────── 144 × 456 = 65.664 ciclos
Scanlines 144-153:       [VBlank: 10 × 456 = 4.560c]
─────────────────────────────────────────────────────── TOTAL: 70.224 ciclos/frame
```

### Modo OAM (modo 2) — 80 ciclos
La PPU busca en la **OAM** (Object Attribute Memory, `0xFE00-0xFE9F`) qué sprites aparecen en la línea actual. Hay un máximo de 40 sprites en total, pero solo **10 pueden mostrarse por scanline** (limitación del hardware original).

### Modo Transfer (modo 3) — 172 ciclos
La PPU dibuja los píxeles de la línea actual en el framebuffer. En este modo la VRAM no es accesible para la CPU (en hardware real; en nuestro emulador no lo restringimos por simplicidad).

### Modo HBlank (modo 0) — 204 ciclos
Pausa horizontal. La línea ya está dibujada. La CPU puede acceder libremente a la VRAM. Si el juego quiere actualizar gráficos durante el frame, este es el momento.

### Modo VBlank (modo 1) — 4.560 ciclos (10 scanlines)
La PPU ha terminado las 144 líneas visibles. Es el equivalente al "descanso" mientras el rayo de electrones del televisor volvía arriba en los CRT. La PPU lanza la **interrupción VBlank** y el juego usa este tiempo para actualizar la lógica y los gráficos. El framebuffer está completo y se muestra en pantalla.

## El registro LCDC: el panel de control de la PPU

El registro `0xFF40` (LCDC) es el principal registro de control. Cada bit activa o desactiva una característica:

```
Bit 7: LCD encendida/apagada
Bit 6: Tile map de la Ventana (0=0x9800, 1=0x9C00)
Bit 5: Ventana activada
Bit 4: Tile data (0=0x8800 modo firmado, 1=0x8000 modo sin signo)
Bit 3: Tile map del Fondo (0=0x9800, 1=0x9C00)
Bit 2: Tamaño de sprite (0=8×8, 1=8×16)
Bit 1: Sprites activados
Bit 0: Fondo activado
```

## Las tres capas gráficas

La PPU dibuja en este orden (de más atrás a más adelante):

### 1. Background (fondo)
El mundo desplazable de 256×256 píxeles. Dibuja el tile correspondiente a cada pixel según SCX/SCY.

### 2. Window (ventana)
Una segunda capa fija (no desplazable) que se superpone al fondo. Se usa típicamente para HUDs: marcadores, vidas, texto de diálogo. Su posición se controla con **WX** y **WY**. La ventana tiene su propio contador de líneas interno (`window_line`) que solo avanza cuando la ventana es visible, independientemente de LY.

### 3. Sprites (objetos)
Los sprites son tiles móviles. Cada sprite ocupa 4 bytes en la OAM:
- **Y** — posición vertical (con offset de 16)
- **X** — posición horizontal (con offset de 8)
- **Tile** — índice del tile a usar
- **Atributos** — prioridad, flip horizontal/vertical, paleta (OBP0 u OBP1)

Los sprites pueden aparecer delante o detrás del fondo según el bit de prioridad. Si el bit está activo, el sprite se dibuja detrás de los píxeles de fondo que no sean color 0 (transparente).

## DMA: cargando sprites rápido

Copiar 160 bytes de datos de sprites a la OAM instrucción a instrucción sería muy lento. Por eso existe el **DMA** (Direct Memory Access): escribir una dirección en `0xFF46` desencadena una transferencia automática de 160 bytes desde esa dirección a la OAM.

En nuestro emulador, la DMA no es instantánea dentro de un ciclo. La guardamos como "pendiente" y la ejecutamos al inicio del siguiente `step()`, cuando tenemos acceso tanto a la MMU como a la PPU:

```rust
// En step():
let pending_dma = self.ppu.borrow_mut().pending_dma.take();
if let Some(page) = pending_dma {
    let src_addr = (page as u16) << 8;
    let src: Vec<u8> = (0..0xA0u16).map(|i| self.mmu.get8(src_addr + i)).collect();
    self.ppu.borrow_mut().execute_dma(&src);
}
```

## Las paletas

Los índices de color (0-3) que salen de los tiles no son colores directos. Pasan por un registro de paleta que los traduce:

- **BGP** (`0xFF47`): paleta del fondo y la ventana
- **OBP0** (`0xFF48`): paleta de sprites grupo 0
- **OBP1** (`0xFF49`): paleta de sprites grupo 1

Cada paleta es un byte donde cada par de bits define el color real para ese índice:

```
BGP = 0b11100100
        ││││││└┘ → índice 0 → color 0 (blanco)
        ││││└┘   → índice 1 → color 1 (gris claro)
        ││└┘     → índice 2 → color 2 (gris oscuro)
        └┘       → índice 3 → color 3 (negro)
```

En nuestro emulador mapeamos los 4 colores a una paleta de verdes que imita el aspecto de la pantalla DMG original:

```rust
const COLOR_WHITE:      u32 = 0xFFFFFFFF;
const COLOR_LIGHT_GREEN: u32 = 0xFFADD794;
const COLOR_DARK_GREEN:  u32 = 0xFF306230;
const COLOR_BLACK:       u32 = 0xFF0F380F;
```
