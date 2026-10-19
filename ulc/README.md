# ulc — untyped lambda calculus in WebAssembly

The untyped lambda calculus interpreter from BusyReaper (`src/ulc`, as of commit `c71450b`), packaged as a Rust library with JavaScript bindings for use on a web page.

Terms are stored with de Bruijn indices and reduced in **normal order** (leftmost-outermost redex first), so any term with a normal form reaches it. Output keeps your original variable names and adds scope indices (`x^1`) where a name is shadowed.

## Layout

```
ulc/
  src/lexer.rs, parser.rs, expression.rs   the original interpreter (lightly patched, see below)
  src/lib.rs                               Rust API: parse, run, run_with, Evaluator, MAX_DEPTH
  src/wasm.rs                              JavaScript API (wasm-bindgen)
  pkg/                                     built package: ulc.js + ulc_bg.wasm + TypeScript types
  js-tests/pkg.test.mjs                    tests that load pkg/ from Node
  examples/index.html                      minimal page using the package
```

`pkg/` is committed on purpose: the site can be deployed (for example to Vercel) without a Rust toolchain on the build server. Rebuild it whenever `src/` changes.

## Build

One-time setup:

```sh
rustup update stable   # wasm-bindgen needs Rust 1.81 or newer
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

Build (from `ulc/`):

```sh
sh build.sh
```

`build.sh` runs `wasm-pack build --release --target web` with `RUSTFLAGS="--remap-path-prefix=$HOME=~"` so the `.wasm` doesn't embed your home-folder paths, deletes the `pkg/.gitignore` that wasm-pack writes (its `*` would keep `pkg/` out of git), and fails if any home-folder path is still in `pkg/`.

`--target web` gives a plain ES module that works in a `<script type="module">` with no bundler. If the site later moves to a bundler (Vite, webpack), `--target web` still works with Vite; use `--target bundler` for webpack.

## Test

```sh
cargo test                        # 81 Rust tests (the original 67 plus the API and depth limit)
node --test js-tests/*.test.mjs   # 18 tests against the built pkg/
```

To try the example page, serve `ulc/` over HTTP (wasm can't load from `file://`):

```sh
python3 -m http.server 8000      # then open http://localhost:8000/examples/
```

## JavaScript API

```js
import init, { run, parse, Evaluator, maxDepth } from "./pkg/ulc.js";

await init();   // fetches ulc_bg.wasm next to ulc.js; call once before anything else

run("λm n f x.(m f (n f x)) λf x.(f (f (f x))) λf x.(f (f x))");
// { steps: ["λf x.(f (f (f (f (f x)))))"], stepCount: 6, status: "normal_form" }

run(src, { outputAll: true, maxSteps: 100, timeoutMs: 250 });
```

**`run(src, options?) → { steps, stepCount, status }`**

| option | default | meaning |
|---|---|---|
| `maxSteps` | no limit | maximum number of beta reductions |
| `outputAll` | `false` | return every intermediate expression, not just the last |
| `timeoutMs` | `1000` | wall-clock budget; `Infinity` disables it |

`status` is `"normal_form"`, `"max_steps"`, `"timeout"` or `"too_deep"`. When a run stops early you still get the expressions reached so far (the old server returned only an error on timeout).

**`parse(src) → { pretty, deBruijn, freeVariables }`** checks syntax without evaluating, e.g. for validating input as the user types.

**`new Evaluator(src)`** steps through a reduction, which is what you want for animating one or for running in slices so the page never freezes:

```js
const ev = new Evaluator(src);
function frame() {
  ev.stepMany(500);                 // up to 500 reductions per frame
  show(ev.current);                 // also: ev.deBruijn, ev.stepCount
  if (!ev.isNormalForm() && !ev.isTooDeep()) requestAnimationFrame(frame);
  else ev.free();                   // release the Rust memory when done
}
requestAnimationFrame(frame);
```

Syntax errors are thrown as ordinary `Error`s carrying the interpreter's message (`Error! Expected '(', 'λ' or variable, recieved unexpected end of input`).

**Long runs belong in a Web Worker.** `run` blocks the thread it's called on for up to `timeoutMs`. In a module worker, set `onmessage` before awaiting `init()`, or messages sent while it loads are lost:

```js
// worker.js  (new Worker(new URL("./worker.js", import.meta.url), { type: "module" }))
import init, { run } from "./pkg/ulc.js";
const ready = init();
self.onmessage = async (e) => { await ready; self.postMessage(run(e.data, { timeoutMs: 5000 })); };
```

## Syntax

- variable: any run of characters except whitespace and `λ . ( ) ^` (so `🦊` and `weirdmageddon` are fine)
- abstraction: `λx.M`; `λx y z.M` is shorthand for `λx.λy.λz.M`
- application: `M N`, left associative
- scope index: `x^n` is the `x` bound `n` binders further out than the nearest one
- a `λ` body is a **single term**: `λx.x y` means `(λx.x) y`; write `λx.(x y)` otherwise

## Limits

- **Depth: 1000.** Parsing, reduction and printing are recursive, and browsers give WebAssembly a small call stack (Chrome/V8 overflows at about 3,500 levels for this code). An overflow traps and leaves the module unusable until reload, so terms nested more than `maxDepth()` = 1000 levels are refused: at parse time with an `Error`, and during evaluation with status `"too_deep"`. In practice that covers Church numerals up to about 990. Removing the limit means rewriting `shift`, `substitute`, `normal_order_reduce` and the printers iteratively.
- **Speed:** about 1M reductions per second on Ω (`λx.(x x) λx.(x x)`) in headless Chromium. Each step clones the term, so large terms are slower.

## Changes from the BusyReaper original

- Test module paths changed from `crate::ulc::` to `crate::`; two unused imports removed.
- `PrettifiedExpression::free_vars()` getter added.
- `Expression::depth()` added, plus an iterative `Drop` so freeing a very deep term can't overflow the stack.
- `SyntaxError::TooDeepError` added, and the parser counts its recursion depth.
- The run loop that lived in the axum server (`brc.rs`) is now `run_with` / `Evaluator` in `lib.rs`. Its output matches the old server's test traces.
