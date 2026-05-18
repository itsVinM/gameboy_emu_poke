use pokegameboy::cpu::Cpu;
use pokegameboy::traits::MemoryBus;

struct FlatBus([u8; 0x10000]);

impl FlatBus {
    fn new() -> Self { Self([0; 0x10000]) }
    fn load(&mut self, addr: u16, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            self.0[addr as usize + i] = b;
        }
    }
}

impl MemoryBus for FlatBus {
    fn read(&self, addr: u16) -> u8 { self.0[addr as usize] }
    fn write(&mut self, addr: u16, val: u8) { self.0[addr as usize] = val; }
}

fn cpu_at(pc: u16) -> Cpu {
    let mut cpu = Cpu::new();
    cpu.regs.pc = pc;
    cpu
}

// ── misc ─────────────────────────────────────────────────────────────────────

#[test]
fn nop_advances_pc_and_costs_4_cycles() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0x00]);
    assert_eq!(cpu.step(&mut bus), 4);
    assert_eq!(cpu.regs.pc, 0x0001);
}

#[test]
fn halt_sets_halted_flag() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0x76]);
    cpu.step(&mut bus);
    assert!(cpu.halted);
}

#[test]
fn halted_cpu_ticks_4_cycles_without_advancing_pc() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0x76]);
    cpu.step(&mut bus);
    let pc_before = cpu.regs.pc;
    assert_eq!(cpu.step(&mut bus), 4);
    assert_eq!(cpu.regs.pc, pc_before);
}

// ── loads ────────────────────────────────────────────────────────────────────

#[test]
fn ld_bc_u16_loads_immediate() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0x01, 0x34, 0x12]); // LD BC, 0x1234
    assert_eq!(cpu.step(&mut bus), 12);
    assert_eq!(cpu.regs.get_bc(), 0x1234);
}

#[test]
fn ld_b_u8_loads_immediate() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0x06, 0xAB]); // LD B, 0xAB
    assert_eq!(cpu.step(&mut bus), 8);
    assert_eq!(cpu.regs.b, 0xAB);
}

#[test]
fn ld_r8_r8_copies_register() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.b = 0x55;
    bus.load(0x0000, &[0x78]); // LD A, B
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x55);
}

#[test]
fn ld_hl_indirect_reads_memory() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.set_hl(0x0200);
    bus.0[0x0200] = 0x42;
    bus.load(0x0000, &[0x7E]); // LD A, (HL)
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x42);
}

// ── ALU ──────────────────────────────────────────────────────────────────────

#[test]
fn add_a_b_overflow_sets_carry_and_zero() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0xFF;
    cpu.regs.b = 0x01;
    bus.load(0x0000, &[0x80]); // ADD A, B
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.regs.get_flag_z());
    assert!(cpu.regs.get_flag_c());
    assert!(!cpu.regs.get_flag_n());
}

#[test]
fn add_a_b_half_carry() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0x0F;
    cpu.regs.b = 0x01;
    bus.load(0x0000, &[0x80]); // ADD A, B
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x10);
    assert!(cpu.regs.get_flag_h());
    assert!(!cpu.regs.get_flag_c());
}

#[test]
fn sub_b_underflow_sets_carry_and_n() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0x00;
    cpu.regs.b = 0x01;
    bus.load(0x0000, &[0x90]); // SUB B
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0xFF);
    assert!(cpu.regs.get_flag_c());
    assert!(cpu.regs.get_flag_n());
    assert!(!cpu.regs.get_flag_z());
}

#[test]
fn and_a_b_sets_h_clears_cn() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0xFF;
    cpu.regs.b = 0x0F;
    bus.load(0x0000, &[0xA0]); // AND B
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x0F);
    assert!(cpu.regs.get_flag_h());
    assert!(!cpu.regs.get_flag_c());
    assert!(!cpu.regs.get_flag_n());
}

#[test]
fn xor_a_a_zeroes_register_and_sets_z() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0xAB;
    bus.load(0x0000, &[0xAF]); // XOR A
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.regs.get_flag_z());
}

#[test]
fn inc_a_sets_half_carry() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0x0F;
    bus.load(0x0000, &[0x3C]); // INC A
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x10);
    assert!(cpu.regs.get_flag_h());
    assert!(!cpu.regs.get_flag_z());
    assert!(!cpu.regs.get_flag_n());
}

#[test]
fn dec_a_sets_n_and_z() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0x01;
    bus.load(0x0000, &[0x3D]); // DEC A
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.regs.get_flag_z());
    assert!(cpu.regs.get_flag_n());
}

// ── branches ─────────────────────────────────────────────────────────────────

#[test]
fn jr_unconditional_jumps() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0x18, 0x04]); // JR +4 → 0x0002 + 4 = 0x0006
    assert_eq!(cpu.step(&mut bus), 12);
    assert_eq!(cpu.regs.pc, 0x0006);
}

#[test]
fn jr_nz_taken_when_z_clear() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.set_flag_z(false);
    bus.load(0x0000, &[0x20, 0x04]); // JR NZ, +4
    assert_eq!(cpu.step(&mut bus), 12);
    assert_eq!(cpu.regs.pc, 0x0006);
}

#[test]
fn jr_nz_not_taken_when_z_set() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.set_flag_z(true);
    bus.load(0x0000, &[0x20, 0x04]); // JR NZ, +4
    assert_eq!(cpu.step(&mut bus), 8);
    assert_eq!(cpu.regs.pc, 0x0002);
}

#[test]
fn jp_u16_jumps_absolute() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0xC3, 0x00, 0x03]); // JP 0x0300
    assert_eq!(cpu.step(&mut bus), 16);
    assert_eq!(cpu.regs.pc, 0x0300);
}

// ── stack / call / ret ────────────────────────────────────────────────────────

#[test]
fn push_bc_writes_to_stack() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.set_bc(0xABCD);
    bus.load(0x0000, &[0xC5]); // PUSH BC
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.sp, 0xFFFC);
    assert_eq!(bus.read(0xFFFD), 0xAB); // high
    assert_eq!(bus.read(0xFFFC), 0xCD); // low
}

#[test]
fn pop_de_reads_from_stack() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.set_bc(0x1234);
    bus.load(0x0000, &[0xC5, 0xD1]); // PUSH BC; POP DE
    cpu.step(&mut bus);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.get_de(), 0x1234);
    assert_eq!(cpu.regs.sp, 0xFFFE);
}

#[test]
fn call_pushes_return_addr_and_jumps() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0xCD, 0x00, 0x02]); // CALL 0x0200
    assert_eq!(cpu.step(&mut bus), 24);
    assert_eq!(cpu.regs.pc, 0x0200);
    assert_eq!(cpu.regs.sp, 0xFFFC);
    // return address = 0x0003 (after 3-byte CALL)
    assert_eq!(bus.read(0xFFFD), 0x00);
    assert_eq!(bus.read(0xFFFC), 0x03);
}

#[test]
fn ret_pops_and_returns() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0xCD, 0x00, 0x02]); // CALL 0x0200
    bus.load(0x0200, &[0xC9]);              // RET
    cpu.step(&mut bus); // CALL
    cpu.step(&mut bus); // RET
    assert_eq!(cpu.regs.pc, 0x0003);
    assert_eq!(cpu.regs.sp, 0xFFFE);
}

// ── CB prefix ────────────────────────────────────────────────────────────────

#[test]
fn cb_swap_a_swaps_nibbles() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0xAB;
    bus.load(0x0000, &[0xCB, 0x37]); // SWAP A
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0xBA);
    assert!(!cpu.regs.get_flag_z());
}

#[test]
fn cb_bit_0_a_sets_z_when_bit_clear() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0x02; // bit 0 is clear
    bus.load(0x0000, &[0xCB, 0x47]); // BIT 0, A
    cpu.step(&mut bus);
    assert!(cpu.regs.get_flag_z());
    assert!(cpu.regs.get_flag_h());
    assert!(!cpu.regs.get_flag_n());
}

#[test]
fn cb_rl_a_rotates_through_carry() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.a = 0x80;
    cpu.regs.set_flag_c(false);
    bus.load(0x0000, &[0xCB, 0x17]); // RL A
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x00);
    assert!(cpu.regs.get_flag_c());
    assert!(cpu.regs.get_flag_z());
}

// ── interrupts ───────────────────────────────────────────────────────────────

#[test]
fn interrupt_dispatches_to_vblank_vector() {
    let mut cpu = cpu_at(0x0100);
    let mut bus = FlatBus::new();
    cpu.regs.ime = true;
    bus.write(0xFF0F, 0x01); // IF: vblank pending
    bus.write(0xFFFF, 0x01); // IE: vblank enabled
    assert_eq!(cpu.step(&mut bus), 20);
    assert_eq!(cpu.regs.pc, 0x0040);
    assert!(!cpu.regs.ime);
    assert_eq!(bus.read(0xFF0F) & 0x01, 0); // IF bit cleared
}

#[test]
fn interrupt_ignored_when_ime_false() {
    let mut cpu = cpu_at(0x0000);
    let mut bus = FlatBus::new();
    cpu.regs.ime = false;
    bus.write(0xFF0F, 0x01);
    bus.write(0xFFFF, 0x01);
    bus.load(0x0000, &[0x00]); // NOP
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0x0001); // not redirected
}
