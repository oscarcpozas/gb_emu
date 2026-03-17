# The Basics (Hexadecimal)

One of the most basic things to understand how the system works is understanding what hexadecimal is and how it works. 
Hexadecimal is basically a **base 16** numbering system, using digits 0-9 and letters A-F. In Rust, we write hex values 
with the `0x` prefix (for example `0x8000`)

- **1 byte = 8 bits**
- Each byte is represented with **2 hex characters**
- Example: `0xFF` = 11111111 in binary = 255 in decimal

| Hexadecimal | Binary   | Decimal |
|-------------|----------|---------|
| 0x00        | 00000000 | 0       |
| 0x01        | 00000001 | 1       |
| 0x0F        | 00001111 | 15      |
| 0x10        | 00010000 | 16      |
| 0xFF        | 11111111 | 255     |

### Game Boy Memory Map

The Game Boy has a **16-bit** address space, which means it can address up to **2^16 = 65536 bytes (64 KB)** of memory.

Here's how the Game Boy's memory is organized:

| Address Range   | Size   | Description                     |
|-----------------|--------|---------------------------------|
| 0x0000 - 0x3FFF | 16 KB  | Cartridge ROM (bank 0)          |
| 0x4000 - 0x7FFF | 16 KB  | Cartridge ROM (switchable bank) |
| 0x8000 - 0x9FFF | 8 KB   | VRAM (Video RAM)                |
| 0xA000 - 0xBFFF | 8 KB   | External cartridge RAM          |
| 0xC000 - 0xDFFF | 8 KB   | WRAM (Work RAM)                 |
| 0xE000 - 0xFDFF | 7.5 KB | Echo RAM (mirror of WRAM)       |
| 0xFE00 - 0xFE9F | 160 B  | OAM (Object Attribute Memory)   |
| 0xFEA0 - 0xFEFF | 96 B   | Unusable                        |
| 0xFF00 - 0xFF7F | 128 B  | I/O Registers                   |
| 0xFF80 - 0xFFFE | 127 B  | HRAM (High RAM)                 |
| 0xFFFF          | 1 B    | Interrupt register              |

### Addressing Examples

When you see ranges like `0x00-0x0F` in the code, it means:
- **Start**: `0x00` (0 in decimal)
- **End**: `0x0F` (15 in decimal)
- **Size**: 16 possible values (0-15), which is **1 byte** range

Another example:
- `0x8000 - 0x9FFF`: VRAM range
- Size calculation: `0x9FFF - 0x8000 + 1 = 0x2000 = 8192 bytes = 8 KB`
