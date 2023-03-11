Trying to implement a GameBoy emulator in Rust. The goal is to have [100% accuracy with the original hardware](https://mgba.io/2017/04/30/emulation-accuracy).

This is still a work in progress, even the notes that I use in the project are [also public](https://www.notion.so/oscarcpozas/Emulando-la-Gameboy-en-Rust-8919f3bae14947ce9ff4111986a27e29?pvs=4).

## Instruction to run the project

### How to generate the instr.rs file

```rust
cargo run --bin codegen ./codegen/res/LR35902_opcodes.patched.json ./emu/src/instr.rs
```

 ### How to run emu

```rust
 cargo run --bin emu ./misc/tetris.gb
```

### Supported by

[<img src="https://resources.jetbrains.com/storage/products/company/brand/logos/jb_beam.png" alt="JetBrains Logo (Main) logo." width="180">](https://jb.gg/OpenSourceSupport)
