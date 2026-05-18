/* tslint:disable */
/* eslint-disable */

export class EmulatorState {
    free(): void;
    [Symbol.dispose](): void;
    af(): number;
    bc(): number;
    de(): number;
    flags(): number;
    framebuffer_ptr(): number;
    frames(): number;
    halted(): boolean;
    hl(): number;
    ie_reg(): number;
    if_reg(): number;
    ime(): boolean;
    is_paused(): boolean;
    lcdc(): number;
    load_save_wasm(data: Uint8Array): void;
    ly(): number;
    constructor(rom: Uint8Array);
    pc(): number;
    save_wasm(): Uint8Array;
    set_paused(p: boolean): void;
    sp(): number;
    stat(): number;
    tick_frame(): void;
    /**
     * Single CPU instruction + proportional PPU/timer — used by the debugger step button.
     */
    tick_step(): void;
    update_joypad(d_pad: number, buttons: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_emulatorstate_free: (a: number, b: number) => void;
    readonly emulatorstate_af: (a: number) => number;
    readonly emulatorstate_bc: (a: number) => number;
    readonly emulatorstate_de: (a: number) => number;
    readonly emulatorstate_flags: (a: number) => number;
    readonly emulatorstate_framebuffer_ptr: (a: number) => number;
    readonly emulatorstate_frames: (a: number) => number;
    readonly emulatorstate_halted: (a: number) => number;
    readonly emulatorstate_hl: (a: number) => number;
    readonly emulatorstate_ie_reg: (a: number) => number;
    readonly emulatorstate_if_reg: (a: number) => number;
    readonly emulatorstate_ime: (a: number) => number;
    readonly emulatorstate_is_paused: (a: number) => number;
    readonly emulatorstate_lcdc: (a: number) => number;
    readonly emulatorstate_load_save_wasm: (a: number, b: number, c: number) => void;
    readonly emulatorstate_ly: (a: number) => number;
    readonly emulatorstate_new: (a: number, b: number) => number;
    readonly emulatorstate_pc: (a: number) => number;
    readonly emulatorstate_save_wasm: (a: number) => [number, number];
    readonly emulatorstate_set_paused: (a: number, b: number) => void;
    readonly emulatorstate_sp: (a: number) => number;
    readonly emulatorstate_stat: (a: number) => number;
    readonly emulatorstate_tick_frame: (a: number) => void;
    readonly emulatorstate_tick_step: (a: number) => void;
    readonly emulatorstate_update_joypad: (a: number, b: number, c: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
