# Website scope — Lambda Interpreter Demo

- **Website name:** Lambda Interpreter Demo (working title)
- **Visitor and outcome:** Connections, professors, employers, and curious visitors run a lambda term in the browser, see it reduce to normal form, and can tell who built it and where the source is.
- **Real or fictional:** Real. Perrin's own interpreter and links, with no invented content.
- **Starter or own design:** Own design. It grows out of the working demo in `ulc/examples/index.html`, uses the prebuilt `ulc/pkg/`, and uses no course starter.
- **Folder / repo:** `lambda-interpreter-web/` → https://github.com/pda-sedano/lambda-interpreter-web (public)

## First version

One page with a header, a menu bar, and two main sections (an About section was dropped on 2026-10-19 because the menu bar covers it):

1. **Try it:** the interpreter, with an expression box, Run, a "show every step" toggle, an optional max-steps field (empty, or a whole number from 0 to 4,294,967,295), a one-line status (step count and stop status, or the error), and the step trace.
2. **How it works:** a few plain sentences on normal-order reduction and de Bruijn indices, plus a compact syntax reference (`λx.M`, `λx y.M`, application, `x^n`, `\` types `λ`, a `λ` body is a single term) and the limits (depth 1000, 1-second timeout).
- **Menu bar:** **View source** (this repo) on the left; "My other stuff:" GitHub (https://github.com/pda-sedano) and LinkedIn (https://www.linkedin.com/in/perrin-alaine-sedano-4589b31b5/) on the right. The header byline credits Perrin.

- **Primary action:** **Run**, in *Try it*, directly under the header and menu bar. The expression box is preloaded with Church 3 + 2, and Run evaluates it in the visitor's own browser.
- **Navigation:** the menu bar under the title and subheading (no in-page section links).
- **Demonstration vs. real service:** the interpreter is real and runs locally in WebAssembly. Nothing is sent to a server, saved, or collected; the footer says "Runs entirely in your browser via WebAssembly."

**Change 2026-10-19 (Perrin, increment 2):** dropped the Try the interpreter button (the interpreter is right below the header) and the separate local-only note (the footer's "Runs entirely in your browser via WebAssembly." is enough). Added a sticky menu bar under the title and subheading. Revised the same day: section links removed; "View source" on the left, "My other stuff: GitHub LinkedIn" on the right; the About section removed.

| Must-have now | Later | Out of scope |
| --- | --- | --- |
| The two sections and the menu bar above | Link to Perrin's main personal website (once it's ready) | Mentioning BusyReaper |
| Preloaded example: Church 3 + 2 | Animated step-by-step playback (`Evaluator`) | Server-side evaluation, accounts, saving terms |
| 3 one-click examples: 3 + 2, identity applied to a term, Ω (to show the timeout) | Showing the de Bruijn form (`parse().deBruijn`) | Contact form, newsletter, analytics or tracking |
| Readable errors for syntax and for an invalid max-steps value; the page keeps working afterward | Shareable links that encode a term in the URL | Typed lambda calculus or other languages |
| Bounded runs: 1 s timeout (the page may freeze for up to ~1.5 s, then accepts input); trace capped at the first and last 100 steps and 50,000 characters | Longer runs in a Web Worker | Paid services or a custom domain purchase |
| GitHub, LinkedIn, and View source links | Short bio line (if Perrin writes one) | |
| Works at phone width and with the keyboard; screen readers hear only the short status line, not the whole trace | Dark mode / visual polish beyond clean and readable | |

## Five acceptance checks

| Check | What I will do | What I expect to observe |
| --- | --- | --- |
| Purpose | Show the page to someone for about 30 seconds | They can say it's a lambda calculus interpreter they can try, built by Perrin. |
| Accurate content | Read the page against `project-brief.md` and `ulc/README.md` | Syntax, limits, and links match. BusyReaper isn't mentioned. Nothing is invented. |
| Main action: real result, not a demo | Press Run on the preloaded example. Then run an invalid term, set max steps to `-1`, run Ω, then 3 + 2 again | 3 + 2 gives `λf x.(f (f (f (f (f x)))))` in 6 steps. Invalid input shows the error. `-1` shows "Max steps must be a whole number from 0 to 4,294,967,295 …". Ω stops with status `timeout` after the page freezes for no more than ~1.5 s, then it accepts input again. The final run still works. The footer's in-browser note is visible. |
| Navigation and keyboard | Follow every menu link. Tab through the page and press Enter on the example buttons and on Run | View source, GitHub, and LinkedIn open the right pages. Focus is visible, and every control works from the keyboard. Only the status line has `aria-live`. |
| Narrow screen | Narrow the browser to phone width (real phone if available, recorded separately) | Text is readable, long output wraps or scrolls inside its box, and nothing spills sideways. |

**Local testing note:** WebAssembly doesn't load from `file://`, so serve the folder with `python3 -m http.server` and test at `http://localhost:8000/`.

**Unresolved:** none. (Optional one-line bio, if Perrin writes one later.)

**Decisions confirmed 2026-10-19:** keep Ω as a must-have example; step-by-step animation stays in Later.

**Next step:** plan the build increments (vc-project-plan) within these limits.
