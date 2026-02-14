use crate::mmu::Mmu;

pub struct Cpu {
    pub a:  u8,
    pub f:  u8,
    pub b:  u8, pub c: u8,
    pub d:  u8, pub e: u8,
    pub h:  u8, pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime:    bool,
    pub halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        // Post-boot register state for DMG
        Self {
            a: 0x01, f: 0xB0,
            b: 0x00, c: 0x13,
            d: 0x00, e: 0xD8,
            h: 0x01, l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
            ime: false,
            halted: false,
        }
    }

    // --- 16-bit register pairs ---
    pub fn bc(&self) -> u16 { (self.b as u16) << 8 | self.c as u16 }
    pub fn de(&self) -> u16 { (self.d as u16) << 8 | self.e as u16 }
    pub fn hl(&self) -> u16 { (self.h as u16) << 8 | self.l as u16 }
    pub fn af(&self) -> u16 { (self.a as u16) << 8 | self.f as u16 }

    pub fn set_bc(&mut self, v: u16) { self.b = (v >> 8) as u8; self.c = v as u8; }
    pub fn set_de(&mut self, v: u16) { self.d = (v >> 8) as u8; self.e = v as u8; }
    pub fn set_hl(&mut self, v: u16) { self.h = (v >> 8) as u8; self.l = v as u8; }
    pub fn set_af(&mut self, v: u16) { self.a = (v >> 8) as u8; self.f = v as u8 & 0xF0; }

    // --- Flags ---
    pub fn flag_z(&self) -> bool { self.f & 0x80 != 0 }
    pub fn flag_n(&self) -> bool { self.f & 0x40 != 0 }
    pub fn flag_h(&self) -> bool { self.f & 0x20 != 0 }
    pub fn flag_c(&self) -> bool { self.f & 0x10 != 0 }

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.f = (z as u8) << 7
               | (n as u8) << 6
               | (h as u8) << 5
               | (c as u8) << 4;
    }

    // --- Fetch ---
    fn fetch8(&mut self, mmu: &Mmu) -> u8 {
        let v = mmu.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }

    fn fetch16(&mut self, mmu: &Mmu) -> u16 {
        let lo = self.fetch8(mmu) as u16;
        let hi = self.fetch8(mmu) as u16;
        hi << 8 | lo
    }

    // --- Stack ---
    fn push16(&mut self, mmu: &mut Mmu, val: u16) {
        self.sp = self.sp.wrapping_sub(1);
        mmu.write(self.sp, (val >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        mmu.write(self.sp, val as u8);
    }

    fn pop16(&mut self, mmu: &Mmu) -> u16 {
        let lo = mmu.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let hi = mmu.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        hi << 8 | lo
    }

    // --- r8 helpers (B C D E H L (HL) A) ---
    fn read_r8(&self, idx: u8, mmu: &Mmu) -> u8 {
        match idx {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => mmu.read(self.hl()),
            7 => self.a,
            _ => unreachable!(),
        }
    }

    fn write_r8(&mut self, idx: u8, val: u8, mmu: &mut Mmu) {
        match idx {
            0 => self.b = val,
            1 => self.c = val,
            2 => self.d = val,
            3 => self.e = val,
            4 => self.h = val,
            5 => self.l = val,
            6 => mmu.write(self.hl(), val),
            7 => self.a = val,
            _ => unreachable!(),
        }
    }

    // --- Main step ---
    pub fn step(&mut self, mmu: &mut Mmu) -> u32 {
        // Handle interrupts
        let triggered = mmu.io[0x0F] & mmu.ie & 0x1F;
        if triggered != 0 {
            self.halted = false;
            if self.ime {
                self.ime = false;
                let bit = triggered.trailing_zeros() as u8;
                mmu.io[0x0F] &= !(1 << bit);
                let vector: u16 = 0x0040 + (bit as u16) * 8;
                self.push16(mmu, self.pc);
                self.pc = vector;
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
            0x00 => 4, // NOP
            0x76 => { self.halted = true; 4 } // HALT

            // --- LD r16, u16 ---
            0x01 => { let v = self.fetch16(mmu); self.set_bc(v); 12 }
            0x11 => { let v = self.fetch16(mmu); self.set_de(v); 12 }
            0x21 => { let v = self.fetch16(mmu); self.set_hl(v); 12 }
            0x31 => { self.sp = self.fetch16(mmu); 12 }

            // --- LD (r16), A ---
            0x02 => { mmu.write(self.bc(), self.a); 8 }
            0x12 => { mmu.write(self.de(), self.a); 8 }
            0x22 => { let hl = self.hl(); mmu.write(hl, self.a); self.set_hl(hl.wrapping_add(1)); 8 }
            0x32 => { let hl = self.hl(); mmu.write(hl, self.a); self.set_hl(hl.wrapping_sub(1)); 8 }

            // --- LD A, (r16) ---
            0x0A => { self.a = mmu.read(self.bc()); 8 }
            0x1A => { self.a = mmu.read(self.de()); 8 }
            0x2A => { let hl = self.hl(); self.a = mmu.read(hl); self.set_hl(hl.wrapping_add(1)); 8 }
            0x3A => { let hl = self.hl(); self.a = mmu.read(hl); self.set_hl(hl.wrapping_sub(1)); 8 }

            // --- INC r16 ---
            0x03 => { self.set_bc(self.bc().wrapping_add(1)); 8 }
            0x13 => { self.set_de(self.de().wrapping_add(1)); 8 }
            0x23 => { self.set_hl(self.hl().wrapping_add(1)); 8 }
            0x33 => { self.sp = self.sp.wrapping_add(1); 8 }

            // --- DEC r16 ---
            0x0B => { self.set_bc(self.bc().wrapping_sub(1)); 8 }
            0x1B => { self.set_de(self.de().wrapping_sub(1)); 8 }
            0x2B => { self.set_hl(self.hl().wrapping_sub(1)); 8 }
            0x3B => { self.sp = self.sp.wrapping_sub(1); 8 }

            // --- INC r8 ---
            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
                let r = (op >> 3) & 0x07;
                let v = self.read_r8(r, mmu);
                let result = v.wrapping_add(1);
                self.write_r8(r, result, mmu);
                let h = (v & 0x0F) == 0x0F;
                self.set_flags(result == 0, false, h, self.flag_c());
                if r == 6 { 12 } else { 4 }
            }

            // --- DEC r8 ---
            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
                let r = (op >> 3) & 0x07;
                let v = self.read_r8(r, mmu);
                let result = v.wrapping_sub(1);
                self.write_r8(r, result, mmu);
                let h = (v & 0x0F) == 0x00;
                self.set_flags(result == 0, true, h, self.flag_c());
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
                    0x09 => self.bc(),
                    0x19 => self.de(),
                    0x29 => self.hl(),
                    _    => self.sp,
                };
                let hl = self.hl();
                let result = hl.wrapping_add(r16);
                let h = (hl & 0x0FFF) + (r16 & 0x0FFF) > 0x0FFF;
                let c = (hl as u32) + (r16 as u32) > 0xFFFF;
                self.set_hl(result);
                self.set_flags(self.flag_z(), false, h, c);
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

            // --- ALU A, u8 (0xC6, 0xCE, 0xD6, 0xDE, 0xE6, 0xEE, 0xF6, 0xFE) ---
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                let operand = self.fetch8(mmu);
                // Map to equivalent r8 opcode for alu()
                let equiv = op - 0xC6 + 0x80 + 0x40;  // ADD=0x86 zone
                self.alu(equiv, operand);
                8
            }

            // --- Rotates on A ---
            0x07 => { // RLCA
                let c = self.a >> 7;
                self.a = self.a.rotate_left(1);
                self.set_flags(false, false, false, c != 0);
                4
            }
            0x0F => { // RRCA
                let c = self.a & 1;
                self.a = self.a.rotate_right(1);
                self.set_flags(false, false, false, c != 0);
                4
            }
            0x17 => { // RLA
                let old_c = self.flag_c() as u8;
                let new_c = self.a >> 7;
                self.a = (self.a << 1) | old_c;
                self.set_flags(false, false, false, new_c != 0);
                4
            }
            0x1F => { // RRA
                let old_c = self.flag_c() as u8;
                let new_c = self.a & 1;
                self.a = (self.a >> 1) | (old_c << 7);
                self.set_flags(false, false, false, new_c != 0);
                4
            }

            // --- DAA ---
            0x27 => {
                let mut a = self.a;
                if !self.flag_n() {
                    if self.flag_h() || a & 0x0F > 9  { a = a.wrapping_add(0x06); }
                    if self.flag_c() || a > 0x9F       { a = a.wrapping_add(0x60); }
                } else {
                    if self.flag_h() { a = a.wrapping_sub(0x06); }
                    if self.flag_c() { a = a.wrapping_sub(0x60); }
                }
                let c = self.flag_c() || (!self.flag_n() && self.a > 0x99);
                self.set_flags(a == 0, self.flag_n(), false, c);
                self.a = a;
                4
            }

            // --- CPL ---
            0x2F => {
                self.a = !self.a;
                self.set_flags(self.flag_z(), true, true, self.flag_c());
                4
            }

            // --- SCF / CCF ---
            0x37 => { self.set_flags(self.flag_z(), false, false, true);              4 }
            0x3F => { let c = !self.flag_c(); self.set_flags(self.flag_z(), false, false, c); 4 }

            // --- JR ---
            0x18 => {
                let offset = self.fetch8(mmu) as i8;
                self.pc = self.pc.wrapping_add(offset as u16);
                12
            }
            0x20 => { let o = self.fetch8(mmu) as i8; if !self.flag_z() { self.pc = self.pc.wrapping_add(o as u16); 12 } else { 8 } }
            0x28 => { let o = self.fetch8(mmu) as i8; if  self.flag_z() { self.pc = self.pc.wrapping_add(o as u16); 12 } else { 8 } }
            0x30 => { let o = self.fetch8(mmu) as i8; if !self.flag_c() { self.pc = self.pc.wrapping_add(o as u16); 12 } else { 8 } }
            0x38 => { let o = self.fetch8(mmu) as i8; if  self.flag_c() { self.pc = self.pc.wrapping_add(o as u16); 12 } else { 8 } }

            // --- JP ---
            0xC3 => { self.pc = self.fetch16(mmu); 16 }
            0xC2 => { let a = self.fetch16(mmu); if !self.flag_z() { self.pc = a; 16 } else { 12 } }
            0xCA => { let a = self.fetch16(mmu); if  self.flag_z() { self.pc = a; 16 } else { 12 } }
            0xD2 => { let a = self.fetch16(mmu); if !self.flag_c() { self.pc = a; 16 } else { 12 } }
            0xDA => { let a = self.fetch16(mmu); if  self.flag_c() { self.pc = a; 16 } else { 12 } }
            0xE9 => { self.pc = self.hl(); 4 } // JP HL

            // --- CALL ---
            0xCD => { let a = self.fetch16(mmu); self.push16(mmu, self.pc); self.pc = a; 24 }
            0xC4 => { let a = self.fetch16(mmu); if !self.flag_z() { self.push16(mmu, self.pc); self.pc = a; 24 } else { 12 } }
            0xCC => { let a = self.fetch16(mmu); if  self.flag_z() { self.push16(mmu, self.pc); self.pc = a; 24 } else { 12 } }
            0xD4 => { let a = self.fetch16(mmu); if !self.flag_c() { self.push16(mmu, self.pc); self.pc = a; 24 } else { 12 } }
            0xDC => { let a = self.fetch16(mmu); if  self.flag_c() { self.push16(mmu, self.pc); self.pc = a; 24 } else { 12 } }

            // --- RET ---
            0xC9 => { self.pc = self.pop16(mmu); 16 }
            0xD9 => { self.pc = self.pop16(mmu); self.ime = true; 16 } // RETI
            0xC0 => { if !self.flag_z() { self.pc = self.pop16(mmu); 20 } else { 8 } }
            0xC8 => { if  self.flag_z() { self.pc = self.pop16(mmu); 20 } else { 8 } }
            0xD0 => { if !self.flag_c() { self.pc = self.pop16(mmu); 20 } else { 8 } }
            0xD8 => { if  self.flag_c() { self.pc = self.pop16(mmu); 20 } else { 8 } }

            // --- PUSH / POP ---
            0xC5 => { let v = self.bc(); self.push16(mmu, v); 16 }
            0xD5 => { let v = self.de(); self.push16(mmu, v); 16 }
            0xE5 => { let v = self.hl(); self.push16(mmu, v); 16 }
            0xF5 => { let v = self.af(); self.push16(mmu, v); 16 }

            0xC1 => { let v = self.pop16(mmu); self.set_bc(v); 12 }
            0xD1 => { let v = self.pop16(mmu); self.set_de(v); 12 }
            0xE1 => { let v = self.pop16(mmu); self.set_hl(v); 12 }
            0xF1 => { let v = self.pop16(mmu); self.set_af(v); 12 }

            // --- RST ---
            0xC7 => { self.push16(mmu, self.pc); self.pc = 0x00; 16 }
            0xCF => { self.push16(mmu, self.pc); self.pc = 0x08; 16 }
            0xD7 => { self.push16(mmu, self.pc); self.pc = 0x10; 16 }
            0xDF => { self.push16(mmu, self.pc); self.pc = 0x18; 16 }
            0xE7 => { self.push16(mmu, self.pc); self.pc = 0x20; 16 }
            0xEF => { self.push16(mmu, self.pc); self.pc = 0x28; 16 }
            0xF7 => { self.push16(mmu, self.pc); self.pc = 0x30; 16 }
            0xFF => { self.push16(mmu, self.pc); self.pc = 0x38; 16 }

            // --- I/O ---
            0xE0 => { let a = 0xFF00 | self.fetch8(mmu) as u16; mmu.write(a, self.a); 12 }
            0xF0 => { let a = 0xFF00 | self.fetch8(mmu) as u16; self.a = mmu.read(a); 12 }
            0xE2 => { mmu.write(0xFF00 | self.c as u16, self.a); 8 }
            0xF2 => { self.a = mmu.read(0xFF00 | self.c as u16); 8 }
            0xEA => { let a = self.fetch16(mmu); mmu.write(a, self.a); 16 }
            0xFA => { let a = self.fetch16(mmu); self.a = mmu.read(a); 16 }

            // --- SP ops ---
            0xE8 => { // ADD SP, i8
                let offset = self.fetch8(mmu) as i8 as i16;
                let sp = self.sp as i16;
                let result = sp.wrapping_add(offset) as u16;
                let h = ((self.sp ^ offset as u16 ^ result) & 0x10) != 0;
                let c = ((self.sp ^ offset as u16 ^ result) & 0x100) != 0;
                self.set_flags(false, false, h, c);
                self.sp = result;
                16
            }
            0xF8 => { // LD HL, SP+i8
                let offset = self.fetch8(mmu) as i8 as i16;
                let sp = self.sp as i16;
                let result = sp.wrapping_add(offset) as u16;
                let h = ((self.sp ^ offset as u16 ^ result) & 0x10) != 0;
                let c = ((self.sp ^ offset as u16 ^ result) & 0x100) != 0;
                self.set_flags(false, false, h, c);
                self.set_hl(result);
                12
            }
            0xF9 => { self.sp = self.hl(); 8 } // LD SP, HL

            // --- DI / EI ---
            0xF3 => { self.ime = false; 4 }
            0xFB => { self.ime = true;  4 }

            // --- CB prefix ---
            0xCB => {
                let cb = self.fetch8(mmu);
                self.execute_cb(cb, mmu)
            }

            op => {
                eprintln!("Unimplemented opcode: 0x{:02X} at PC=0x{:04X}", op, self.pc - 1);
                4
            }
        }
    }

    // Shared ALU logic — op tells us which operation (ADD/ADC/SUB/SBC/AND/XOR/OR/CP)
    fn alu(&mut self, op: u8, operand: u8) {
        let kind = (op >> 3) & 0x07;
        match kind {
            0 => { // ADD
                let (r, c) = self.a.overflowing_add(operand);
                let h = (self.a & 0xF) + (operand & 0xF) > 0xF;
                self.set_flags(r == 0, false, h, c);
                self.a = r;
            }
            1 => { // ADC
                let cy = self.flag_c() as u8;
                let r = self.a.wrapping_add(operand).wrapping_add(cy);
                let h = (self.a & 0xF) + (operand & 0xF) + cy > 0xF;
                let c = (self.a as u16) + (operand as u16) + (cy as u16) > 0xFF;
                self.set_flags(r == 0, false, h, c);
                self.a = r;
            }
            2 => { // SUB
                let (r, c) = self.a.overflowing_sub(operand);
                let h = (self.a & 0xF) < (operand & 0xF);
                self.set_flags(r == 0, true, h, c);
                self.a = r;
            }
            3 => { // SBC
                let cy = self.flag_c() as u8;
                let r = self.a.wrapping_sub(operand).wrapping_sub(cy);
                let h = (self.a & 0xF) < (operand & 0xF) + cy;
                let c = (self.a as u16) < (operand as u16) + (cy as u16);
                self.set_flags(r == 0, true, h, c);
                self.a = r;
            }
            4 => { // AND
                self.a &= operand;
                self.set_flags(self.a == 0, false, true, false);
            }
            5 => { // XOR
                self.a ^= operand;
                self.set_flags(self.a == 0, false, false, false);
            }
            6 => { // OR
                self.a |= operand;
                self.set_flags(self.a == 0, false, false, false);
            }
            7 => { // CP (compare, like SUB but discard result)
                let (r, c) = self.a.overflowing_sub(operand);
                let h = (self.a & 0xF) < (operand & 0xF);
                self.set_flags(r == 0, true, h, c);
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
            0 => { // Shifts/rotates
                match bit {
                    0 => { let c = v >> 7; let r = v.rotate_left(1);  self.set_flags(r == 0, false, false, c != 0); r } // RLC
                    1 => { let c = v & 1;  let r = v.rotate_right(1); self.set_flags(r == 0, false, false, c != 0); r } // RRC
                    2 => { let c = v >> 7; let r = (v << 1) | self.flag_c() as u8; self.set_flags(r == 0, false, false, c != 0); r } // RL
                    3 => { let c = v & 1;  let r = (v >> 1) | ((self.flag_c() as u8) << 7); self.set_flags(r == 0, false, false, c != 0); r } // RR
                    4 => { let c = v >> 7; let r = v << 1;              self.set_flags(r == 0, false, false, c != 0); r } // SLA
                    5 => { let c = v & 1;  let r = (v >> 1) | (v & 0x80); self.set_flags(r == 0, false, false, c != 0); r } // SRA
                    6 => { let r = v.rotate_left(4); self.set_flags(r == 0, false, false, false); r } // SWAP
                    7 => { let c = v & 1;  let r = v >> 1;              self.set_flags(r == 0, false, false, c != 0); r } // SRL
                    _ => unreachable!()
                }
            }
            1 => { // BIT — only sets flags, doesn't write back
                self.set_flags(v & (1 << bit) == 0, false, true, self.flag_c());
                return if r == 6 { 12 } else { 8 };
            }
            2 => v & !(1 << bit), // RES
            3 => v | (1 << bit),  // SET
            _ => unreachable!()
        };

        self.write_r8(r, result, mmu);
        cycles
    }
}