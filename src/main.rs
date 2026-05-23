use minifb::{Key, KeyRepeat, Window, WindowOptions};
use pokegameboy::{cpu::Cpu, mmu::{Mmu, Tickable}, ppu::Ppu, MAX_FRAME_CYCLES};

const DPAD: [(Key, u8); 4] = [(Key::Down,0x08),(Key::Up,0x04),(Key::Left,0x02),(Key::Right,0x01)];
const BTNS: [(Key, u8); 4] = [(Key::Enter,0x08),(Key::S,0x04),(Key::B,0x02),(Key::A,0x01)];

trait Steppable {
    fn step(&mut self) -> u32;
    fn run_frame(&mut self) {
        let mut c = 0;
        while c < MAX_FRAME_CYCLES { c += self.step(); }
    }
}

struct Emulator { cpu: Cpu, ppu: Ppu, mmu: Mmu, div: u32, timer: u32, pub paused: bool }

impl Emulator {
    fn new(rom: Vec<u8>) -> Self {
        let mut mmu = Mmu::new(rom, vec![0u8; 0x8000]);
        if let Ok(s) = std::fs::read("rom.sav") { mmu.load_save_data(s); }
        Self { cpu: Cpu::new(), ppu: Ppu::new(), mmu, div: 0, timer: 0, paused: false }
    }
    fn save(&self) { std::fs::write("rom.sav", self.mmu.get_save_data()).ok(); }
    fn debug_line(&self) {
        println!("PC:{:04X} AF:{:04X} BC:{:04X} DE:{:04X} HL:{:04X} SP:{:04X} LY:{:02X}",
            self.cpu.regs.pc, self.cpu.regs.get_af(), self.cpu.regs.get_bc(),
            self.cpu.regs.get_de(), self.cpu.regs.get_hl(), self.cpu.regs.sp, self.mmu.read(0xFF44));
    }
}

impl Steppable for Emulator {
    fn step(&mut self) -> u32 {
        let s = self.cpu.step(&mut self.mmu);
        self.ppu.tick(s, &mut self.mmu);
        self.div += s;
        if self.div >= 256 { self.div -= 256; self.mmu.io[0x04] = self.mmu.io[0x04].wrapping_add(1); }
        let tac = self.mmu.read(0xFF07);
        if tac & 0x04 != 0 {
            self.timer += s;
            let th = match tac & 0x03 { 0=>1024, 1=>16, 2=>64, _=>256 };
            while self.timer >= th {
                self.timer -= th;
                let tima = self.mmu.read(0xFF05);
                if tima == 0xFF { self.mmu.write(0xFF05, self.mmu.read(0xFF06)); self.mmu.write(0xFF0F, self.mmu.read(0xFF0F)|0x04); }
                else            { self.mmu.write(0xFF05, tima+1); }
            }
        }
        s
    }
}

fn make_window() -> Window {
    let mut w = Window::new("PokéGB", 160*4, 144*4, WindowOptions::default()).unwrap();
    w.limit_update_rate(Some(std::time::Duration::from_micros(16_742)));
    w
}

fn handle_input(w: &Window, emu: &mut Emulator) {
    if w.is_key_pressed(Key::Space, KeyRepeat::No) {
        emu.paused = !emu.paused;
        if emu.paused { println!("\n── PAUSED  [N] step  [Space] resume ──"); }
    }
    if w.is_key_pressed(Key::F5, KeyRepeat::No) { emu.save(); println!("saved → rom.sav"); }
    if emu.paused && w.is_key_pressed(Key::N, KeyRepeat::Yes) { emu.step(); emu.debug_line(); return; }
    if emu.paused { return; }

    let d = DPAD.iter().fold(0x0Fu8, |a,&(k,b)| if w.is_key_down(k) { a & !b } else { a });
    let b = BTNS.iter().fold(0x0Fu8, |a,&(k,b)| if w.is_key_down(k) { a & !b } else { a });
    let sel = emu.mmu.io[0x00] & 0x30;
    let joyp = 0x0Fu8 & (if sel&0x10==0 { d } else { 0x0F }) & (if sel&0x20==0 { b } else { 0x0F });
    if (emu.mmu.prev_joyp & !joyp) & 0x0F != 0 { emu.mmu.write(0xFF0F, emu.mmu.read(0xFF0F)|0x10); }
    emu.mmu.dpad = d; emu.mmu.buttons = b; emu.mmu.prev_joyp = joyp;
}

fn render(fb: &mut [u32], ppu: &Ppu) {
    const W: usize = 160; const SC: usize = 4;
    (0..144*W).for_each(|i| {
        let p = i * 4;
        let c = (ppu.framebuffer[p] as u32)<<16 | (ppu.framebuffer[p+1] as u32)<<8 | ppu.framebuffer[p+2] as u32;
        let row = (i/W)*SC*W*SC + (i%W)*SC;
        (0..SC).for_each(|dy| fb[row+dy*W*SC..row+dy*W*SC+SC].fill(c));
    });
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("rom.gb");
    let rom  = std::fs::read(path).unwrap_or_else(|_| { eprintln!("error: {path} not found"); std::process::exit(1); });

    let mut emu    = Emulator::new(rom);
    let mut fb     = vec![0u32; 160*4*144*4];
    let mut window = make_window();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        handle_input(&window, &mut emu);
        if !emu.paused { emu.run_frame(); }
        render(&mut fb, &emu.ppu);
        window.update_with_buffer(&fb, 160*4, 144*4).unwrap();
    }
    emu.save();
}
