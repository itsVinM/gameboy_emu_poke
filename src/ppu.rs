use crate::mmu::{MemoryBus, Tickable};

const SHADES: [u8; 4] = [0xFF, 0xAA, 0x55, 0x00];

pub struct Screen(pub [u8; 160 * 144 * 4]);
impl Screen {
    fn new() -> Self { Self([0xFF; 160 * 144 * 4]) }
    fn set(&mut self, x: usize, y: usize, shade: u8) {
        let i = (y*160+x)*4; 
        self.0[i..i+3].fill(shade); 
        self.0[i+3] = 0xFF;
    }
    fn shade_at(&self, x: usize, y: usize) -> u8 { 
        self.0[(y*160+x)*4]
    }
}

fn draw_bg<B: MemoryBus>(bus: &B, screen: &mut Screen, ly: u8, lcdc: u8) {
    let (scx, scy) = (bus.read(0xFF43), bus.read(0xFF42));
    let (wx,  wy)  = (bus.read(0xFF4B).wrapping_sub(7), bus.read(0xFF4A));
    let bgp        = bus.read(0xFF47);
    for x in 0u8..160 {
        let (win, px, py) = if lcdc & 0x20 != 0 && ly >= wy && x >= wx {
            (true, (x - wx) as u16, (ly - wy) as u16)
        } else {
            (false, x.wrapping_add(scx) as u16, ly.wrapping_add(scy) as u16)
        };
        let map  = if lcdc & (if win { 0x40 } else { 0x08 }) != 0 { 0x9C00u16 } else { 0x9800 };
        let tidx = bus.read(map + (py / 8) * 32 + (px / 8));
        let base = if lcdc & 0x10 != 0 { 0x8000 + tidx as u16 * 16 } else { (0x9000i32 + tidx as i8 as i32 * 16) as u16 };
        let row  = base + (py % 8) * 2;
        let (lo, hi) = (bus.read(row), bus.read(row + 1));
        let bit  = 7 - (px % 8);
        let id   = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
        screen.set(x as usize, ly as usize, SHADES[((bgp >> (id * 2)) & 0x03) as usize]);
    }
}

fn draw_sprites<B: MemoryBus>(bus: &B, screen: &mut Screen, ly: u8) {
    let (ly_i, obp0, obp1) = (ly as i16, bus.read(0xFF48), bus.read(0xFF49));
    let mut count = 0;
    for i in 0..40usize {
        if count == 10 { break; }
        let base = 0xFE00 + i as u16 * 4;
        let sy = bus.read(base) as i16 - 16;
        if ly_i < sy || ly_i >= sy + 8 { continue; }
        count += 1;
        let (sx, tile, attr) = (bus.read(base+1) as i16 - 8, bus.read(base+2), bus.read(base+3));
        let pal = if attr & 0x10 != 0 { obp1 } else { obp0 };
        let mut row = (ly_i - sy) as u16;
        if attr & 0x40 != 0 { row = 7 - row; }
        let addr = 0x8000 + tile as u16 * 16 + row * 2;
        let (lo, hi) = (bus.read(addr), bus.read(addr + 1));
        for px in 0i16..8 {
            let tx = sx + px;
            if !(0..160).contains(&tx) { continue; }
            let bit = if attr & 0x20 != 0 { px } else { 7 - px } as u8;
            let id  = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
            if id == 0 { continue; }
            if attr & 0x80 != 0 && screen.shade_at(tx as usize, ly as usize) != 0xFF { continue; }
            screen.set(tx as usize, ly as usize, SHADES[((pal >> (id * 2)) & 0x03) as usize]);
        }
    }
}

pub struct Ppu { pub screen: Screen, dot: u32, ly: u8 }

impl Ppu {
    pub fn new() -> Self { Self { screen: Screen::new(), dot: 0, ly: 0 } }
}

impl Default for Ppu {
    fn default() -> Self { Self::new() }
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
            let lyc = self.ly == bus.read(0xFF45);
            let s = if lyc { bus.read(0xFF41) | 0x04 } else { bus.read(0xFF41) & !0x04 };
            bus.write(0xFF41, s);
            if lyc && s & 0x40 != 0 { bus.write(0xFF0F, bus.read(0xFF0F) | 0x02); }
            if self.ly == 144 { bus.write(0xFF0F, bus.read(0xFF0F) | 0x01); }
        }
        let mut stat = bus.read(0xFF41);
        let mode = if self.ly >= 144 { 1 } else if self.dot < 80 { 2 } else if self.dot < 252 { 3 } else { 0 };
        if stat & 0x03 != mode {
            stat = (stat & 0xFC) | mode;
            let irq = match mode { 0 => stat & 0x08, 1 => stat & 0x10, 2 => stat & 0x20, _ => 0 };
            if irq != 0 { bus.write(0xFF0F, bus.read(0xFF0F) | 0x02); }
            if mode == 0 && self.ly < 144 {
                draw_bg(bus, &mut self.screen, self.ly, lcdc);
                if lcdc & 0x02 != 0 { draw_sprites(bus, &mut self.screen, self.ly); }
            }
        }
        bus.write(0xFF41, stat);
    }
}
