# kuda — presentation deck

Slidev deck for **kuda** — *a hippocampus for your codebase* — aimed at
engineers evaluating repo-memory tooling.

Fifteen slides, problem-first: agent amnesia and why every current fix
rots differently → the indexing insight → the mechanism (tiers, committed
cards, measured staleness, diff-driven sync) → falsifiability → a recorded
terminal demo → honest status.

## Run

```sh
pnpm i && pnpm dev    # local loop: slidev slides.md
pnpm build            # static build -> dist/
pnpm export           # PDF export
```

## Theme

`slidev-theme-9aia` (from `github:9aia/slidev-theme-9aia` in
`package.json`), set via `theme: 9aia` in the frontmatter. Local overrides
live only in `style.css` (asciinema mount sizing + player theme skin).

## Terminal demo

Slide 13 plays `public/casts/terminal-demo.cast` through
`slidev-addon-asciinema`. The player is pinned to `asciinema-player@3.4.0`
in `pnpm-workspace.yaml` — 3.17's canvas renderer does not paint in this
setup; 3.4's DOM renderer does.

To re-record: script the session with `asciinema rec --cols 80 --rows 18`,
emit ANSI colors on prompt lines (`\033[38;5;147m` is the deck's indigo),
and overwrite the cast file.
