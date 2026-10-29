# flopha Design

## Direction

flopha is for developers who cut releases from Git: library and app maintainers, and the people who own CI. The landing page has to show what one command does with merged commits. The docs have to get a reader to a first tag quickly.

The direction is **terminal typography**: warm off-white paper, black ink, monospace headlines, and a single green that marks the new version. The octopus mark nods to GitHub and is a sibling of the skul ghost. On the landing page it is drawn on a dot grid, with a stepped path from `v1.2.3` up to `v1.3.0`, so the page explains "the version goes up" before anyone reads a word.

Principles:

- Every demo line is real output. Use the CLI's actual messages and changelog format, never invented UI.
- Green means "the new version" and nothing else.
- Leave space empty rather than fill it. No gradients, glows, shadows, or decorative badges.

[Selected preview](assets/brand/flopha-social-preview.png)

## Visual rules

| Area | Roles and values or source | Usage rules |
| --- | --- | --- |
| Typography | Headings: JetBrains Mono 700. Body: Inter 400–600. Wordmark: Inter 800, letter-spacing −0.03em. | Mono for headlines, section titles, docs headings, code, and the terminal. Inter for paragraphs and UI text. Turn off ligatures in code so `->` stays literal. |
| Color | Tokens in `website/src/css/custom.css`. Light: paper `#fafaf9`, surface `#ffffff`, ink `#151515`, muted `#66655f`, line `#e7e5e4` / `#d6d3d1`, accent `#1a7f37`. Dark: `#0a0a0a`, ink `#ededed`, accent `#3fb950`. Terminal: `#151515` with version green `#3fb950`. | Accent only on the new version: path target, release title, `Released vX`. The primary button is ink on paper. |
| Layout | Docusaurus container; hero grid is copy 1.15fr / dot path 1fr; demo row is terminal 1.5fr / release card 1fr. | Hero copy order: headline → lead → actions → install → supported-platform note. The install command wraps instead of scrolling, so the whole `curl … | sh` stays readable. |
| Shape and elevation | Radius 0.4rem for buttons, 0.6rem for code blocks, the terminal, and cards. 1px borders, no shadows. | Code blocks and the terminal are flat black cards in both themes. |

## Flow and components

Primary flow: landing → `Tag your next release` → Quick Start → first tag (`--initial-version` when the repository has none) → `flopha release` with a `flopha.toml`.

Landing hero (`website/src/pages/index.tsx`) runs one loop that drives three parts:

- **Terminal**: types `git log --oneline v1.2.3..`, two commits, `flopha release`, then prints `Released v1.3.0` and the GitHub Release URL. No title bar or traffic-light dots.
- **Dot path** (SVG): each commit line lights one step. On `Released`, the path reaches `v1.3.0`, the target turns green, and the octopus blinks once. Unreached steps are drawn in `line-strong`.
- **Release card**: appears when the URL prints. It shows the generated changelog (Features / Bug Fixes) for the same commits.

Timing: the finished state holds for 5s, everything fades out over 450ms, and the loop restarts. The server-rendered and no-JS state is the finished frame.

Code blocks: lines always wrap, so there is no horizontal scroll and no word-wrap toggle. Copy is the only button. It stays hidden until hover or focus, sits on a transparent background, and is always visible on touch devices.

Brand assets live in `assets/brand` and are synced to the site by `scripts/sync-brand.mjs`. `flopha-icon.svg` is ink and `flopha-icon-dark.svg` is light. `favicon.svg` switches with the OS color scheme. PNG icons sit on a paper tile.

## Responsive behavior and accessibility

Responsive behavior: at ≤996px the hero becomes one column in this order: copy, dot path (max 300px), terminal, release card. The terminal wraps long lines instead of scrolling. "Two ways to run it" and "Fits the release flow" stack into one column.

Accessibility:

- The terminal and the dot path each expose a text alternative through `role="img"` and `aria-label`.
- With `prefers-reduced-motion`, the finished frame stays static and the cursor and blink animations stop.
- Muted text `#66655f` on paper meets 4.5:1.

## Open decisions

- Docs search: none today. Add Algolia DocSearch only if the docs grow beyond the current six pages.
