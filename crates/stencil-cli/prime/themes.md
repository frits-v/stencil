# Themes

A theme sets paint only. Layout, text sizes, icons and the measured JSON are identical under every theme, so pick it last. Set "theme" on the Page (default center) to a built-in name or a theme file path ending in .json, or pass `--theme` to render and check; the flag wins over the page.

- center: Architecture Center on white; customer slides and documents, the default.
- paper: warm paper with one saturated blue; print and long documents.
- dusk: deep navy with a widened surface ladder; dark slides and screens.
- clear: tints chosen to stay apart under color-vision deficiency, light.
- clear-dark: the same, dark.
- wire: one ink, white fills, lines told apart by pattern and end dots; design docs and reviews.

The imported tier maps editor color schemes onto the same roles. None passes the colorblind or contrast thresholds; `stencil theme check <name>` lists every failing row. Use one only when a reader asked for that look.

- tokyo-night: editor heritage, below the colorblind bar; navy with cyan and lavender.
- solarized-light: editor heritage, below the colorblind bar; cream with muted accents.
- solarized-dark: editor heritage, below the colorblind bar; deep teal with the same accents.
- material-dark: editor heritage, below the colorblind bar; near-black with pastel accents.
- gruvbox-dark: editor heritage, below the colorblind bar; warm brown-gray with earthy accents.
- dracula: editor heritage, below the colorblind bar; purple-gray with saturated accents.
- nord: editor heritage, below the colorblind bar; slate with frosted blues.

Tint slots. A Box tint or a solid or dash line's tint picks one of eight slots. Center names them, and legend labels in every theme are measured with these names:

| Slot | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| center | blue | pink | teal | amber | violet | green | orange | cyan |

Other themes paint the same slot in their own color and name the legend label after it ("Solid rose" in paper). Under wire a label names the line instead: "Solid line", "Ringed line" (hollow dots), "Plain line" (no dots), "Dashed line", "Thin line", "Dotted line". Write legend text as meaning ("request path"), never as color. Keep a paired path on one slot end to end: Metro 1 and Region A on tint 1, Metro 2 and Region B on tint 2.

Theme files. `stencil theme show <name>` prints a built-in theme's JSON; copy it as the start of a theme file. `stencil theme check <theme>` prints every contrast and separation row and exits 1 when one fails; render and check never fail on contrast. A theme file that breaks a structural rule (lowercase hex, a width outside 0.5 to 4, an unknown role) stops every command with `violation` lines.

Overrides. "theme_overrides" on the Page is a partial theme merged onto the chosen theme: objects merge key by key, other values replace. `tints` takes an object keyed by slot number, so `{"tints": {"3": {"wire": "#00796B"}}}` recolors one wire. Colors are uppercase #RRGGBB.

Under wire, links have no end dots, so two link tints draw the same; tell them apart by label. A Callout shows its kind only through the title you write: "Risk: ...", "Decision: ...".

Projection: "projection": "iso" (or render and check with --projection iso) draws the body as a 30 degree isometric scene: filled Boxes become slabs, an untinted Box of the strong tone (gcp vpc) becomes a ring on its parent, items become blocks, links run over the slabs to their endpoint blocks, and every label stays upright. A body narrower than the canvas is zoomed, up to 1.6 times, until it spans 80 percent of it; text keeps its size. Every Box label is a tab on the Box's back corner. Layout and the flat checks do not change; check adds iso-labels-clear and iso-links-clear, which fails when a link leg runs within 24 px of a parallel Box edge. Leave open floor: Row grow 0, items 64 apart, a gap between Boxes of at least 48 px for a link leg along it, and overlapping spans for blocks joined straight across.
