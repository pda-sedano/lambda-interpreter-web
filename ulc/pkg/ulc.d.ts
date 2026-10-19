/* tslint:disable */
/* eslint-disable */

export interface RunOptions {
    /** Maximum number of beta reductions. Omit for no limit. */
    maxSteps?: number;
    /** Return every intermediate expression instead of only the last one. Default false. */
    outputAll?: boolean;
    /** Wall-clock budget in milliseconds. Default 1000. Pass Infinity to disable. */
    timeoutMs?: number;
}

export type RunStatus = "normal_form" | "max_steps" | "timeout" | "too_deep";

export interface RunResult {
    /** Pretty-printed expressions: all of them with outputAll, otherwise just the last one reached. */
    steps: string[];
    /** Number of beta reductions performed. */
    stepCount: number;
    /**
     * "normal_form": fully reduced. "max_steps": stopped by maxSteps. "timeout": stopped by timeoutMs.
     * "too_deep": the next term would be nested deeper than MAX_DEPTH (1000) levels.
     */
    status: RunStatus;
}

export interface ParseResult {
    /** The expression printed back in canonical form. */
    pretty: string;
    /** The expression in de Bruijn notation, e.g. "λ.λ.(1 0)". */
    deBruijn: string;
    /** Free variable names in order of first appearance. */
    freeVariables: string[];
}



/**
 * Step-by-step evaluation, for animating a reduction or running it in slices
 * (e.g. a few hundred steps per animation frame) so the page never freezes.
 */
export class Evaluator {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * True when no redex remains. (Computes the next reduct and caches it for `step`.)
     */
    isNormalForm(): boolean;
    /**
     * True when evaluation is stuck because the next term would exceed the depth limit.
     */
    isTooDeep(): boolean;
    /**
     * Parses `src`. Throws on a syntax error.
     */
    constructor(src: string);
    /**
     * Performs up to `n` reductions and returns how many were actually done.
     */
    stepMany(n: number): number;
    /**
     * Performs one reduction. Returns false, changing nothing, in normal form or when too deep.
     */
    step(): boolean;
    /**
     * The current expression with its original variable names.
     */
    readonly current: string;
    /**
     * The current expression in de Bruijn notation.
     */
    readonly deBruijn: string;
    /**
     * Reductions performed so far.
     */
    readonly stepCount: number;
}

/**
 * The deepest expression tree the interpreter accepts (see the Rust docs for why).
 */
export function maxDepth(): number;

/**
 * Parses `src` without evaluating it. Throws on a syntax error.
 */
export function parse(src: string): ParseResult;

/**
 * Parses and evaluates `src` in normal order. Throws on a syntax error or invalid options.
 */
export function run(src: string, options?: RunOptions | null): RunResult;

/**
 * Called automatically when the module is instantiated. Routes Rust panics to `console.error`
 * so an internal bug shows a readable message instead of only "unreachable".
 */
export function start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_evaluator_free: (a: number, b: number) => void;
    readonly evaluator_current: (a: number) => [number, number];
    readonly evaluator_deBruijn: (a: number) => [number, number];
    readonly evaluator_isNormalForm: (a: number) => number;
    readonly evaluator_isTooDeep: (a: number) => number;
    readonly evaluator_new: (a: number, b: number) => [number, number, number];
    readonly evaluator_step: (a: number) => number;
    readonly evaluator_stepCount: (a: number) => number;
    readonly evaluator_stepMany: (a: number, b: number) => number;
    readonly maxDepth: () => number;
    readonly parse: (a: number, b: number) => [number, number, number];
    readonly run: (a: number, b: number, c: number) => [number, number, number];
    readonly start: () => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __externref_table_alloc: () => number;
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
