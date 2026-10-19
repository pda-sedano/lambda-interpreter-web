//! An interpreter for the untyped lambda calculus.
//!
//! Terms are stored with de Bruijn indices and reduced in normal order
//! (leftmost-outermost redex first), so any term that has a normal form reaches it.
//!
//! Syntax:
//! - variable: any run of characters other than whitespace and `λ . ( ) ^`
//! - abstraction: `λx.M`, with `λx y z.M` as shorthand for `λx.λy.λz.M`
//! - application: `M N`, left associative (`a b c` = `(a b) c`)
//! - scope index: `x^n` refers to the `x` bound `n` binders further out than the nearest one
//!
//! The body of a `λ` is a single term, so `λx.x y` parses as `(λx.x) y`.
//! Write `λx.(x y)` to put the application inside the abstraction.
//!
//! Native use:
//! ```
//! let result = ulc::run("λm n f x.(m f (n f x)) λf x.(f (f x)) λf x.(f x)", &ulc::RunOptions::default()).unwrap();
//! assert_eq!(result.steps.last().unwrap(), "λf x.(f (f (f x)))");
//! ```
//!
//! The JavaScript API lives in [`wasm`]; see the README for how to build and load it.

pub mod expression;
pub mod lexer;
pub mod parser;
pub mod wasm;

use expression::{Expression, PrettifiedExpression};
use lexer::Lex;
pub use parser::SyntaxError;
use parser::Parser;

/// Deepest expression tree the interpreter will build. Parsing, reduction and printing are
/// recursive, and WebAssembly in the browser has a small call stack (V8 overflows at roughly
/// 3,500 levels for these functions), so terms deeper than this are rejected instead of
/// crashing the module.
pub const MAX_DEPTH: usize = 1000;

/// Parses source text into an expression that remembers its variable names for display.
pub fn parse(src: &str) -> Result<PrettifiedExpression, SyntaxError> {
    let pretty = Parser::new(Lex::new(src)).expr()?;

    if pretty.get_expr().depth() > MAX_DEPTH {
        return Err(SyntaxError::TooDeepError { limit: MAX_DEPTH });
    }

    Ok(pretty)
}

/// Why an evaluation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// No redex remains: the result is the term's normal form.
    NormalForm,
    /// `max_steps` reductions were performed and the term is still reducible.
    MaxSteps,
    /// The caller's `should_stop` callback asked to stop (used for timeouts).
    Interrupted,
    /// The next reduct would be deeper than [`MAX_DEPTH`].
    TooDeep,
}

/// Options for [`run`] and [`run_with`].
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Maximum number of beta reductions. `None` means no limit.
    pub max_steps: Option<usize>,
    /// Record every intermediate expression, not just the last one.
    pub output_all: bool,
}

/// The outcome of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    /// Pretty-printed expressions. With `output_all`, every expression from the input
    /// to the last one reached; otherwise only the last one.
    pub steps: Vec<String>,
    /// Number of beta reductions performed.
    pub step_count: usize,
    pub stop: StopReason,
}

enum Next {
    Reduct(Expression),
    NormalForm,
    TooDeep,
}

/// Steps through the reduction of one expression.
///
/// The next reduct is computed at most once and cached, so asking whether the
/// current term is in normal form and then stepping costs a single reduction.
pub struct Evaluator {
    pretty: PrettifiedExpression,
    step_count: usize,
    next: Option<Next>,
}

impl Evaluator {
    pub fn new(pretty: PrettifiedExpression) -> Self {
        Self { pretty, step_count: 0, next: None }
    }

    pub fn from_source(src: &str) -> Result<Self, SyntaxError> {
        parse(src).map(Self::new)
    }

    fn peek(&mut self) -> &Next {
        if self.next.is_none() {
            self.next = Some(match self.pretty.get_expr().normal_order_reduce() {
                None => Next::NormalForm,
                Some(reduct) if reduct.depth() > MAX_DEPTH => Next::TooDeep,
                Some(reduct) => Next::Reduct(reduct),
            });
        }
        self.next.as_ref().unwrap()
    }

    /// True when no redex remains.
    pub fn is_normal_form(&mut self) -> bool {
        matches!(self.peek(), Next::NormalForm)
    }

    /// True when the next reduction would produce a term deeper than [`MAX_DEPTH`],
    /// so evaluation cannot continue.
    pub fn is_too_deep(&mut self) -> bool {
        matches!(self.peek(), Next::TooDeep)
    }

    /// Performs one normal-order reduction. Returns false (and changes nothing) when in
    /// normal form or when the next term would be too deep.
    pub fn step(&mut self) -> bool {
        self.peek();
        match self.next.take().unwrap() {
            Next::Reduct(reduct) => {
                self.pretty.set_expr(reduct);
                self.step_count += 1;
                true
            }
            blocked => {
                self.next = Some(blocked);
                false
            }
        }
    }

    pub fn step_count(&self) -> usize {
        self.step_count
    }

    /// The current expression with its original variable names.
    pub fn current(&self) -> String {
        self.pretty.to_string()
    }

    /// The current expression in de Bruijn notation, e.g. `λ.λ.(1 0)`.
    pub fn current_de_bruijn(&self) -> String {
        self.pretty.get_expr().to_string()
    }

    pub fn pretty(&self) -> &PrettifiedExpression {
        &self.pretty
    }
}

/// Reduces `pretty` until it reaches normal form, hits `max_steps`, or `should_stop` returns true.
/// `should_stop` is called once before every reduction.
pub fn run_with(
    pretty: PrettifiedExpression,
    options: &RunOptions,
    mut should_stop: impl FnMut() -> bool,
) -> RunResult {
    let mut eval = Evaluator::new(pretty);
    let mut steps = Vec::new();

    let stop = loop {
        if options.output_all {
            steps.push(eval.current());
        }
        if should_stop() {
            break StopReason::Interrupted;
        }
        if eval.is_normal_form() {
            break StopReason::NormalForm;
        }
        if eval.is_too_deep() {
            break StopReason::TooDeep;
        }
        if options.max_steps.is_some_and(|max| eval.step_count() >= max) {
            break StopReason::MaxSteps;
        }
        eval.step();
    };

    if !options.output_all {
        steps.push(eval.current());
    }

    RunResult { steps, step_count: eval.step_count(), stop }
}

/// Parses and reduces `src`. There is no time limit, so a term without a normal form
/// runs until `max_steps` (or forever if that is `None`).
pub fn run(src: &str, options: &RunOptions) -> Result<RunResult, SyntaxError> {
    Ok(run_with(parse(src)?, options, || false))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLUS_3_2: &str = "λm n f x.(m f (n f x)) λf x.(f (f (f x))) λf x.(f (f x))";

    // Expected trace taken from the old HTTP server's test suite (brc.rs, run_output_all).
    const PLUS_3_2_TRACE: [&str; 7] = [
        "λm n f x.(m f (n f x)) λf x.(f (f (f x))) λf x.(f (f x))",
        "λn f x.(λf x.(f (f (f x))) f (n f x)) λf x.(f (f x))",
        "λf x.(λf x.(f (f (f x))) f (λf x.(f (f x)) f x))",
        "λf x.(λx.(f (f (f x))) (λf x.(f (f x)) f x))",
        "λf x.(f (f (f (λf x.(f (f x)) f x))))",
        "λf x.(f (f (f (λx.(f (f x)) x))))",
        "λf x.(f (f (f (f (f x)))))",
    ];

    fn opts(max_steps: Option<usize>, output_all: bool) -> RunOptions {
        RunOptions { max_steps, output_all }
    }

    #[test]
    fn output_all_matches_old_server() {
        let r = run(PLUS_3_2, &opts(None, true)).unwrap();
        assert_eq!(r.steps, PLUS_3_2_TRACE);
        assert_eq!(r.step_count, 6);
        assert_eq!(r.stop, StopReason::NormalForm);
    }

    #[test]
    fn final_only() {
        let r = run(PLUS_3_2, &opts(None, false)).unwrap();
        assert_eq!(r.steps, [PLUS_3_2_TRACE[6]]);
        assert_eq!(r.stop, StopReason::NormalForm);
    }

    #[test]
    fn max_steps_stops_early() {
        let r = run(PLUS_3_2, &opts(Some(2), true)).unwrap();
        assert_eq!(r.steps, PLUS_3_2_TRACE[..3]);
        assert_eq!(r.step_count, 2);
        assert_eq!(r.stop, StopReason::MaxSteps);

        let r = run(PLUS_3_2, &opts(Some(2), false)).unwrap();
        assert_eq!(r.steps, [PLUS_3_2_TRACE[2]]);
    }

    #[test]
    fn max_steps_equal_to_needed_reports_normal_form() {
        let r = run(PLUS_3_2, &opts(Some(6), false)).unwrap();
        assert_eq!(r.stop, StopReason::NormalForm);
        assert_eq!(r.steps, [PLUS_3_2_TRACE[6]]);
    }

    #[test]
    fn zero_max_steps_returns_input() {
        let r = run(PLUS_3_2, &opts(Some(0), false)).unwrap();
        assert_eq!(r.steps, [PLUS_3_2_TRACE[0]]);
        assert_eq!(r.stop, StopReason::MaxSteps);
    }

    #[test]
    fn omega_hits_max_steps() {
        let r = run("(λx.(x x)) λx.(x x)", &opts(Some(1000), false)).unwrap();
        assert_eq!(r.stop, StopReason::MaxSteps);
        assert_eq!(r.step_count, 1000);
        assert_eq!(r.steps, ["λx.(x x) λx.(x x)"]);
    }

    #[test]
    fn interrupt_callback_stops_run() {
        let mut calls = 0;
        let r = run_with(parse("(λx.(x x)) λx.(x x)").unwrap(), &opts(None, false), || {
            calls += 1;
            calls > 10
        });
        assert_eq!(r.stop, StopReason::Interrupted);
        assert_eq!(r.step_count, 10);
    }

    #[test]
    fn already_normal() {
        let r = run("λx.x", &opts(None, true)).unwrap();
        assert_eq!(r.steps, ["λx.x"]);
        assert_eq!(r.step_count, 0);
        assert_eq!(r.stop, StopReason::NormalForm);
    }

    #[test]
    fn free_variables_keep_their_names() {
        let r = run("(λx.(x x)) y", &opts(None, false)).unwrap();
        assert_eq!(r.steps, ["y y"]);
    }

    #[test]
    fn syntax_error_message() {
        let e = run("λx.", &opts(None, false)).unwrap_err();
        assert_eq!(e.to_string(), "Error! Expected '(', 'λ' or variable, recieved unexpected end of input");
    }

    fn church(n: usize) -> String {
        format!("λf x.{}x{}", "(f ".repeat(n), ")".repeat(n))
    }

    #[test]
    fn depth_limit_on_parse() {
        // church(n) has depth n + 2: two binders, then n nested applications.
        assert_eq!(parse(&church(MAX_DEPTH - 2)).unwrap().get_expr().depth(), MAX_DEPTH);
        let too_deep = SyntaxError::TooDeepError { limit: MAX_DEPTH };
        assert_eq!(parse(&church(MAX_DEPTH - 1)).unwrap_err(), too_deep);
        // Deep without parser recursion: a long left-nested application chain.
        assert_eq!(parse(&"x ".repeat(100_000)).unwrap_err(), too_deep);
        // Deep through the parser's own recursion.
        assert_eq!(parse(&format!("{}x{}", "(".repeat(100_000), ")".repeat(100_000))).unwrap_err(), too_deep);
        assert_eq!(parse(&format!("{}x", "λx.".repeat(100_000))).unwrap_err(), too_deep);
    }

    #[test]
    fn depth_limit_during_reduction() {
        // λx.(x x x) applied to itself grows by one application per step.
        let r = run("(λx.(x x x)) λx.(x x x)", &opts(None, false)).unwrap();
        assert_eq!(r.stop, StopReason::TooDeep);
        assert!(r.step_count >= MAX_DEPTH - 5);

        let mut e = Evaluator::from_source("(λx.(x x x)) λx.(x x x)").unwrap();
        while e.step() {}
        assert!(e.is_too_deep());
        assert!(!e.is_normal_form());
        assert!(!e.step());
    }

    #[test]
    fn dropping_a_huge_tree_does_not_overflow() {
        let mut e = expression::Expression::Variable(0);
        for _ in 0..1_000_000 {
            e = expression::Expression::Lambda(Box::new(e), String::new());
        }
        assert_eq!(e.depth(), 1_000_000);
        drop(e);
    }

    #[test]
    fn evaluator_steps_and_caches() {
        let mut e = Evaluator::from_source(PLUS_3_2).unwrap();
        assert!(!e.is_normal_form());
        let mut seen = vec![e.current()];
        while e.step() {
            seen.push(e.current());
        }
        assert_eq!(seen, PLUS_3_2_TRACE);
        assert!(e.is_normal_form());
        assert!(!e.step());
        assert_eq!(e.step_count(), 6);
        assert_eq!(e.current_de_bruijn(), "λ.λ.(1 (1 (1 (1 (1 0)))))");
    }
}
