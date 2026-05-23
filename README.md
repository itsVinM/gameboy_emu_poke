# PokéGB — Game Boy Emulator in Rust

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WASM](https://img.shields.io/badge/target-WebAssembly-654ff0?style=flat&logo=webassembly&logoColor=white)

Game Boy (DMG) emulator written in Rust, compiled to WebAssembly. Compatible with most MBC3 cartridges — tested with Pokémon Red/Blue, The Legend of Zelda, and Super Mario Land.

[**Play it live →**](https://itsvinm.github.io/gameboy_emu_poke/)

<p align="center">
  <img src="images/pokemon.png"   width="260" height="400" alt="Pokémon Red">
  &nbsp;
  <img src="images/zelda.png"     width="260" height="400" alt="Zelda">
  &nbsp;
  <img src="images/supermario.jpg" width="260" height="400" alt="Super Mario Land">
</p>

---

## Stack

| | |
|---|---|
| CPU | Sharp LR35902 — full instruction set, interrupts, HALT |
| MMU | MBC3 ROM/RAM banking, DMA |
| PPU | Scanline renderer — BG, window, sprites, STAT interrupts |
| Runtime | `wasm-bindgen` → browser canvas |

## Build

```bash
wasm-pack build --target web
python3 -m http.server 8080
```

## Playing

Load any `.gb` file with **LOAD ROM** (ROM not included — copyright).

- **Keyboard**: arrows = D-pad · `A`/`B` · `Enter` = Start · `S` = Select
- **Save**: auto-saves to `localStorage` every 5s · **EXPORT / IMPORT .SAV** for transfer

## Tests

Covers CPU opcodes, timer, PPU scanlines, interrupts, and a Pokémon Red boot integration test.

```bash
cargo test && cargo test --test integration
```
