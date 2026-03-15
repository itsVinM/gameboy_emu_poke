/* tslint:disable */
/* eslint-disable */

export class EmulatorState {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Returns a pointer to the PPU framebuffer for zero-copy drawing in JS
     */
    framebuffer_ptr(): number;
    /**
     * Loads external save data into the MMU
     */
    load_save(data: Uint8Array): void;
    constructor(rom: Uint8Array);
    /**
     * Returns the current save data (External RAM)
     */
    save(): Uint8Array;
    /**
     * Executes one full frame of Game Boy logic (~16.7ms)
     */
    tick_frame(): void;
    /**
     * Updates Joypad state from JavaScript key events
     * dpad_mask and button_mask should be passed as bitflags (Active Low)
     */
    update_joypad(d_pad: number, buttons: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_emulatorstate_free: (a: number, b: number) => void;
    readonly emulatorstate_framebuffer_ptr: (a: number) => number;
    readonly emulatorstate_load_save: (a: number, b: number, c: number) => void;
    readonly emulatorstate_new: (a: number, b: number) => number;
    readonly emulatorstate_save: (a: number) => [number, number];
    readonly emulatorstate_tick_frame: (a: number) => void;
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
