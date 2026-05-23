use crate::mmu::{MemoryBus, Tickable};

const SHADES: [u8; 4] = [0xFF, 0xAA, 0x55, 0x00];

// SCREEN
pub struct Screen(pub [u8; 160 * 144 * 4]);

impl Screen {
    fn new() -> Self { Self([0xFF; 160 * 144 * 4]) }

    fn set(&mut self, x: usize, y: usize, shade: u8) {
        let i = (y * 160 + x) * 4;
        self.0[i..i+3].fill(shade);
        self.0[i+3] = 0xFF;
    }

    fn shade_at(&self, x: usize, y: usize) -> u8 {
        self.0[(y * 160 + x) * 4]
    }
}

// LAYER TRAIT
trait Layer {
    fn draw<B: MemoryBus>(&self, bus: &B, screen: &mut Screen, ly: u8, lcdc: u8);
}

// BACKGROUND & WINDOW
struct BgLayer;

impl BgLayer {
    fn pixel<B: MemoryBus>(&self, bus: &B, lcdc: u8, px: u16, py: u16, palette: u8, is_win: bool) -> u8 {
        let map_base = if lcdc & (if is_win { 0x40 } else { 0x08 }) != 0 { 0x9C00u16 } else { 0x9800 };
        let tile_idx = bus.read(map_base + (py / 8) * 32 + (px / 8));
        let tile_addr = if lcdc & 0x10 != 0 {
            0x8000 + tile_idx as u16 * 16
        } else {
            (0x9000i32 + tile_idx as i8 as i32 * 16) as u16
        };
        let row = tile_addr + (py % 8) * 2;
        let (lo, hi) = (bus.read(row), bus.read(row + 1));
        let bit = 7 - (px % 8);
        let id = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
        (palette >> (id * 2)) & 0x03
    }
}

impl Layer for BgLayer {
    fn draw<B: MemoryBus>(&self, bus: &B, screen: &mut Screen, ly: u8, lcdc: u8) {
        let scx = bus.read(0xFF43);
        let scy = bus.read(0xFF42);
        let wx  = bus.read(0xFF4B).wrapping_sub(7);
        let wy  = bus.read(0xFF4A);
        let bgp = bus.read(0xFF47);

        for x in 0u8..160 {
            let (win, px, py) = if (lcdc & 0x20 != 0) && ly >= wy && x >= wx {
                (true, (x - wx) as u16, (ly - wy) as u16)
            } else {
                (false, x.wrapping_add(scx) as u16, ly.wrapping_add(scy) as u16)
            };
            let color = self.pixel(bus, lcdc, px, py, bgp, win);
            screen.set(x as usize, ly as usize, SHADES[color as usize]);
        }
    }
}

// SPRITS
struct SpriteLayer;

impl Layer for SpriteLayer {
    fn draw<B: MemoryBus>(&self, bus: &B, screen: &mut Screen, ly: u8, _lcdc: u8) {
        let ly_i = ly as i16;
        let obp0 = bus.read(0xFF48);
        let obp1 = bus.read(0xFF49);
        let mut count = 0;

        for i in 0..40usize {
            if count == 10 { break; }
            let base = 0xFE00 + i as u16 * 4;
            let sy = bus.read(base) as i16 - 16;
            if ly_i < sy || ly_i >= sy + 8 { continue; }
            count += 1;

            let sx   = bus.read(base + 1) as i16 - 8;
            let tile = bus.read(base + 2);
            let attr = bus.read(base + 3);
            let pal  = if attr & 0x10 != 0 { obp1 } else { obp0 };
            let mut row = (ly_i - sy) as u16;
            if attr & 0x40 != 0 { row = 7 - row; }

            let addr = 0x8000 + tile as u16 * 16 + row * 2;
            let (lo, hi) = (bus.read(addr), bus.read(addr + 1));

            for px in 0i16..8 {
                let tx = sx + px;
                if tx < 0 || tx >= 160 { continue; }
                let bit = if attr & 0x20 != 0 { px } else { 7 - px } as u8;
                let id  = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
                if id == 0 { continue; } // color 0 is transparent

                // BG-over-sprite: 0xFF means BG color 0 (transparent)
                if attr & 0x80 != 0 && screen.shade_at(tx as usize, ly as usize) != 0xFF { continue; }

                screen.set(tx as usize, ly as usize, SHADES[((pal >> (id * 2)) & 0x03) as usize]);
            }
        }
    }
}

// PPU
pub struct Ppu {
    pub screen: Screen,
    dot: u32,
    ly:  u8,
}

impl Ppu {
    pub fn new() -> Self {
        Self { screen: Screen::new(), dot: 0, ly: 0 }
    }

    fn render_scanline<B: MemoryBus>(&mut self, bus: &B, lcdc: u8) {
        BgLayer.draw(bus, &mut self.screen, self.ly, lcdc);
        if lcdc & 0x02 != 0 { SpriteLayer.draw(bus, &mut self.screen, self.ly, lcdc); }
    }
}

impl Tickable for Ppu {
    fn tick<B: MemoryBus>(&mut self, cycles: u32, bus: &mut B) {
        let lcdc = bus.read(0xFF40);
        if lcdc & 0x80 == 0 {
            self.ly = 0; self.dot = 0;
            bus.write(0xFF44, 0);
            bus.write(0xFF41, bus.read(0xFF41) & 0xFC);
            return;
        }

        self.dot += cycles;
        if self.dot >= 456 {
            self.dot -= 456;
            self.ly = (self.ly + 1) % 154;
            bus.write(0xFF44, self.ly);

            // LYC=LY coincidence
            if self.ly == bus.read(0xFF45) {
                let stat = bus.read(0xFF41) | 0x04;
                bus.write(0xFF41, stat);
                if stat & 0x40 != 0 { bus.write(0xFF0F, bus.read(0xFF0F) | 0x02); }
            } else {
                bus.write(0xFF41, bus.read(0xFF41) & !0x04);
            }

            if self.ly == 144 { bus.write(0xFF0F, bus.read(0xFF0F) | 0x01); } // VBlank
        }

        let mut stat = bus.read(0xFF41);
        let old_mode = stat & 0x03;
        let new_mode = if self.ly >= 144 { 1 } else if self.dot < 80 { 2 } else if self.dot < 252 { 3 } else { 0 };

        if old_mode != new_mode {
            stat = (stat & 0xFC) | new_mode;
            let irq = match new_mode { 0 => stat & 0x08, 1 => stat & 0x10, 2 => stat & 0x20, _ => 0 };
            if irq != 0 { bus.write(0xFF0F, bus.read(0xFF0F) | 0x02); }
            if new_mode == 0 && self.ly < 144 { self.render_scanline(bus, lcdc); }
        }
        bus.write(0xFF41, stat);
    }
}
