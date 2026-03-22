use crate::mmu::Mmu;
use crate::registers::Registers;

pub struct Cpu {
    pub regs: Registers,
    pub halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self { regs: Registers::new(), halted: false }
    }

    pub fn debug_print(&self, mmu: &Mmu) {
        let pc = self.regs.pc;
        let op = mmu.read(pc);
        let if_reg = mmu.read(0xFF0F);
        let ie_reg = mmu.read(0xFFFF);
        let joyp = mmu.read(0xFF00);
        let ly = mmu.read(0xFF44);

        println!("--- [PC: {:#06X} OP: {:#04X}] ---", pc, op);
        println!("REG | AF: {:#06X} BC: {:#06X} DE: {:#06X} HL: {:#06X}", self.regs.get_af(), self.regs.get_bc(), self.regs.get_de(), self.regs.get_hl());
        println!("SYS | IF: {:#04X} IE: {:#04X} LY: {:#04X} JOYP: {:#04X}", if_reg, ie_reg, ly, joyp);
        println!("---------------------------------");

        use std::io::{self, Write};
        io::stdout().flush().unwrap();
    }

    // --- Fetch ---
    fn fetch8(&mut self, mmu: &Mmu) -> u8 {
        let v = mmu.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        v
    }

    fn fetch16(&mut self, mmu: &Mmu) -> u16 {
        let lo = self.fetch8(mmu) as u16;
        let hi = self.fetch8(mmu) as u16;
        hi << 8 | lo
    }

    // --- Stack ---
    pub fn push16(&mut self, mmu: &mut Mmu, val: u16) {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        mmu.write(self.regs.sp, (val >> 8) as u8);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        mmu.write(self.regs.sp, (val & 0xFF) as u8);
    }

    pub fn pop16(&mut self, mmu: &mut Mmu) -> u16 {
        let low = mmu.read(self.regs.sp) as u16;
        self.regs.sp = self.regs.sp.wrapping_add(1);
        let high = mmu.read(self.regs.sp) as u16;
        self.regs.sp = self.regs.sp.wrapping_add(1);
        (high << 8) | low
    }

    // --- r8 helpers (B C D E H L (HL) A) ---
    fn read_r8(&self, idx: u8, mmu: &Mmu) -> u8 {
        match idx {
            0 => self.regs.b,
            1 => self.regs.c,
            2 => self.regs.d,
            3 => self.regs.e,
            4 => self.regs.h,
            5 => self.regs.l,
            6 => mmu.read(self.regs.get_hl()),
            7 => self.regs.a,
            _ => unreachable!(),
        }
    }

    fn write_r8(&mut self, idx: u8, val: u8, mmu: &mut Mmu) {
        match idx {
            0 => self.regs.b = val,
            1 => self.regs.c = val,
            2 => self.regs.d = val,
            3 => self.regs.e = val,
            4 => self.regs.h = val,
            5 => self.regs.l = val,
            6 => mmu.write(self.regs.get_hl(), val),
            7 => self.regs.a = val,
            _ => unreachable!(),
        }
    }

    // --- Main step ---
    pub fn step(&mut self, mmu: &mut Mmu) -> u32 {
        let triggered = mmu.read(0xFF0F) & mmu.read(0xFFFF) & 0x1F;
        if triggered != 0 {
            self.halted = false;
            if self.regs.ime {
                self.regs.ime = false;

                let bit = triggered.trailing_zeros() as u8;
                let mut if_reg = mmu.read(0xFF0F);
                if_reg &= !(1 << bit);
                mmu.write(0xFF0F, if_reg);

                self.push16(mmu, self.regs.pc);
                self.regs.pc = 0x0040 + (bit as u16 * 8);
                return 20;
            }
        }

        if self.halted {
            return 4;
        }

        let op = self.fetch8(mmu);
        self.execute(op, mmu)
    }

    fn execute(&mut self, op: u8, mmu: &mut Mmu) -> u32 {
        match op {
            // --- Misc ---
            0x00 => 4,
            0x76 => { self.halted = true; 4 }

            // --- LD r16, u16 ---
            0x01 => { let v = self.fetch16(mmu); self.regs.set_bc(v); 12 }
            0x11 => { let v = self.fetch16(mmu); self.regs.set_de(v); 12 }
            0x21 => { let v = self.fetch16(mmu); self.regs.set_hl(v); 12 }
            0x31 => { self.regs.sp = self.fetch16(mmu); 12 }

            // --- LD (r16), A ---
            0x02 => { mmu.write(self.regs.get_bc(), self.regs.a); 8 }
            0x12 => { mmu.write(self.regs.get_de(), self.regs.a); 8 }
            0x22 => { let hl = self.regs.get_hl(); mmu.write(hl, self.regs.a); self.regs.set_hl(hl.wrapping_add(1)); 8 }
            0x32 => { let hl = self.regs.get_hl(); mmu.write(hl, self.regs.a); self.regs.set_hl(hl.wrapping_sub(1)); 8 }

            // --- LD A, (r16) ---
            0x0A => { self.regs.a = mmu.read(self.regs.get_bc()); 8 }
            0x1A => { self.regs.a = mmu.read(self.regs.get_de()); 8 }
            0x2A => { let hl = self.regs.get_hl(); self.regs.a = mmu.read(hl); self.regs.set_hl(hl.wrapping_add(1)); 8 }
            0x3A => { let hl = self.regs.get_hl(); self.regs.a = mmu.read(hl); self.regs.set_hl(hl.wrapping_sub(1)); 8 }

            // --- INC r16 ---
            0x03 => { self.regs.set_bc(self.regs.get_bc().wrapping_add(1)); 8 }
            0x13 => { self.regs.set_de(self.regs.get_de().wrapping_add(1)); 8 }
            0x23 => { self.regs.set_hl(self.regs.get_hl().wrapping_add(1)); 8 }
            0x33 => { self.regs.sp = self.regs.sp.wrapping_add(1); 8 }

            // --- DEC r16 ---
            0x0B => { self.regs.set_bc(self.regs.get_bc().wrapping_sub(1)); 8 }
            0x1B => { self.regs.set_de(self.regs.get_de().wrapping_sub(1)); 8 }
            0x2B => { self.regs.set_hl(self.regs.get_hl().wrapping_sub(1)); 8 }
            0x3B => { self.regs.sp = self.regs.sp.wrapping_sub(1); 8 }

            // --- INC r8 ---
            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
                let r = (op >> 3) & 0x07;
                let v = self.read_r8(r, mmu);
                let result = v.wrapping_add(1);
                self.write_r8(r, result, mmu);
                let h = (v & 0x0F) == 0x0F;
                self.regs.set_flags(result == 0, false, h, self.regs.get_flag_c());
                if r == 6 { 12 } else { 4 }
            }

            // --- DEC r8 ---
            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
                let r = (op >> 3) & 0x07;
                let v = self.read_r8(r, mmu);
                let result = v.wrapping_sub(1);
                self.write_r8(r, result, mmu);
                let h = (v & 0x0F) == 0x00;
                self.regs.set_flags(result == 0, true, h, self.regs.get_flag_c());
                if r == 6 { 12 } else { 4 }
            }

            // --- LD r8, u8 ---
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => {
                let r = (op >> 3) & 0x07;
                let v = self.fetch8(mmu);
                self.write_r8(r, v, mmu);
                if r == 6 { 12 } else { 8 }
            }

            // --- LD r8, r8 (0x40-0x7F, skip HALT 0x76) ---
            0x40..=0x75 | 0x77..=0x7F => {
                let dst = (op >> 3) & 0x07;
                let src = op & 0x07;
                let v = self.read_r8(src, mmu);
                self.write_r8(dst, v, mmu);
                if src == 6 || dst == 6 { 8 } else { 4 }
            }

            // --- ADD HL, r16 ---
            0x09 | 0x19 | 0x29 | 0x39 => {
                let r16 = match op {
                    0x09 => self.regs.get_bc(),
                    0x19 => self.regs.get_de(),
                    0x29 => self.regs.get_hl(),
                    _    => self.regs.sp,
                };
                let hl = self.regs.get_hl();
                let result = hl.wrapping_add(r16);
                let h = (hl & 0x0FFF) + (r16 & 0x0FFF) > 0x0FFF;
                let c = (hl as u32) + (r16 as u32) > 0xFFFF;
                self.regs.set_hl(result);
                self.regs.set_flags(self.regs.get_flag_z(), false, h, c);
                8
            }

            // --- ALU A, r8 (0x80-0xBF) ---
            0x80..=0xBF => {
                let src = op & 0x07;
                let operand = self.read_r8(src, mmu);
                let cycles = if src == 6 { 8 } else { 4 };
                self.alu(op, operand);
                cycles
            }

            // --- ALU A, u8 ---
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                let operand = self.fetch8(mmu);
                let equiv = op - 0xC6 + 0x80 + 0x40;
                self.alu(equiv, operand);
                8
            }

            // --- Rotates on A ---
            0x07 => {
                let c = self.regs.a >> 7;
                self.regs.a = self.regs.a.rotate_left(1);
                self.regs.set_flags(false, false, false, c != 0);
                4
            }
            0x0F => {
                let c = self.regs.a & 1;
                self.regs.a = self.regs.a.rotate_right(1);
                self.regs.set_flags(false, false, false, c != 0);
                4
            }
            0x17 => {
                let old_c = self.regs.get_flag_c() as u8;
                let new_c = self.regs.a >> 7;
                self.regs.a = (self.regs.a << 1) | old_c;
                self.regs.set_flags(false, false, false, new_c != 0);
                4
            }
            0x1F => {
                let old_c = self.regs.get_flag_c() as u8;
                let new_c = self.regs.a & 1;
                self.regs.a = (self.regs.a >> 1) | (old_c << 7);
                self.regs.set_flags(false, false, false, new_c != 0);
                4
            }

            // --- DAA ---
            0x27 => {
                let mut a = self.regs.a;
                if !self.regs.get_flag_n() {
                    if self.regs.get_flag_h() || a & 0x0F > 9 { a = a.wrapping_add(0x06); }
                    if self.regs.get_flag_c() || a > 0x9F      { a = a.wrapping_add(0x60); }
                } else {
                    if self.regs.get_flag_h() { a = a.wrapping_sub(0x06); }
                    if self.regs.get_flag_c() { a = a.wrapping_sub(0x60); }
                }
                let c = self.regs.get_flag_c() || (!self.regs.get_flag_n() && self.regs.a > 0x99);
                self.regs.set_flags(a == 0, self.regs.get_flag_n(), false, c);
                self.regs.a = a;
                4
            }

            // --- CPL ---
            0x2F => {
                self.regs.a = !self.regs.a;
                self.regs.set_flags(self.regs.get_flag_z(), true, true, self.regs.get_flag_c());
                4
            }

            // --- SCF / CCF ---
            0x37 => { self.regs.set_flags(self.regs.get_flag_z(), false, false, true); 4 }
            0x3F => { let c = !self.regs.get_flag_c(); self.regs.set_flags(self.regs.get_flag_z(), false, false, c); 4 }

            // --- JR ---
            0x18 => {
                let offset = self.fetch8(mmu) as i8;
                self.regs.pc = self.regs.pc.wrapping_add(offset as u16);
                12
            }
            0x20 => { let o = self.fetch8(mmu) as i8; if !self.regs.get_flag_z() { self.regs.pc = self.regs.pc.wrapping_add(o as u16); 12 } else { 8 } }
            0x28 => { let o = self.fetch8(mmu) as i8; if  self.regs.get_flag_z() { self.regs.pc = self.regs.pc.wrapping_add(o as u16); 12 } else { 8 } }
            0x30 => { let o = self.fetch8(mmu) as i8; if !self.regs.get_flag_c() { self.regs.pc = self.regs.pc.wrapping_add(o as u16); 12 } else { 8 } }
            0x38 => { let o = self.fetch8(mmu) as i8; if  self.regs.get_flag_c() { self.regs.pc = self.regs.pc.wrapping_add(o as u16); 12 } else { 8 } }

            // --- JP ---
            0xC3 => { self.regs.pc = self.fetch16(mmu); 16 }
            0xC2 => { let a = self.fetch16(mmu); if !self.regs.get_flag_z() { self.regs.pc = a; 16 } else { 12 } }
            0xCA => { let a = self.fetch16(mmu); if  self.regs.get_flag_z() { self.regs.pc = a; 16 } else { 12 } }
            0xD2 => { let a = self.fetch16(mmu); if !self.regs.get_flag_c() { self.regs.pc = a; 16 } else { 12 } }
            0xDA => { let a = self.fetch16(mmu); if  self.regs.get_flag_c() { self.regs.pc = a; 16 } else { 12 } }
            0xE9 => { self.regs.pc = self.regs.get_hl(); 4 }

            // --- CALL ---
            0xCD => { let a = self.fetch16(mmu); self.push16(mmu, self.regs.pc); self.regs.pc = a; 24 }
            0xC4 => { let a = self.fetch16(mmu); if !self.regs.get_flag_z() { self.push16(mmu, self.regs.pc); self.regs.pc = a; 24 } else { 12 } }
            0xCC => { let a = self.fetch16(mmu); if  self.regs.get_flag_z() { self.push16(mmu, self.regs.pc); self.regs.pc = a; 24 } else { 12 } }
            0xD4 => { let a = self.fetch16(mmu); if !self.regs.get_flag_c() { self.push16(mmu, self.regs.pc); self.regs.pc = a; 24 } else { 12 } }
            0xDC => { let a = self.fetch16(mmu); if  self.regs.get_flag_c() { self.push16(mmu, self.regs.pc); self.regs.pc = a; 24 } else { 12 } }

            // --- RET ---
            0xC9 => { self.regs.pc = self.pop16(mmu); 16 }
            0xD9 => { self.regs.pc = self.pop16(mmu); self.regs.ime = true; 16 }
            0xC0 => { if !self.regs.get_flag_z() { self.regs.pc = self.pop16(mmu); 20 } else { 8 } }
            0xC8 => { if  self.regs.get_flag_z() { self.regs.pc = self.pop16(mmu); 20 } else { 8 } }
            0xD0 => { if !self.regs.get_flag_c() { self.regs.pc = self.pop16(mmu); 20 } else { 8 } }
            0xD8 => { if  self.regs.get_flag_c() { self.regs.pc = self.pop16(mmu); 20 } else { 8 } }

            // --- PUSH / POP ---
            0xC5 => { let v = self.regs.get_bc(); self.push16(mmu, v); 16 }
            0xD5 => { let v = self.regs.get_de(); self.push16(mmu, v); 16 }
            0xE5 => { let v = self.regs.get_hl(); self.push16(mmu, v); 16 }
            0xF5 => { let v = self.regs.get_af(); self.push16(mmu, v); 16 }

            0xC1 => { let v = self.pop16(mmu); self.regs.set_bc(v); 12 }
            0xD1 => { let v = self.pop16(mmu); self.regs.set_de(v); 12 }
            0xE1 => { let v = self.pop16(mmu); self.regs.set_hl(v); 12 }
            0xF1 => { let v = self.pop16(mmu); self.regs.set_af(v); 12 }

            // --- RST ---
            0xC7 => { self.push16(mmu, self.regs.pc); self.regs.pc = 0x00; 16 }
            0xCF => { self.push16(mmu, self.regs.pc); self.regs.pc = 0x08; 16 }
            0xD7 => { self.push16(mmu, self.regs.pc); self.regs.pc = 0x10; 16 }
            0xDF => { self.push16(mmu, self.regs.pc); self.regs.pc = 0x18; 16 }
            0xE7 => { self.push16(mmu, self.regs.pc); self.regs.pc = 0x20; 16 }
            0xEF => { self.push16(mmu, self.regs.pc); self.regs.pc = 0x28; 16 }
            0xF7 => { self.push16(mmu, self.regs.pc); self.regs.pc = 0x30; 16 }
            0xFF => { self.push16(mmu, self.regs.pc); self.regs.pc = 0x38; 16 }

            // --- I/O ---
            0xE0 => { let a = 0xFF00 | self.fetch8(mmu) as u16; mmu.write(a, self.regs.a); 12 }
            0xF0 => { let a = 0xFF00 | self.fetch8(mmu) as u16; self.regs.a = mmu.read(a); 12 }
            0xE2 => { mmu.write(0xFF00 | self.regs.c as u16, self.regs.a); 8 }
            0xF2 => { self.regs.a = mmu.read(0xFF00 | self.regs.c as u16); 8 }
            0xEA => { let a = self.fetch16(mmu); mmu.write(a, self.regs.a); 16 }
            0xFA => { let a = self.fetch16(mmu); self.regs.a = mmu.read(a); 16 }

            // --- SP ops ---
            0xE8 => {
                let offset = self.fetch8(mmu) as i8 as i16;
                let sp = self.regs.sp as i16;
                let result = sp.wrapping_add(offset) as u16;
                let h = ((self.regs.sp ^ offset as u16 ^ result) & 0x10) != 0;
                let c = ((self.regs.sp ^ offset as u16 ^ result) & 0x100) != 0;
                self.regs.set_flags(false, false, h, c);
                self.regs.sp = result;
                16
            }
            0xF8 => {
                let offset = self.fetch8(mmu) as i8 as i16;
                let sp = self.regs.sp as i16;
                let result = sp.wrapping_add(offset) as u16;
                let h = ((self.regs.sp ^ offset as u16 ^ result) & 0x10) != 0;
                let c = ((self.regs.sp ^ offset as u16 ^ result) & 0x100) != 0;
                self.regs.set_flags(false, false, h, c);
                self.regs.set_hl(result);
                12
            }
            0xF9 => { self.regs.sp = self.regs.get_hl(); 8 }

            // --- DI / EI ---
            0xF3 => { self.regs.ime = false; 4 }
            0xFB => { self.regs.ime = true;  4 }

            // --- CB prefix ---
            0xCB => {
                let cb = self.fetch8(mmu);
                self.execute_cb(cb, mmu)
            }

            op => {
                eprintln!("Unimplemented opcode: 0x{:02X} at PC=0x{:04X}", op, self.regs.pc - 1);
                4
            }
        }
    }

    fn alu(&mut self, op: u8, operand: u8) {
        let kind = (op >> 3) & 0x07;
        match kind {
            0 => {
                let (r, c) = self.regs.a.overflowing_add(operand);
                let h = (self.regs.a & 0xF) + (operand & 0xF) > 0xF;
                self.regs.set_flags(r == 0, false, h, c);
                self.regs.a = r;
            }
            1 => {
                let cy = self.regs.get_flag_c() as u8;
                let r = self.regs.a.wrapping_add(operand).wrapping_add(cy);
                let h = (self.regs.a & 0xF) + (operand & 0xF) + cy > 0xF;
                let c = (self.regs.a as u16) + (operand as u16) + (cy as u16) > 0xFF;
                self.regs.set_flags(r == 0, false, h, c);
                self.regs.a = r;
            }
            2 => {
                let (r, c) = self.regs.a.overflowing_sub(operand);
                let h = (self.regs.a & 0xF) < (operand & 0xF);
                self.regs.set_flags(r == 0, true, h, c);
                self.regs.a = r;
            }
            3 => {
                let cy = self.regs.get_flag_c() as u8;
                let r = self.regs.a.wrapping_sub(operand).wrapping_sub(cy);
                let h = (self.regs.a & 0xF) < (operand & 0xF) + cy;
                let c = (self.regs.a as u16) < (operand as u16) + (cy as u16);
                self.regs.set_flags(r == 0, true, h, c);
                self.regs.a = r;
            }
            4 => {
                self.regs.a &= operand;
                self.regs.set_flags(self.regs.a == 0, false, true, false);
            }
            5 => {
                self.regs.a ^= operand;
                self.regs.set_flags(self.regs.a == 0, false, false, false);
            }
            6 => {
                self.regs.a |= operand;
                self.regs.set_flags(self.regs.a == 0, false, false, false);
            }
            7 => {
                let (r, c) = self.regs.a.overflowing_sub(operand);
                let h = (self.regs.a & 0xF) < (operand & 0xF);
                self.regs.set_flags(r == 0, true, h, c);
            }
            _ => unreachable!()
        }
    }

    fn execute_cb(&mut self, op: u8, mmu: &mut Mmu) -> u32 {
        let r = op & 0x07;
        let v = self.read_r8(r, mmu);
        let bit = (op >> 3) & 0x07;
        let cycles = if r == 6 { 16 } else { 8 };

        let result = match op >> 6 {
            0 => {
                match bit {
                    0 => { let c = v >> 7; let r = v.rotate_left(1);  self.regs.set_flags(r == 0, false, false, c != 0); r } // RLC
                    1 => { let c = v & 1;  let r = v.rotate_right(1); self.regs.set_flags(r == 0, false, false, c != 0); r } // RRC
                    2 => { let c = v >> 7; let r = (v << 1) | self.regs.get_flag_c() as u8; self.regs.set_flags(r == 0, false, false, c != 0); r } // RL
                    3 => { let c = v & 1;  let r = (v >> 1) | ((self.regs.get_flag_c() as u8) << 7); self.regs.set_flags(r == 0, false, false, c != 0); r } // RR
                    4 => { let c = v >> 7; let r = v << 1;             self.regs.set_flags(r == 0, false, false, c != 0); r } // SLA
                    5 => { let c = v & 1;  let r = (v >> 1) | (v & 0x80); self.regs.set_flags(r == 0, false, false, c != 0); r } // SRA
                    6 => { let r = v.rotate_left(4); self.regs.set_flags(r == 0, false, false, false); r } // SWAP
                    7 => { let c = v & 1;  let r = v >> 1;             self.regs.set_flags(r == 0, false, false, c != 0); r } // SRL
                    _ => unreachable!()
                }
            }
            1 => {
                self.regs.set_flags(v & (1 << bit) == 0, false, true, self.regs.get_flag_c());
                return if r == 6 { 12 } else { 8 };
            }
            2 => v & !(1 << bit),
            3 => v | (1 << bit),
            _ => unreachable!()
        };

        self.write_r8(r, result, mmu);
        cycles
    }
}