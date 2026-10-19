# Website scope — Lambda Interpreter Demo

- **Website name:** Lambda Interpreter Demo (working title)
- **Visitor and outcome:** Connections, professors, employers, and curious visitors run a lambda term in the browser, see it reduce to normal form, and can tell who built it and where the source is.
- **Real or fictional:** Real. Perrin's own interpreter and links, with no invented content.
- **Starter or own design:** Own design. It grows out of the working demo in `ulc/examples/index.html`, uses the prebuilt `ulc/pkg/`, and uses no course starter.
- **Folder / repo:** `lambda-interpreter-web/` → https://github.com/pda-sedano/lambda-interpreter-web (public)

## First version

One page with a heading and three main sections:

1. **Try it:** the interpreter, with an expression box, Run, a "show every step" toggle, an optional max-steps field, and the output (steps, step count, stop status).
2. **How it works:** a few plain sentences on normal-order reduction and de Bruijn indices, plus a compact syntax reference (`λx.M`, `λx y.M`, application, `x^n`, `\` types `λ`, a `λ` body is a single term) and the limits (depth 1000, 1-second timeout).
3. **About:** "Built by Perrin Alaine-Sedano", plus links to GitHub (https://github.com/pda-sedano), LinkedIn (https://www.linkedin.com/in/perrin-alaine-sedano-4589b31b5/), and View source (this repo).

- **Primary action:** **Try the interpreter**, a button at the top.
- **What it does:** scrolls to *Try it* and puts the cursor in the expression box, which is preloaded with Church 3 + 2. Pressing **Run** evaluates it in the visitor's own browser.
- **Demonstration vs. real service:** the interpreter is real and runs locally in WebAssembly. Nothing is sent to a server, saved, or collected, and the page says so in one line under *Try it*.

| Must-have now | Later | Out of scope |
| --- | --- | --- |
| The three sections above and the Try the interpreter button | Link to Perrin's main personal website (once it's ready) | Mentioning BusyReaper |
| Preloaded example: Church 3 + 2 | Animated step-by-step playback (`Evaluator`) | Server-side evaluation, accounts, saving terms |
| 3 one-click examples: 3 + 2, identity applied to a term, Ω (to show the timeout) | Showing the de Bruijn form (`parse().deBruijn`) | Contact form, newsletter, analytics or tracking |
| Readable syntax errors; the page keeps working afterward | Shareable links that encode a term in the URL | Typed lambda calculus or other languages |
| Bounded runs (1 s timeout) so the page never freezes | Longer runs in a Web Worker | Paid services or a custom domain purchase |
| GitHub, LinkedIn, and View source links | Short bio line (if Perrin writes one) | |
| Works at phone width and with the keyboard | Dark mode / visual polish beyond clean and readable | |

## Five acceptance checks

| Check | What I will do | What I expect to observe |
| --- | --- | --- |
| Purpose | Show the page to someone for about 30 seconds | They can say it's a lambda calculus interpreter they can try, built by Perrin. |
| Accurate content | Read the page against `project-brief.md` and `ulc/README.md` | Syntax, limits, and links match. BusyReaper isn't mentioned. Nothing is invented. |
| Main action: real result, not a demo | Press Try the interpreter, then Run on the preloaded example. Then run an invalid term, then Ω, then 3 + 2 again | Focus lands in the box. 3 + 2 gives `λf x.(f (f (f (f (f x)))))` in 6 steps. Invalid input shows the error. Ω stops with status `timeout` and the page stays responsive. The final run still works. The local-only note is visible. |
| Navigation and keyboard | Follow every link. Tab through the page and press Enter on the button and on Run | GitHub, LinkedIn, and View source open the right pages. Focus is visible, and every control works from the keyboard. |
| Narrow screen | Narrow the browser to phone width (real phone if available, recorded separately) | Text is readable, long output wraps or scrolls inside its box, and nothing spills sideways. |

**Local testing note:** WebAssembly doesn't load from `file://`, so serve the folder with `python3 -m http.server` and test at `http://localhost:8000/`.

**Unresolved:** none. (Optional one-line bio, if Perrin writes one later.)

**Decisions confirmed 2026-10-19:** keep Ω as a must-have example; step-by-step animation stays in Later.

**Next step:** plan the build increments (vc-project-plan) within these limits.
