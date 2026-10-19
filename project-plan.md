# Build plan — Lambda Interpreter Demo

Builds the agreed `project-scope.md`. Own design, grown from the working `ulc/examples/index.html`. That example and the interpreter itself stay unchanged. (`ulc/build.sh` and the README's Build section were added separately on 2026-10-19 so rebuilds don't embed local folder paths.)

## Visitor journey

- **First impression (top of the page):** the name, one line ("An interpreter for the untyped lambda calculus. Try it in your browser."), "by Perrin Alaine-Sedano", and the **Try the interpreter** button.
- **Task:** press the button, run the preloaded 3 + 2 or click an example, read the result, then optionally scroll to *How it works* and *About*.

## Page outline (in order)

1. **Header:** title, one-line description, byline, Try the interpreter button, and small nav (Try it · How it works · About).
2. **Try it** (`#try`): one-line local-only note; example buttons (3 + 2, Identity, Ω); expression box (preloaded 3 + 2, `\` types `λ`); max-steps field; "Show every step" toggle; Run; a one-line **status** (`aria-live="polite"`: step count and status, or "Error: …"); and the trace box with the steps (not a live region, so screen readers don't read the whole trace).
3. **How it works** (`#how`): 3–4 sentences on normal-order reduction and de Bruijn indices; a syntax table; limits (depth 1000, 1-second timeout).
4. **About** (`#about`): "Built by Perrin Alaine-Sedano" plus GitHub, LinkedIn, and View source links.
5. **Footer:** "Runs entirely in your browser via WebAssembly."

## Style

Simple typography and no images: a system sans font for text and monospace for terms and output. One accent color for the button and links. Generous spacing and a max width of about 48rem. Light theme only for this version. Error text uses the interpreter's own message in red, with the word "Error" so it doesn't rely on color alone.

## Files

- **New:** `index.html` at the repo root (HTML, CSS, and JS in one file). It imports `./ulc/pkg/ulc.js`. No framework, no build step, and no other new files.
- **Unchanged:** everything in `ulc/` during these increments. `ulc/.gitignore` already excludes `ulc/target/`.
- **Local preview:** run `python3 -m http.server 8000` in `lambda-interpreter-web/`, then open `http://localhost:8000/`. WebAssembly won't load from a double-clicked file.
- **Vercel later:** root directory = repo root, preset **Other**, empty build command.

## Content to use (already supplied)

Example terms, verified against the built package on 2026-10-19:

| Button | Term | Expected result |
| --- | --- | --- |
| 3 + 2 | `λm n f x.(m f (n f x)) λf x.(f (f (f x))) λf x.(f (f x))` | `λf x.(f (f (f (f (f x)))))`, 6 steps, `normal_form` |
| Identity | `λx.x λy.y` | `λy.y`, 1 step, `normal_form` |
| Ω | `λx.(x x) λx.(x x)` | status `timeout` after about 1 s (the page is frozen for that second by design) |

Syntax, limits, and wording come from `ulc/README.md`. Links come from `project-brief.md`. Don't mention BusyReaper on the web page. Mentions in the repo (READMEs, notes) are fine.

## Build increments

### 1. Content and structure

- **Build:** create `index.html` with the header, three sections, footer, and all real text. Move the existing example's working interpreter (Run, `\`→`λ`, max steps, Show every step, error display) into *Try it*, with three changes from the review:
  - **Output cap from the start:** show at most the first 100 and last 100 steps, with a "… N steps hidden …" line, and cut the trace off at 50,000 characters with a "… output truncated …" line. Measured today: Ω with every step returns about 160,000–400,000 steps (several MB of text), and a long typed term can still be about 900 KB after a step-only cap.
  - **Max-steps check:** the field is a plain text box (`inputmode="numeric"`). Empty means no limit; otherwise it must be a whole number ≥ 0, or the page shows "Max steps must be a whole number (0 or more)" without calling `run`. Today `-1` shows a serde error (`invalid type: floating point '-1.0', expected usize`).
  - **Short live status:** `aria-live` goes on the one-line status, not the trace box.
- **Check:** serve the folder and open `http://localhost:8000/`.
  - The text matches the brief and README, and BusyReaper isn't mentioned.
  - Run on the preloaded term shows `λf x.(f (f (f (f (f x)))))` and 6 steps.
  - A broken term such as `λx.` shows the error message in the status line.
  - Max steps: `-1`, `1.5`, and `abc` each show the "whole number" message. `2` on 3 + 2 stops with status `max_steps` after 2 steps. Empty means no limit.
  - Ω with Show every step on: the page is frozen for no more than about 1.5 s, then shows `timeout`, the hidden-steps line, and a trace no longer than 50,000 characters. 3 + 2 runs normally afterward.
- **File:** `index.html` (new).

### 2. Main action and navigation

- **Build:**
  - Try the interpreter jumps to `#try` and puts the cursor in the expression box. The nav links reach each section.
  - The three example buttons fill the box (they don't run it automatically).
  - Add the local-only note.
- **Check:** use only the keyboard. Tab to Try the interpreter, press Enter, and confirm focus lands in the box. Tab to Run and press Enter.
  - Each example gives the result in the table above.
  - Ω via its button: frozen for no more than about 1.5 s, then `timeout`, and the page accepts input again.
  - Each nav link reaches its section.
- **File:** `index.html`.

### 3. Usability and finish

- **Build:**
  - Phone layout: controls wrap, and the output box scrolls inside itself instead of widening the page.
  - Visible focus outlines.
  - A `<title>` and meta description.
  - Make sure GitHub, LinkedIn, and View source open the right pages.
  - Add `.vercelignore` so the deployment serves only `index.html` and `ulc/pkg/` (not the planning notes, `AGENTS.md`, `ulc/src/`, or the READMEs).
- **Check:** narrow the browser to about 375 px wide.
  - Nothing spills sideways.
  - Repeat the increment 1 and 2 checks.
  - Click all three external links.
  - Code check: only the status line has `aria-live`. Optional: with VoiceOver on (Cmd+F5), Run announces only the short status. Record as Not tested if skipped.
  - Record a real-phone check separately (Not tested until done).
- **Files:** `index.html`, `.vercelignore` (new).

## Checkpoints

Local git. Baseline for increment 1: commit `b583568` ("Initial commit."). Commit locally after each passing increment. No push until Perrin approves.

## Missing facts or decisions

None blocking. An optional bio line can be added later.
