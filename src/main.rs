use minifb::{Key, Window, WindowOptions};

mod cpu;
mod mmu;
mod ppu;

use cpu::Cpu;
use mmu::Mmu;
use ppu::Ppu;



fn main() {
    let rom = std::fs::read("rom.gb").expect("rom.gb not found");
    let extram = std::fs::read("rom.sav").unwrap_or(vec![0u8; 0x8000]);

    let mut mmu = Mmu::new(rom, extram);
    let mut cpu = Cpu::new();
    let mut ppu = Ppu::new();

    let scale = 3;
    let mut window = Window::new(
        "Game Boy",
        160 * scale,
        144 * scale,
        WindowOptions::default(),
    )
    .expect("Failed to create window");

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let mut fb: Vec<u32> = vec![0u32; 160 * scale * 144 * scale];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        update_joypad(&window, &mut mmu);

        let mut frame_cycles = 0u32;
        while frame_cycles < 70224 {
            let cycles = cpu.step(&mut mmu);
            ppu.tick(cycles, &mut mmu);
            frame_cycles += cycles;
        }

        scale_frame(&ppu.framebuffer, &mut fb, scale);

        window
            .update_with_buffer(&fb, 160 * scale, 144 * scale)
            .unwrap();

        std::fs::write("rom.sav", &mmu.extram).ok();
    }
}

fn scale_frame(src: &[u8], dst: &mut Vec<u32>, scale: usize) {
    for y in 0..144usize {
        for x in 0..160usize {
            let i = (y * 160 + x) * 4;
            let r = src[i]     as u32;
            let g = src[i + 1] as u32;
            let b = src[i + 2] as u32;
            let pixel: u32 = (r << 16) | (g << 8) | b;
            for dy in 0..scale {
                for dx in 0..scale {
                    dst[(y * scale + dy) * 160 * scale + (x * scale + dx)] = pixel;
                }
            }
        }
    }
}

fn update_joypad(window: &Window, mmu: &mut Mmu) {
    let joyp = mmu.io[0x00];

    let dirs: u8 = !(
        (window.is_key_down(Key::Down)  as u8) << 3 |
        (window.is_key_down(Key::Up)    as u8) << 2 |
        (window.is_key_down(Key::Left)  as u8) << 1 |
        (window.is_key_down(Key::Right) as u8)
    ) & 0x0F;

    let btns: u8 = !(
        (window.is_key_down(Key::Enter) as u8) << 3 |
        (window.is_key_down(Key::Space) as u8) << 2 |
        (window.is_key_down(Key::Z)     as u8) << 1 |
        (window.is_key_down(Key::X)     as u8)
    ) & 0x0F;

    let low = if joyp & 0x10 == 0 {
        dirs
    } else if joyp & 0x20 == 0 {
        btns
    } else {
        0x0F
    };

    mmu.io[0x00] = (joyp & 0x30) | low | 0xC0;
}