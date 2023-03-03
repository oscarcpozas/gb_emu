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
