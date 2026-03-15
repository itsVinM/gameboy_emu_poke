use minifb::{Key, KeyRepeat, Window, WindowOptions};
use pokegameboy::{cpu::Cpu, mmu::Mmu, ppu::Ppu, MAX_FRAME_CYCLES};

fn main() {
    let rom = std::fs::read("rom.gb").expect("rom.gb missing");
    let mut mmu = Mmu::new(rom, vec![0u8; 0x8000]);
    let (mut cpu, mut ppu) = (Cpu::new(), Ppu::new());

    let (w, h, sc) = (160, 144, 4);
    let mut window = Window::new("PokéGB Principal Build", w * sc, h * sc, WindowOptions::default()).unwrap();
    window.limit_update_rate(Some(std::time::Duration::from_micros(16666)));

    let mut fb = vec![0u32; (w * sc) * (h * sc)];
    let mut paused = false;
    
    // Internal hardware counters
    let mut div_acc: u32 = 0;
    let mut timer_acc: u32 = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::Space, KeyRepeat::No) { paused = !paused; }

        if !paused {
            update_joypad(&window, &mut mmu);
            
            let mut frame_cycles = 0;
            while frame_cycles < MAX_FRAME_CYCLES {
                let s = cpu.step(&mut mmu);
                ppu.tick(s, &mut mmu);
                
                // --- 1. THE HEARTBEAT (DIV) ---
                div_acc += s;
                if div_acc >= 256 {
                    div_acc -= 256;
                    // Directly incrementing IO space to ensure game logic sees the change
                    mmu.io[0x04] = mmu.io[0x04].wrapping_add(1);
                }

                // --- 2. THE DIALOGUE DRIVER (TIMER) ---
                let tac = mmu.read(0xFF07);
                if tac & 0x04 != 0 {
                    timer_acc += s;
                    let threshold = match tac & 0x03 {
                        0x00 => 1024, 0x01 => 16, 0x02 => 64, 0x03 => 256,
                        _ => 1024,
                    };

                    while timer_acc >= threshold {
                        timer_acc -= threshold;
                        let tima = mmu.read(0xFF05);
                        if tima == 0xFF {
                            mmu.write(0xFF05, mmu.read(0xFF06)); // Reload TMA
                            let if_val = mmu.read(0xFF0F);
                            mmu.write(0xFF0F, if_val | 0x04); // Request Timer IRQ
                        } else {
                            mmu.write(0xFF05, tima + 1);
                        }
                    }
                }
                frame_cycles += s;
            }
        }

        // --- PERFORMANCE TELEMETRY ---
        window.set_title(&format!(
            "PokéGB | PC:{:04X} | LY:{:02X} | IF:{:02X} | IE:{:02X}",
            cpu.pc, mmu.read(0xFF44), mmu.read(0xFF0F), mmu.read(0xFFFF)
        ));

        render_frame(&mut fb, &ppu, w, h, sc);
        window.update_with_buffer(&fb, w * sc, h * sc).unwrap();
    }
}

fn render_frame(fb: &mut [u32], ppu: &Ppu, w: usize, h: usize, sc: usize) {
    for y in 0..h {
        let src_row = y * 640;
        for x in 0..w {
            let i = src_row + (x * 4);
            let color = ((ppu.framebuffer[i] as u32) << 16) | 
                        ((ppu.framebuffer[i+1] as u32) << 8) | 
                         (ppu.framebuffer[i+2] as u32);
            
            for dy in 0..sc {
                let start = ((y * sc + dy) * (w * sc)) + (x * sc);
                fb[start..start + sc].fill(color);
            }
        }
    }
}

fn update_joypad(w: &Window, m: &mut Mmu) {
    let (mut d, mut b) = (0x0F, 0x0F);
    if w.is_key_down(Key::Down)  { d &= !0x8; } if w.is_key_down(Key::Up)    { d &= !0x4; }
    if w.is_key_down(Key::Left)  { d &= !0x2; } if w.is_key_down(Key::Right) { d &= !0x1; }
    if w.is_key_down(Key::Enter) { b &= !0x8; } if w.is_key_down(Key::S)     { b &= !0x4; }
    if w.is_key_down(Key::X)     { b &= !0x2; } if w.is_key_down(Key::Z)     { b &= !0x1; }

    m.dpad = d;
    m.buttons = b;
    
    // Joypad IRQ: Essential for Oak's text-advance polling
    if (d | b) != 0x0F {
        let if_val = m.read(0xFF0F);
        m.write(0xFF0F, if_val | 0x10);
    }
}