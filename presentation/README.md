# kuda — presentation deck

Slidev deck for **kuda** — *a hippocampus for your codebase* — aimed at
engineers evaluating repo-memory tooling.

Sixteen slides in a claim-then-proof spine: four claims about what the tool
does, each followed by the receipt that proves it (a measured number, a real
card excerpt, a git command, a table of surfaces), then where it sits in the
9aia stack, honest status, a four-command tutorial, and references.

## Run

```sh
pnpm i && pnpm dev    # local loop: slidev slides.md
pnpm build            # static build -> dist/
pnpm export           # PDF export
```

## Theme

The deck uses `slidev-theme-9aia` (installed from
`github:9aia/slidev-theme-9aia` in `package.json`) and sets `theme: 9aia`
in the `slides.md` frontmatter. Layouts come from the theme (`cover`,
`section`, `statement`, `fact`, `quote`, `end`) plus built-ins (`default`).
To change the theme, update the dependency and the `theme:` key together;
no local CSS is used — the theme owns the visual system.
