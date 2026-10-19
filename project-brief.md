# Project brief — Lambda Interpreter Demo

**Owner:** Perrin Alaine-Sedano · **Folder:** `lambda-interpreter-web/` · **Repo:** https://github.com/pda-sedano/lambda-interpreter-web · **Written:** 2026-10-19

## Who it's for and why

- **Visitors:** professional connections, professors, potential employers, and anyone curious about Perrin's work.
- **Their task:** try a working untyped lambda calculus interpreter in the browser, without installing anything, and come away knowing who built it and where to see more.
- **Outcome (one sentence):** A visitor runs a lambda term, watches it reduce to normal form, and can tell who built the interpreter and where to find the source.

## Real or fictional

**Real.** This is Perrin's own project and identity. The site has **no fictional practice content**: nothing invented about him, his work, or the interpreter. Anything not yet supplied is marked **Unknown** below instead of filled in.

## Supplied facts and assets

From `AGENTS.md`, `project-state.md`, `ulc/README.md`, and the files in `ulc/`:

- Perrin built the interpreter last year. Internally, `ulc/` is packaged from his BusyReaper project (`src/ulc`, commit `c71450b`). **The site must not mention BusyReaper yet**; say nothing about where it came from beyond "built by Perrin".
- It reduces untyped lambda calculus terms in **normal order** using de Bruijn indices. Output keeps the original variable names and adds scope indices (`x^1`) when a name is shadowed.
- Syntax: `λx.M`, multi-binder shorthand `λx y z.M`, left-associative application, and `x^n` scope indices. A `λ` body is a single term. Typing `\` inserts `λ`.
- It is packaged as WebAssembly with a JavaScript API (`run`, `parse`, `Evaluator`, `maxDepth`). The prebuilt package is in `ulc/pkg/`, so the site deploys without a Rust toolchain.
- Limits: nesting depth 1000 (refused cleanly with an error or `"too_deep"`). About 1M reductions/s on Ω in headless Chromium.
- Existing assets: a minimal working demo page (`ulc/examples/index.html`), 81 Rust tests, and 18 JS tests (results as recorded in `project-state.md`).
- About-me links: **GitHub** (profile `https://github.com/pda-sedano`, same owner as the repo) and **LinkedIn** (`https://www.linkedin.com/in/perrin-alaine-sedano-4589b31b5/`).
- The repo is **public**, so a "View source" link to it is in scope.
- Later, not now: a link to Perrin's main personal website, once that site is ready.
- Known example: Church 3 + 2 →
  `λf x.(f (f (f (f (f x)))))` in 6 steps.

## Main action

**Run** (updated 2026-10-19: no separate jump button; the interpreter sits right under the header and menu bar). The box is preloaded with an example. Pressing **Run** reduces the expression in the browser and shows the steps, the step count, and the stop status. Nothing is sent to a server, and no form, account, or contact message is processed.

## Unknown

1. **Short bio line:** optional. If none is supplied, the about section is just the name and links.

## Assumptions

- First version: **one page**: the interpreter and how it works, with a menu bar for View source, GitHub, and LinkedIn (revised 2026-10-19; see `project-scope.md`).
- The site is static HTML/CSS/JS plus the committed `ulc/pkg/` files, deployed to a personal (noncommercial) Vercel project.
- WebAssembly will not load from a `file://` address, so local checks use `python3 -m http.server` instead of double-clicking `index.html`. This differs from the course's default "open the file directly" check.
- Long runs stay bounded by the 1000 ms timeout: the page may freeze for up to about 1.5 s, then accepts input. A Web Worker (no freeze) is a later option.

## Observable success statements

1. A first-time visitor can say what the page does and who built it within about 30 seconds.
2. Running the preloaded 3 + 2 example shows `λf x.(f (f (f (f (f x)))))` and 6 steps.
3. Invalid syntax shows the interpreter's error message, and the next valid Run still works.
4. Ω (`λx.(x x) λx.(x x)`) stops with a visible timeout status after a freeze of no more than about 1.5 s, and the page then accepts input.
5. At phone width the page is readable, and Run can be reached and activated with Tab plus Enter.
6. Every link opens its intended destination.
