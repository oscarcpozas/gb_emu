### How to generate the instr.rs file

```rust
cargo run --bin codegen ./codegen/res/LR35902_opcodes.patched.json ./emu/src/instr.rs
```

 ### How to run emu

```rust
 cargo run --bin emu ./misc/tetris.gb
```
