#[cfg(not(target_arch = "wasm32"))]
use minifb::{Key, KeyRepeat, Window, WindowOptions};
#[cfg(not(target_arch = "wasm32"))]
use pokegameboy::{cpu::Cpu, mmu::{Mmu, Tickable}, ppu::Ppu, MAX_FRAME_CYCLES};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rom_path = args.get(1).map(|s| s.as_str()).unwrap_or("*.gb");
    let rom = std::fs::read(rom_path).unwrap_or_else(|_| { eprintln!("error: {} not found", rom_path); std::process::exit(1); });

    let mut mmu = Mmu::new(rom, vec![0u8; 0x8000]);
    if let Ok(save) = std::fs::read("*.sav") { mmu.load_save_data(save); }

    let (mut cpu, mut ppu) = (Cpu::new(), Ppu::new());
    let (w, h, sc) = (160usize, 144usize, 4usize);
    let mut window = Window::new("PokéGB", w * sc, h * sc, WindowOptions::default()).unwrap();
    window.limit_update_rate(Some(std::time::Duration::from_micros(16_742)));

    let mut fb       = vec![0u32; w * sc * h * sc];
    let mut div_acc  = 0u32;
    let mut timer_acc = 0u32;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::F5, KeyRepeat::No) {
            std::fs::write("rom.sav", mmu.get_save_data()).ok();
        }

        update_joypad(&window, &mut mmu);

        let mut cycles = 0;
        while cycles < MAX_FRAME_CYCLES {
            let s = cpu.step(&mut mmu);
            ppu.tick(s, &mut mmu);

            div_acc += s;
            if div_acc >= 256 { div_acc -= 256; mmu.io[0x04] = mmu.io[0x04].wrapping_add(1); }

            let tac = mmu.read(0xFF07);
            if tac & 0x04 != 0 {
                timer_acc += s;
                let thresh = match tac & 0x03 { 0 => 1024, 1 => 16, 2 => 64, _ => 256 };
                while timer_acc >= thresh {
                    timer_acc -= thresh;
                    let tima = mmu.read(0xFF05);
                    if tima == 0xFF { mmu.write(0xFF05, mmu.read(0xFF06)); mmu.write(0xFF0F, mmu.read(0xFF0F) | 0x04); }
                    else            { mmu.write(0xFF05, tima + 1); }
                }
            }
            cycles += s;
        }

        for y in 0..h {
            for x in 0..w {
                let i = (y * 160 + x) * 4;
                let c = (ppu.framebuffer[i] as u32) << 16
                      | (ppu.framebuffer[i+1] as u32) << 8
                      |  ppu.framebuffer[i+2] as u32;
                for dy in 0..sc {
                    let start = (y * sc + dy) * w * sc + x * sc;
                    fb[start..start+sc].fill(c);
                }
            }
        }
        window.update_with_buffer(&fb, w * sc, h * sc).unwrap();
    }

    std::fs::write("*.sav", mmu.get_save_data()).ok();
}

#[cfg(not(target_arch = "wasm32"))]
fn update_joypad(w: &Window, m: &mut Mmu) {
    let mut d = 0x0Fu8;
    let mut b = 0x0Fu8;
    if w.is_key_down(Key::Down)  { d &= !0x08; }
    if w.is_key_down(Key::Up)    { d &= !0x04; }
    if w.is_key_down(Key::Left)  { d &= !0x02; }
    if w.is_key_down(Key::Right) { d &= !0x01; }
    if w.is_key_down(Key::Enter) { b &= !0x08; }
    if w.is_key_down(Key::S)     { b &= !0x04; }
    if w.is_key_down(Key::B)     { b &= !0x02; }
    if w.is_key_down(Key::A)     { b &= !0x01; }

    let select = m.io[0x00] & 0x30;
    let mut joyp = 0x0F;
    if select & 0x10 == 0 { joyp &= d; }
    if select & 0x20 == 0 { joyp &= b; }
    if (m.prev_joyp & !joyp) & 0x0F != 0 { m.write(0xFF0F, m.read(0xFF0F) | 0x10); }
    m.dpad = d; m.buttons = b; m.prev_joyp = joyp;
}
