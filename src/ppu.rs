use crate::mmu::{MemoryBus, Tickable};

pub struct Ppu {
    pub framebuffer: [u8; 160 * 144 * 4],
    pub dot: u32,
    pub ly:  u8,
}

impl Ppu {
    pub fn new() -> Self {
        Self { framebuffer: [0xFF; 160 * 144 * 4], dot: 0, ly: 0 }
    }
}

impl Tickable for Ppu {
    fn tick<B: MemoryBus>(&mut self, cycles: u32, bus: &mut B) {
        let lcdc = bus.read(0xFF40);
        if lcdc & 0x80 == 0 {
            self.ly = 0;
            self.dot = 0;
            bus.write(0xFF44, 0);
            let stat = bus.read(0xFF41) & 0xFC;
            bus.write(0xFF41, stat);
            return;
        }

        self.dot += cycles;

        // A scanline takes exactly 456 dots
        if self.dot >= 456 {
            self.dot -= 456;
            self.ly = (self.ly + 1) % 154;
            bus.write(0xFF44, self.ly);

            // LYC check: bit 2 of STAT set if LY == LYC
            if self.ly == bus.read(0xFF45) {
                let stat = bus.read(0xFF41) | 0x04;
                bus.write(0xFF41, stat);
                if stat & 0x40 != 0 {
                    let ifr = bus.read(0xFF0F) | 0x02;
                    bus.write(0xFF0F, ifr);
                }
            } else {
                let stat = bus.read(0xFF41) & !0x04;
                bus.write(0xFF41, stat);
            }

            if self.ly == 144 {
                let ifr = bus.read(0xFF0F) | 0x01;
                bus.write(0xFF0F, ifr);
            }
        }

        // --- Mode switching ---
        let mut stat = bus.read(0xFF41);
        let old_mode = stat & 0x03;
        let new_mode = if self.ly >= 144 {
            1 // V-Blank
        } else if self.dot < 80 {
            2 // OAM Search
        } else if self.dot < 252 {
            3 // Data Transfer
        } else {
            0 // H-Blank
        };

        if old_mode != new_mode {
            stat = (stat & 0xFC) | new_mode;

            let interrupt = match new_mode {
                0 => stat & 0x08 != 0,
                1 => stat & 0x10 != 0,
                2 => stat & 0x20 != 0,
                _ => false,
            };
            if interrupt {
                let ifr = bus.read(0xFF0F) | 0x02;
                bus.write(0xFF0F, ifr);
            }

            if new_mode == 0 && self.ly < 144 {
                self.render_scanline(bus, lcdc);
            }
        }
        bus.write(0xFF41, stat);
    }
}

impl Ppu {
    fn render_scanline<B: MemoryBus>(&mut self, bus: &B, lcdc: u8) {
        let scx  = bus.read(0xFF43);
        let scy  = bus.read(0xFF42);
        let wx   = bus.read(0xFF4B).wrapping_sub(7);
        let wy   = bus.read(0xFF4A);
        let bgp  = bus.read(0xFF47);

        (0u8..160).for_each(|x| {
            let (win, px, py) = if (lcdc & 0x20 != 0) && self.ly >= wy && x >= wx {
                (true, x - wx, self.ly - wy)
            } else {
                (false, x.wrapping_add(scx), self.ly.wrapping_add(scy))
            };

            let color = self.get_bg_pixel(bus, lcdc, px as u16, py as u16, bgp, win);
            self.set_pixel(x as usize, self.ly as usize, color);
        });

        if lcdc & 0x02 != 0 { self.render_sprites(bus); }
    }

    fn get_bg_pixel<B: MemoryBus>(&self, bus: &B, lcdc: u8, px: u16, py: u16, palette: u8, is_win: bool) -> u8 {
        let map_bit  = if is_win { 0x40 } else { 0x08 };
        let map_base = if lcdc & map_bit != 0 { 0x9C00 } else { 0x9800 };
        let tile_idx = bus.read(map_base + (py / 8) * 32 + (px / 8));

        let tile_addr = if lcdc & 0x10 != 0 {
            0x8000 + (tile_idx as u16 * 16)
        } else {
            (0x9000i32 + (tile_idx as i8 as i32 * 16)) as u16
        };

        let row = tile_addr + (py % 8) * 2;
        let (lo, hi) = (bus.read(row), bus.read(row + 1));
        let bit = 7 - (px % 8);
        let id = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
        (palette >> (id * 2)) & 0x03
    }

    fn render_sprites<B: MemoryBus>(&mut self, bus: &B) {
        let ly   = self.ly;
        let obp0 = bus.read(0xFF48);
        let obp1 = bus.read(0xFF49);

        (0..40usize)
            .filter_map(|i| {
                let base = 0xFE00 + (i as u16 * 4);
                let sy = bus.read(base) as i16 - 16;
                if (ly as i16) < sy || (ly as i16) >= sy + 8 { return None; }
                Some((base, sy))
            })
            .take(10)
            .for_each(|(base, sy)| {
                let sx   = bus.read(base + 1) as i16 - 8;
                let tile = bus.read(base + 2);
                let attr = bus.read(base + 3);
                let pal  = if attr & 0x10 != 0 { obp1 } else { obp0 };
                let mut row = (ly as i16 - sy) as u16;
                if attr & 0x40 != 0 { row = 7 - row; }

                let addr = 0x8000 + (tile as u16 * 16) + (row * 2);
                let (lo, hi) = (bus.read(addr), bus.read(addr + 1));

                (0..8i16).for_each(|px| {
                    let tx = sx + px;
                    if tx < 0 || tx >= 160 { return; }
                    let bit = if attr & 0x20 != 0 { px } else { 7 - px } as u8;
                    let id  = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);

                    if id == 0 { return; }

                    if attr & 0x80 != 0 {
                        let i = (ly as usize * 160 + tx as usize) * 4;
                        if self.framebuffer[i] != 0xFF { return; }
                    }

                    let shade = self.shade((pal >> (id * 2)) & 0x03);
                    self.set_raw(tx as usize, ly as usize, shade);
                });
            });
    }

    #[inline(always)]
    fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        self.set_raw(x, y, self.shade(color));
    }

    #[inline(always)]
    fn set_raw(&mut self, x: usize, y: usize, shade: u8) {
        let i = (y * 160 + x) * 4;
        self.framebuffer[i..i+3].fill(shade);
        self.framebuffer[i+3] = 0xFF;
    }

    fn shade(&self, color: u8) -> u8 {
        match color { 0 => 0xFF, 1 => 0xAA, 2 => 0x55, _ => 0x00 }
    }
}
