/* tslint:disable */
/* eslint-disable */

/**
 * `WasmMicrogridController` - a JS-friendly wrapper that holds a
 * `MicrogridState` and emits control actions on each step.
 */
export class WasmMicrogridController {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Construct a controller from a JSON state, validating it up front.
     */
    constructor(state_json: string);
    /**
     * Run one control step and return the actions as JSON.
     */
    step(measurements_json: string, config_json?: string | null): string;
    /**
     * The current state JSON.
     */
    readonly state_json: string;
}

export function _start(): void;

/**
 * Report the carbon intensity of a system, as kg CO2 per MWh.
 */
export function wasm_carbon_intensity_json(input: string): any;

/**
 * Run a lossless economic dispatch. See the Rust docs for the request shape.
 */
export function wasm_economic_dispatch_json(request: string): any;

/**
 * Compute the levelized cost of energy. See the Rust docs for the request shape.
 */
export function wasm_lcoe_json(request: string): any;

/**
 * Run one microgrid control step and return the decision as JSON.
 */
export function wasm_microgrid_step_json(controller_state_json: string, measurements_json: string, config_json?: string | null): string;

/**
 * Run a Newton-Raphson power flow from a JSON `EnergySystem`.
 *
 * Throws an object `{ kind, message }` on failure.
 */
export function wasm_run_powerflow_json(input: string): any;

/**
 * Run a priority-list unit commitment. See the Rust docs for the request shape.
 */
export function wasm_unit_commitment_json(request: string): any;

/**
 * Validate a JSON `EnergySystem`, throwing a typed error if it is invalid.
 */
export function wasm_validate_system_json(input: string): void;

/**
 * Render a single-line diagram and heatmap as an SVG string.
 */
export function wasm_visualize_json(request: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmmicrogridcontroller_free: (a: number, b: number) => void;
    readonly _start: () => void;
    readonly wasm_carbon_intensity_json: (a: number, b: number, c: number) => void;
    readonly wasm_economic_dispatch_json: (a: number, b: number, c: number) => void;
    readonly wasm_lcoe_json: (a: number, b: number, c: number) => void;
    readonly wasm_microgrid_step_json: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly wasm_run_powerflow_json: (a: number, b: number, c: number) => void;
    readonly wasm_unit_commitment_json: (a: number, b: number, c: number) => void;
    readonly wasm_validate_system_json: (a: number, b: number, c: number) => void;
    readonly wasm_visualize_json: (a: number, b: number, c: number) => void;
    readonly wasmmicrogridcontroller_new: (a: number, b: number, c: number) => void;
    readonly wasmmicrogridcontroller_state_json: (a: number, b: number) => void;
    readonly wasmmicrogridcontroller_step: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
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
