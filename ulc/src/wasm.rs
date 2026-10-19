//! JavaScript bindings (via wasm-bindgen).
//!
//! ```js
//! import init, { run, parse, Evaluator } from "./pkg/ulc.js";
//! await init();
//! run("(λx.(x x)) y");                       // { steps: ["y y"], stepCount: 1, status: "normal_form" }
//! run(src, { outputAll: true, maxSteps: 50, timeoutMs: 200 });
//! ```
//! Syntax errors are thrown as `Error`s carrying the interpreter's message.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{RunOptions, StopReason};

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
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
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "RunOptions")]
    pub type JsRunOptions;
    #[wasm_bindgen(typescript_type = "RunResult")]
    pub type JsRunResult;
    #[wasm_bindgen(typescript_type = "ParseResult")]
    pub type JsParseResult;
}

const DEFAULT_TIMEOUT_MS: f64 = 1000.0;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Options {
    max_steps: Option<usize>,
    output_all: bool,
    timeout_ms: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunOutput {
    steps: Vec<String>,
    step_count: usize,
    status: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ParseOutput {
    pretty: String,
    de_bruijn: String,
    free_variables: Vec<String>,
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsError::new(&e.to_string()))
}

fn syntax_error(e: crate::SyntaxError) -> JsError {
    JsError::new(&e.to_string())
}

/// Called automatically when the module is instantiated. Routes Rust panics to `console.error`
/// so an internal bug shows a readable message instead of only "unreachable".
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// The deepest expression tree the interpreter accepts (see the Rust docs for why).
#[wasm_bindgen(js_name = maxDepth)]
pub fn max_depth() -> usize {
    crate::MAX_DEPTH
}

/// Parses `src` without evaluating it. Throws on a syntax error.
#[wasm_bindgen(js_name = parse)]
pub fn parse_js(src: &str) -> Result<JsParseResult, JsError> {
    let pretty = crate::parse(src).map_err(syntax_error)?;
    let out = ParseOutput {
        pretty: pretty.to_string(),
        de_bruijn: pretty.get_expr().to_string(),
        free_variables: pretty.free_vars().to_vec(),
    };
    Ok(to_js(&out)?.unchecked_into())
}

/// Parses and evaluates `src` in normal order. Throws on a syntax error or invalid options.
#[wasm_bindgen(js_name = run)]
pub fn run_js(src: &str, options: Option<JsRunOptions>) -> Result<JsRunResult, JsError> {
    let opts: Options = match options {
        Some(o) if !o.is_undefined() && !o.is_null() => serde_wasm_bindgen::from_value(o.into())
            .map_err(|e| JsError::new(&format!("Invalid options: {e}")))?,
        _ => Options::default(),
    };

    let timeout_ms = opts.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let deadline = js_sys::Date::now() + timeout_ms;
    let run_options = RunOptions { max_steps: opts.max_steps, output_all: opts.output_all };

    let pretty = crate::parse(src).map_err(syntax_error)?;
    let result = crate::run_with(pretty, &run_options, || {
        timeout_ms.is_finite() && js_sys::Date::now() >= deadline
    });

    let out = RunOutput {
        steps: result.steps,
        step_count: result.step_count,
        status: match result.stop {
            StopReason::NormalForm => "normal_form",
            StopReason::MaxSteps => "max_steps",
            StopReason::Interrupted => "timeout",
            StopReason::TooDeep => "too_deep",
        },
    };
    Ok(to_js(&out)?.unchecked_into())
}

/// Step-by-step evaluation, for animating a reduction or running it in slices
/// (e.g. a few hundred steps per animation frame) so the page never freezes.
#[wasm_bindgen(js_name = Evaluator)]
pub struct JsEvaluator(crate::Evaluator);

#[wasm_bindgen(js_class = Evaluator)]
impl JsEvaluator {
    /// Parses `src`. Throws on a syntax error.
    #[wasm_bindgen(constructor)]
    pub fn new(src: &str) -> Result<JsEvaluator, JsError> {
        crate::Evaluator::from_source(src).map(JsEvaluator).map_err(syntax_error)
    }

    /// Performs one reduction. Returns false, changing nothing, in normal form or when too deep.
    pub fn step(&mut self) -> bool {
        self.0.step()
    }

    /// Performs up to `n` reductions and returns how many were actually done.
    #[wasm_bindgen(js_name = stepMany)]
    pub fn step_many(&mut self, n: u32) -> u32 {
        let mut done = 0;
        while done < n && self.0.step() {
            done += 1;
        }
        done
    }

    /// The current expression with its original variable names.
    #[wasm_bindgen(getter)]
    pub fn current(&self) -> String {
        self.0.current()
    }

    /// The current expression in de Bruijn notation.
    #[wasm_bindgen(getter, js_name = deBruijn)]
    pub fn de_bruijn(&self) -> String {
        self.0.current_de_bruijn()
    }

    /// Reductions performed so far.
    #[wasm_bindgen(getter, js_name = stepCount)]
    pub fn step_count(&self) -> usize {
        self.0.step_count()
    }

    /// True when no redex remains. (Computes the next reduct and caches it for `step`.)
    #[wasm_bindgen(js_name = isNormalForm)]
    pub fn is_normal_form(&mut self) -> bool {
        self.0.is_normal_form()
    }

    /// True when evaluation is stuck because the next term would exceed the depth limit.
    #[wasm_bindgen(js_name = isTooDeep)]
    pub fn is_too_deep(&mut self) -> bool {
        self.0.is_too_deep()
    }
}
