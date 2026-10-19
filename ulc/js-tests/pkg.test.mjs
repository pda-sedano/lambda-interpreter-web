// Tests the built WebAssembly package (../pkg) from JavaScript.
// Build first (see README), then run:  node --test js-tests/
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import init, { run, parse, Evaluator, maxDepth } from "../pkg/ulc.js";

await init({ module_or_path: await readFile(new URL("../pkg/ulc_bg.wasm", import.meta.url)) });

const PLUS_3_2 = "λm n f x.(m f (n f x)) λf x.(f (f (f x))) λf x.(f (f x))";
// Same trace the old HTTP server returned (brc.rs, run_output_all).
const PLUS_3_2_TRACE = [
  "λm n f x.(m f (n f x)) λf x.(f (f (f x))) λf x.(f (f x))",
  "λn f x.(λf x.(f (f (f x))) f (n f x)) λf x.(f (f x))",
  "λf x.(λf x.(f (f (f x))) f (λf x.(f (f x)) f x))",
  "λf x.(λx.(f (f (f x))) (λf x.(f (f x)) f x))",
  "λf x.(f (f (f (λf x.(f (f x)) f x))))",
  "λf x.(f (f (f (λx.(f (f x)) x))))",
  "λf x.(f (f (f (f (f x)))))",
];
const OMEGA = "λx.(x x) λx.(x x)";

test("run with default options returns only the normal form", () => {
  assert.deepEqual(run(PLUS_3_2), { steps: [PLUS_3_2_TRACE[6]], stepCount: 6, status: "normal_form" });
});

test("outputAll returns the full trace", () => {
  assert.deepEqual(run(PLUS_3_2, { outputAll: true }).steps, PLUS_3_2_TRACE);
});

test("maxSteps matches the old server", () => {
  assert.deepEqual(run(PLUS_3_2, { outputAll: true, maxSteps: 3 }), {
    steps: PLUS_3_2_TRACE.slice(0, 4), stepCount: 3, status: "max_steps",
  });
  assert.deepEqual(run(PLUS_3_2, { maxSteps: 3 }).steps, [PLUS_3_2_TRACE[3]]);
});

test("a term without a normal form times out instead of hanging", () => {
  const t0 = performance.now();
  const r = run(OMEGA, { timeoutMs: 50 });
  const elapsed = performance.now() - t0;
  assert.equal(r.status, "timeout");
  assert.deepEqual(r.steps, [OMEGA]);
  assert.ok(r.stepCount > 0);
  assert.ok(elapsed < 500, `took ${elapsed} ms`);
});

test("maxSteps wins over a generous timeout", () => {
  assert.deepEqual(run(OMEGA, { maxSteps: 10000, timeoutMs: Infinity }), {
    steps: [OMEGA], stepCount: 10000, status: "max_steps",
  });
});

test("null and undefined options are accepted", () => {
  assert.equal(run("λx.x", null).status, "normal_form");
  assert.equal(run("λx.x", undefined).status, "normal_form");
});

test("bad option types throw a readable Error", () => {
  assert.throws(() => run("λx.x", { maxSteps: "ten" }), /Invalid options/);
});

test("syntax errors throw an Error with the interpreter's message", () => {
  assert.throws(() => run("λx."), {
    message: "Error! Expected '(', 'λ' or variable, recieved unexpected end of input",
  });
  assert.throws(() => run("(a b"), /Expected .*'\)'/);
  assert.throws(() => run("λx.x^5"), /scope index 5 is out of bounds/);
  assert.throws(() => parse(""), Error);
});

test("parse reports pretty, de Bruijn, and free variables", () => {
  assert.deepEqual(parse("λx y.(x z w)"), {
    pretty: "λx y.(x z w)", deBruijn: "λ.λ.((1 2) 3)", freeVariables: ["z", "w"],
  });
});

test("scope indices round-trip", () => {
  assert.equal(parse("λx x.x^1").pretty, "λx x.x^1");
  assert.equal(run("(λy x.y) x").steps[0], "λx.x^1");
});

test("Evaluator steps through a reduction", () => {
  const e = new Evaluator(PLUS_3_2);
  const seen = [e.current];
  assert.equal(e.isNormalForm(), false);
  while (e.step()) seen.push(e.current);
  assert.deepEqual(seen, PLUS_3_2_TRACE);
  assert.equal(e.stepCount, 6);
  assert.equal(e.isNormalForm(), true);
  assert.equal(e.deBruijn, "λ.λ.(1 (1 (1 (1 (1 0)))))");
  e.free();
});

test("Evaluator.stepMany runs in slices", () => {
  const e = new Evaluator(PLUS_3_2);
  assert.equal(e.stepMany(4), 4);
  assert.equal(e.stepMany(100), 2);
  assert.equal(e.current, PLUS_3_2_TRACE[6]);
  e.free();
});

test("Evaluator constructor throws on syntax errors", () => {
  assert.throws(() => new Evaluator("λ"), Error);
});

test("unicode variable names survive the JS boundary", () => {
  assert.equal(run("(λ🦊.(🦊 🦊)) 犬").steps[0], "犬 犬");
});

test("Church arithmetic: 2^9 = 512", () => {
  const pow = "(λb e.(e b)) (λf x.(f (f x))) (λf x.(f (f (f (f (f (f (f (f (f x))))))))))";
  const r = run(pow, { timeoutMs: Infinity });
  assert.equal(r.status, "normal_form");
  // The result is printed as λx x.(x^1 (x^1 ... x)): one x^1 per application of the outer function.
  assert.ok(r.steps[0].startsWith("λx x.(x^1 "));
  assert.equal(r.steps[0].split("x^1").length - 1, 512);
});

test("maxDepth is exported", () => {
  assert.equal(maxDepth(), 1000);
});

// Before the depth limit, inputs like these overflowed the WebAssembly stack and left the
// module unusable until the page was reloaded.
test("over-deep input throws a normal Error and the module keeps working", () => {
  const huge = [
    "x ".repeat(200000),
    "(".repeat(200000) + "x" + ")".repeat(200000),
    "λx.".repeat(200000) + "x",
    "λf x." + "(f ".repeat(5000) + "x" + ")".repeat(5000),
  ];
  for (const src of huge) {
    assert.throws(() => run(src), /nested more than 1000 levels/);
    assert.throws(() => parse(src), /nested more than 1000 levels/);
    assert.throws(() => new Evaluator(src), /nested more than 1000 levels/);
  }
  assert.deepEqual(run("(λx.x) y").steps, ["y"]);
});

test("a term that grows without bound stops with status too_deep", () => {
  const r = run("(λx.(x x x)) λx.(x x x)", { timeoutMs: Infinity });
  assert.equal(r.status, "too_deep");
  const e = new Evaluator("(λx.(x x x)) λx.(x x x)");
  while (e.step()) {}
  assert.equal(e.isTooDeep(), true);
  assert.equal(e.isNormalForm(), false);
  e.free();
  assert.deepEqual(run("(λx.x) y").steps, ["y"]);
});
