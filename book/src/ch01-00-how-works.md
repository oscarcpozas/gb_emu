What happens from when the emulator starts up?

On each step, the following actions happen:

1. **OAM DMA Transfer** - Checking if there's a pending DMA transfer in register 0xFF46

The PPU (screen) has an Object Attribute Memory (OAM) located at `0xFE00-0xFE9F` that stores sprite data. The recommended way to update the OAM is using DMA (Direct Memory Access). [See pandocs](https://gbdev.io/pandocs/OAM.html#writing-data-to-oam)

When you write to register `0xFF46`, you're telling the Game Boy to copy 160 bytes of sprite data into OAM. Here's how it works:

**The register stores a "page" (high byte) of a memory address:**
- Writing `0xC1` to `0xFF46` means "copy from address `0xC100`"
- The address is constructed by shifting the page value: `(page as u16) << 8`
- Example: `0xC1 << 8 = 0xC100`

**Quick mental model:**
- Register value `0xXX` → copies from `0xXX00` to `0xXX9F` (160 bytes)
- Always copies TO: `0xFE00-0xFE9F` (OAM)
- Always copies FROM: `0xXX00-0xXX9F` (where XX is what you wrote to 0xFF46)

**Why only the high byte?**
Since we always copy exactly 160 bytes and the destination is always OAM, we only need to specify where to copy FROM. The low byte is always `0x00` (start of the page), making it simple to just write one byte.

[More info on OAM DMA Transfer](https://gbdev.io/pandocs/OAM_DMA_Transfer.html)

2. WIP