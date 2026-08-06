# PokéGB — Game Boy Emulator in Rust

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WASM](https://img.shields.io/badge/target-WebAssembly-654ff0?style=flat&logo=webassembly&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green)

A complete Game Boy (DMG) emulator written in Rust, compiled to WebAssembly for the browser and native via `minifb`. Implements the Sharp LR35902 CPU, MBC3 memory bank controller, scanline PPU, timer subsystem, and joypad input — all in **~960 lines of Rust**.

[**Play it live →**](https://itsvinm.github.io/gameboy_emu_poke/)

<p align="center">
  <img src="images/pokemon.png"   width="260" height="400" alt="Pokémon Red">
  &nbsp;
  <img src="images/zelda.png"     width="260" height="400" alt="The Legend of Zelda">
  &nbsp;
  <img src="images/supermario.jpg" width="260" height="400" alt="Super Mario Land">
</p>

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     EmulatorState                           │
│                                                             │
│  ┌──────────┐    ┌──────────┐    ┌──────────────────────┐  │
│  │   CPU    │───▶│   MMU    │◀───│        PPU           │  │
│  │ LR35902  │    │  MBC3    │    │  Scanline Renderer   │  │
│  │          │    │          │    │  BG / Window / Sprite │  │
│  │ Regs:    │    │ ROM Bank │    │  STAT Interrupts      │  │
│  │  A F     │    │ RAM Bank │    │  LY/LYC Compare      │  │
│  │  B C     │    │ DMA      │    └──────────────────────┘  │
│  │  D E     │    │ I/O Regs │                              │
│  │  H L     │    │ HRAM     │    ┌──────────────────────┐  │
│  │  SP PC   │    │ Save/Load│    │     Timer             │  │
│  │  IME     │    └──────────┘    │  DIV (0xFF04)         │  │
│  └──────────┘                    │  TIMA/TMA/TAC         │  │
│                                  │  Configurable Freq    │  │
│  ┌──────────┐                    └──────────────────────┘  │
│  │  Joypad  │                                              │
│  │ D-pad    │    Bus: MemoryBus trait (generic over MMU)   │
│  │ Buttons  │    Tickable trait  (PPU + Timer sync)        │
│  │ IRQ edge │                                              │
│  └──────────┘                                              │
└─────────────────────────────────────────────────────────────┘
```

### Tick Pipeline

Every CPU instruction returns a cycle count. That count drives the PPU and timer proportionally, keeping everything in sync without a separate scheduler:

```text
cpu.step(bus)  ──▶  cycles  ──▶  ppu.tick(cycles)
                       │
                       ▼
                  div_acc += cycles  (overflow at 256 → DIV++)
                  timer_acc += cycles (threshold from TAC → TIMA++)
```

One full frame = **70,224 T-cycles** (154 scanlines × 456 dots each).

---

## Stack

| Component | Implementation | Detail |
|-----------|---------------|--------|
| **CPU** | `src/cpu.rs` (466 lines) | Full LR35902: 256 base opcodes + 256 CB-prefixed. Fetch-decode-execute loop with cycle-accurate timing. |
| **Registers** | `src/registers.rs` (42 lines) | A/F/B/C/D/E/H/L/SP/PC/IME. Z/N/H/C flag accessors. 16-bit pair read/write (AF, BC, DE, HL). |
| **MMU** | `src/mmu.rs` (107 lines) | `MemoryBus` trait for generic memory access. MBC3 banking (ROM up to 2MB, RAM up to 32KB). DMA transfer at 0xFF46. |
| **PPU** | `src/ppu.rs` (109 lines) | Scanline-based renderer. BG with SCX/SCY scroll, window with WX/WY, sprites (40 OAM, 10/line limit). STAT mode interrupts. |
| **Runtime** | `src/lib.rs` / `src/main.rs` | WASM via `wasm-bindgen` → browser canvas. Native via `minifb` window. Conditional compilation with `cfg(target_arch)`. |
| **Timer** | Inlined in tick loop | DIV (free-running 16-bit), TIMA/TMA/TAC with 4 configurable frequencies. Timer overflow triggers IRQ. |
| **Joypad** | `src/lib.rs` | Edge-triggered interrupt on button press. `prev_joyp` tracking for falling-edge detection. |

---

## Build

### WebAssembly (browser)

```bash
# Install wasm-pack if you haven't
cargo install wasm-pack

# Build
wasm-pack build --target web

# Serve locally
python3 -m http.server 8080
# Open http://localhost:8080
```

### Native (desktop)

```bash
cargo run --release -- rom.gb
```

Requires `rom.gb` in the project root (ROM not included — copyright).

---

## Usage

### Loading a ROM

Open the live demo and click **LOAD ROM** to select any `.gb` file. The ROM is cached in `localStorage` so it persists across reloads.

### Keyboard Controls

| Key | Game Boy Button |
|-----|----------------|
| `↑` `↓` `←` `→` | D-pad |
| `A` | A button |
| `B` | B button |
| `Enter` | Start |
| `S` | Select |

Touch controls are available on mobile. The on-screen D-pad and action buttons map directly to joypad input.

### Save System

- **Auto-save**: SRAM is saved to `localStorage` every 5 seconds
- **Export**: Click **EXPORT .SAV** to download a `.sav` file
- **Import**: Click **IMPORT .SAV** to load a save from another device
- **Native**: Save data writes to `rom.sav` in the working directory (press `F5` to save manually)

### Debugger

Click **DBG** to open the debug panel, which exposes:

- **Registers**: PC, SP, AF, BC, DE, HL
- **Flags**: Z, N, H, C, IME, HALT status
- **System**: LY, LCDC, STAT, IF, IE
- **Controls**: Pause / Resume / Step (single instruction)

---

## Technical Deep Dive

### CPU — Instruction Decoding

The LR35902 has 256 base opcodes and 256 CB-prefixed opcodes. The decoder uses a single `match` expression on the fetched opcode byte, with bit-field extraction to handle grouped instructions:

```rust
// INC/DEC r8 — bits [5:3] select the target register
0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
    let r = (op >> 3) & 0x07;          // register index (0=B,1=C,...,6=(HL),7=A)
    let v = self.read_r8(r, bus);      // read current value
    let result = v.wrapping_add(1);
    self.write_r8(r, result, bus);     // write back
    let h = (v & 0x0F) == 0x0F;       // half-carry
    self.regs.set_flags(result == 0, false, h, self.regs.get_flag_c());
    if r == 6 { 12 } else { 4 }       // (HL) costs extra cycles
}
```

**ALU operations** (0x80–0xBF) are decoded the same way — the opcode bits select both the operation (ADD/SUB/AND/OR/XOR/CP) and the source register. Immediate variants (0xC6, 0xD6, etc.) share the same `alu()` method by normalizing the opcode before dispatch.

**CB-prefix** instructions decode from the top 2 bits: `00` = rotates/shifts/SWAP, `01` = BIT test, `10` = RES, `11` = SET. The bit index and register are extracted from the lower bits.

**Flag computation** follows the hardware spec exactly:
- **Z** (Zero): Set when result is 0
- **N** (Subtract): Set by subtract instructions
- **H** (Half-Carry): Carry from bit 3 (8-bit) or bit 11 (16-bit)
- **C** (Carry): Carry from bit 7 (8-bit) or bit 15 (16-bit)

### Memory Management — MBC3 Banking

The Game Boy address space is partitioned with bank switching:

```
0x0000-0x3FFF  ROM Bank 0          (fixed, 16KB)
0x4000-0x7FFF  ROM Bank N          (switchable, 16KB)
0x8000-0x9FFF  VRAM                 (8KB)
0xA000-0xBFFF  External RAM Bank M  (switchable, 8KB each)
0xC000-0xDFFF  Work RAM             (8KB)
0xE000-0xFDFF  Echo RAM             (mirror of WRAM)
0xFE00-0xFE9F  OAM                  (sprite attribute table)
0xFF00-0xFF7F  I/O Registers
0xFF80-0xFFFE  High RAM             (fast access)
0xFFFF         Interrupt Enable
```

**Bank switching** is triggered by writes to specific address ranges:

| Write Address | Effect |
|---------------|--------|
| `0x2000-0x3FFF` | Select ROM bank (1–127). Bank 0 maps as bank 1. |
| `0x4000-0x5FFF` | Select RAM bank (0–3). |
| `0xFF46` | DMA: Copies 160 bytes from `(val << 8)` to OAM. |

**DIV register** (0xFF40) resets to 0 on any write — this is faithful hardware behavior, not a bug.

### PPU — Scanline Rendering

The PPU renders one scanline at a time during HBlank (mode 0), triggered when the STAT mode changes. The full scanline timing:

```
One scanline = 456 dots
  ├── Mode 2 (OAM scan):    80 dots
  ├── Mode 3 (Drawing):    172 dots
  ├── Mode 0 (HBlank):     204 dots  ◀── BG + sprites rendered here
  │
  After line 143: Mode 1 (VBlank) for 10 lines
  Total: 154 lines per frame
```

**Background rendering** (`draw_bg`):
1. Read SCX/SCY scroll registers and LCDC flags
2. For each pixel (0–159), compute the tile map address from scroll-adjusted coordinates
3. Look up the tile index in VRAM (0x9800 or 0x9C00 based on LCDC bit 3)
4. Decode the tile data (two bytes per row, bit-plane format) at 0x8000 + idx×16
5. Extract the 2-bit pixel ID and map through the BGP palette

**Window rendering** overlays the background when the pixel falls within the WX/WY window region and LCDC bit 5 is set. It uses the same tile decoding pipeline but with the window-specific tile map (LCDC bit 6).

**Sprite rendering** (`draw_sprites`):
1. Scan all 40 OAM entries, stop at 10 sprites on the current line (hardware limit)
2. For each visible sprite: read tile index, position, and attributes
3. Handle Y-flip (bit 6) and X-flip (bit 5) on the tile row
4. Apply priority: if bit 7 is set, sprite is hidden behind non-zero BG pixels
5. Render using OBP0 or OBP1 palette (attribute bit 4)

The **STAT register** tracks mode transitions and fires IRQs on configured events (mode 0/1/2, LY=LYC).

### Joypad — Edge-Triggered Interrupt

The joypad uses active-low logic (0 = pressed). The interrupt fires on a **falling edge** — when a button transitions from unpressed to pressed:

```rust
if (prev_joyp & !current_joyp) & 0x0F != 0 {
    // Button just pressed → set joypad interrupt bit in IF
    bus.write(0xFF0F, bus.read(0xFF0F) | 0x10);
}
prev_joyp = current_joyp;
```

The joypad select bits (0xFF00 bits 4–5) determine whether the D-pad or button matrix is read.

### Dual-Target Compilation

The codebase compiles for both WASM and native targets using conditional compilation:

```rust
#[cfg(not(target_arch = "wasm32"))]
pub fn debug_print<B: MemoryBus>(&self, bus: &B) { /* native-only */ }
```

The WASM target exposes `EmulatorState` via `#[wasm_bindgen]` with methods for frame stepping, register reads, joypad input, and save/load. The browser renders directly to a `<canvas>` via `putImageData`, reading the framebuffer pointer from WASM linear memory.

---

## Status

The emulator runs Pokemon Red, Zelda, and Super Mario Land. CPU, PPU, timer, and joypad are functionally complete. Test suite is planned but not yet implemented.

---

## Project Structure

```
gameboy_emu_poke/
├── src/
│   ├── registers.rs   ── Register file with flag helpers
│   ├── cpu.rs         ── LR35902 instruction decoder (256 + 256 CB)
│   ├── mmu.rs         ── Memory bus, MBC3 banking, DMA
│   ├── ppu.rs         ── Scanline PPU (BG, window, sprites)
│   ├── lib.rs         ── WASM entry point, EmulatorState
│   └── main.rs        ── Native entry point (minifb)
├── index.html         ── Browser UI with debugger panel
├── Cargo.toml
└── images/
    ├── pokemon.png
    ├── zelda.png
    └── supermario.jpg
```

---

## License

MIT
