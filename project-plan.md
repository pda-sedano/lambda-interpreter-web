# Build plan — Lambda Interpreter Demo

Builds the agreed `project-scope.md`. Own design, grown from the working `ulc/examples/index.html`. That example and `ulc/` stay unchanged.

## Visitor journey

- **First impression (top of the page):** the name, one line ("An interpreter for the untyped lambda calculus. Try it in your browser."), "by Perrin Alaine-Sedano", and the **Try the interpreter** button.
- **Task:** press the button, run the preloaded 3 + 2 or click an example, read the result, then optionally scroll to *How it works* and *About*.

## Page outline (in order)

1. **Header:** title, one-line description, byline, Try the interpreter button, and small nav (Try it · How it works · About).
2. **Try it** (`#try`): one-line local-only note; example buttons (3 + 2, Identity, Ω); expression box (preloaded 3 + 2, `\` types `λ`); max-steps field; "Show every step" toggle; Run; output box (`aria-live`) with steps, step count, and status.
3. **How it works** (`#how`): 3–4 sentences on normal-order reduction and de Bruijn indices; a syntax table; limits (depth 1000, 1-second timeout).
4. **About** (`#about`): "Built by Perrin Alaine-Sedano" plus GitHub, LinkedIn, and View source links.
5. **Footer:** "Runs entirely in your browser via WebAssembly."

## Style

Simple typography and no images: a system sans font for text and monospace for terms and output. One accent color for the button and links. Generous spacing and a max width of about 48rem. Light theme only for this version. Error text uses the interpreter's own message in red, with the word "Error" so it doesn't rely on color alone.

## Files

- **New:** `index.html` at the repo root (HTML, CSS, and JS in one file). It imports `./ulc/pkg/ulc.js`. No framework, no build step, and no other new files.
- **Unchanged:** everything in `ulc/`. `ulc/.gitignore` already excludes `ulc/target/`.
- **Local preview:** run `python3 -m http.server 8000` in `lambda-interpreter-web/`, then open `http://localhost:8000/`. WebAssembly won't load from a double-clicked file.
- **Vercel later:** root directory = repo root, preset **Other**, empty build command.

## Content to use (already supplied)

Example terms, verified against the built package on 2026-10-19:

| Button | Term | Expected result |
| --- | --- | --- |
| 3 + 2 | `λm n f x.(m f (n f x)) λf x.(f (f (f x))) λf x.(f (f x))` | `λf x.(f (f (f (f (f x)))))`, 6 steps, `normal_form` |
| Identity | `λx.x λy.y` | `λy.y`, 1 step, `normal_form` |
| Ω | `λx.(x x) λx.(x x)` | status `timeout` after about 1 s |

Syntax, limits, and wording come from `ulc/README.md`. Links come from `project-brief.md`. Don't mention BusyReaper on the web page. Mentions in the repo (READMEs, notes) are fine.

## Build increments

### 1. Content and structure

- **Build:** create `index.html` with the header, three sections, footer, and all real text. Move the existing example's working interpreter (Run, `\`→`λ`, max steps, Show every step, error display) into *Try it* unchanged.
- **Check:** serve the folder and open `http://localhost:8000/`.
  - The text matches the brief and README, and BusyReaper isn't mentioned.
  - Run on the preloaded term shows `λf x.(f (f (f (f (f x)))))` and 6 steps.
  - A broken term such as `λx.` shows the error message.
- **File:** `index.html` (new).

### 2. Main action and navigation

- **Build:**
  - Try the interpreter jumps to `#try` and puts the cursor in the expression box. The nav links reach each section.
  - The three example buttons fill the box (they don't run it automatically).
  - Add the local-only note.
  - **Cap long output:** with Show every step on, Ω currently returns about 394,000 steps (about 7 MB of text, measured today). Show the first 100 and last 100 steps with a line saying "… N steps hidden …".
- **Check:** use only the keyboard. Tab to Try the interpreter, press Enter, and confirm focus lands in the box. Tab to Run and press Enter.
  - Each example gives the result in the table above.
  - Ω with Show every step on returns within about 1–2 s, shows the hidden-steps line, and 3 + 2 runs normally afterward.
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
  - Record a real-phone check separately (Not tested until done).
- **Files:** `index.html`, `.vercelignore` (new).

## Checkpoints

The folder isn't under git yet. Before increment 1, either run `git init` and commit locally after each passing increment (no push until approved), or copy `index.html` to a dated backup after each increment.

## Missing facts or decisions

None blocking. An optional bio line can be added later.
