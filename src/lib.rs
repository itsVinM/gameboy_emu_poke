use wasm_bindgen::prelude::*;

pub mod cpu;
pub mod mmu;
pub mod ppu;
pub mod registers;
pub mod traits;

use cpu::Cpu;
use mmu::Mmu;
use ppu::Ppu;
use traits::Tickable;

pub const MAX_FRAME_CYCLES: u32 = 70224;

#[wasm_bindgen]
pub struct EmulatorState {
    cpu:       Cpu,
    ppu:       Ppu,
    mmu:       Mmu,
    div_acc:   u32,
    timer_acc: u32,
    paused:    bool,
    frames:    u32,
}

#[wasm_bindgen]
impl EmulatorState {
    #[wasm_bindgen(constructor)]
    pub fn new(rom: Vec<u8>) -> Self {
        console_error_panic_hook::set_once();
        Self {
            cpu:       Cpu::new(),
            ppu:       Ppu::new(),
            mmu:       Mmu::new(rom, vec![0u8; 0x8000]),
            div_acc:   0,
            timer_acc: 0,
            paused:    false,
            frames:    0,
        }
    }

    // ── run controls ─────────────────────────────────────────────────────────

    pub fn tick_frame(&mut self) {
        if self.paused { return; }
        let mut cycles = 0;
        while cycles < MAX_FRAME_CYCLES {
            cycles += self.tick_cycle();
        }
        self.frames = self.frames.wrapping_add(1);
    }

    /// Single CPU instruction + proportional PPU/timer — used by the debugger step button.
    pub fn tick_step(&mut self) {
        self.tick_cycle();
    }

    pub fn set_paused(&mut self, p: bool) { self.paused = p; }
    pub fn is_paused(&self) -> bool { self.paused }

    // ── register reads (for the debug panel) ─────────────────────────────────

    pub fn pc(&self)    -> u16 { self.cpu.regs.pc }
    pub fn sp(&self)    -> u16 { self.cpu.regs.sp }
    pub fn af(&self)    -> u16 { self.cpu.regs.get_af() }
    pub fn bc(&self)    -> u16 { self.cpu.regs.get_bc() }
    pub fn de(&self)    -> u16 { self.cpu.regs.get_de() }
    pub fn hl(&self)    -> u16 { self.cpu.regs.get_hl() }
    pub fn flags(&self) -> u8  { self.cpu.regs.f }

    pub fn ly(&self)    -> u8  { self.mmu.read(0xFF44) }
    pub fn if_reg(&self)-> u8  { self.mmu.read(0xFF0F) }
    pub fn ie_reg(&self)-> u8  { self.mmu.read(0xFFFF) }
    pub fn lcdc(&self)  -> u8  { self.mmu.read(0xFF40) }
    pub fn stat(&self)  -> u8  { self.mmu.read(0xFF41) }
    pub fn halted(&self)-> bool { self.cpu.halted }
    pub fn frames(&self)-> u32  { self.frames }
    pub fn ime(&self)   -> bool { self.cpu.regs.ime }

    // ── save / load ───────────────────────────────────────────────────────────

    pub fn save_wasm(&self) -> Vec<u8> { self.mmu.get_save_data() }
    pub fn load_save_wasm(&mut self, data: Vec<u8>) { self.mmu.load_save_data(data); }

    // ── joypad ────────────────────────────────────────────────────────────────

    pub fn update_joypad(&mut self, d_pad: u8, buttons: u8) {
        let select = self.mmu.io[0x00] & 0x30;
        let mut current_joyp = 0x0F;
        if (select & 0x10) == 0 { current_joyp &= d_pad; }
        if (select & 0x20) == 0 { current_joyp &= buttons; }
        if (self.mmu.prev_joyp & !current_joyp) & 0x0F != 0 {
            let if_val = self.mmu.read(0xFF0F);
            self.mmu.write(0xFF0F, if_val | 0x10);
        }
        self.mmu.dpad     = d_pad;
        self.mmu.buttons  = buttons;
        self.mmu.prev_joyp = current_joyp;
    }

    // ── framebuffer ───────────────────────────────────────────────────────────

    pub fn framebuffer_ptr(&self) -> *const u8 { self.ppu.framebuffer.as_ptr() }
}

impl EmulatorState {
    /// One CPU instruction + PPU tick + DIV/timer update. Returns cycles consumed.
    fn tick_cycle(&mut self) -> u32 {
        let s = self.cpu.step(&mut self.mmu);
        self.ppu.tick(s, &mut self.mmu);

        self.div_acc += s;
        if self.div_acc >= 256 {
            self.div_acc -= 256;
            self.mmu.io[0x04] = self.mmu.io[0x04].wrapping_add(1);
        }

        let tac = self.mmu.read(0xFF07);
        if tac & 0x04 != 0 {
            self.timer_acc += s;
            let threshold = match tac & 0x03 {
                0x00 => 1024, 0x01 => 16, 0x02 => 64, 0x03 => 256, _ => 1024,
            };
            while self.timer_acc >= threshold {
                self.timer_acc -= threshold;
                let tima = self.mmu.read(0xFF05);
                if tima == 0xFF {
                    self.mmu.write(0xFF05, self.mmu.read(0xFF06));
                    let ifv = self.mmu.read(0xFF0F);
                    self.mmu.write(0xFF0F, ifv | 0x04);
                } else {
                    self.mmu.write(0xFF05, tima + 1);
                }
            }
        }
        s
    }
}
