# Themes

A theme sets colors only. Layout, text sizes, icons and the measured JSON are identical under every theme, so pick it last. Set "theme" on the Page (default center), or pass `--theme center|dusk|wire` to render and check; the flag wins over the page.

| Theme | Use for | Look |
|---|---|---|
| center | customer-facing slides and documents on white | Google Cloud Architecture Center stencil: white page, blue gcp bar, tinted zones |
| dusk | dark slides and screens | navy page, light ink, darkened tints; each icon sits on a white 36 px chip so dark icons stay visible |
| wire | design docs, reviews, print | white page, #222222 ink and borders, no fills; kinds differ by line style only |

| Role | center | dusk | wire |
|---|---|---|---|
| page | #FFFFFF | #0B1220 | #FFFFFF |
| ink, secondary ink | #202124, #5F6368 | #E6EDF7, #9AA7BD | #222222, #555555 |
| card fill, border | #FFFFFF, #DADCE0 | #111A2E, #2A3550 | white, #222222 |
| fact box, ask box | #F1F3F4, #FEF7E0 | #182238 | white |
| gcp frame and bar | #1A73E8 bar, #FAFBFC body | #1A73E8 bar, #0F172A body | 2 px border, white bar |
| tint a: region-a, onprem-a | #D2E3FC | #14213A | none |
| tint b: region-b, onprem-b, k8s | #FCE4EC | #2A1626 | none |
| subnet | #EDE7F6, dashed | #1D1836, dashed | dotted |
| project | #FFF8E1 | #1F1B10 | solid |
| optional | #F8FBFF, dashed blue | #10203A, dashed blue | dashed |
| perimeter | #FFFBF5, dashed orange | #17130B, dashed orange | dashed |
| gray wire | #5F6368 | #9AA7BD | thin solid |
| blue wire | #1A73E8 | #5B9CFF | 2 px solid |
| pink wire | #C2185B | #FF5C8A | 2 px, hollow end dots |
| dash wire | #1A73E8 dashed | #5B9CFF dashed | dashed |
| deny wire | #C5221F dashed, red tag | #FF6B6B dashed | dotted |
| callout note, risk, decision, open | accents #1A73E8, #C5221F, #188038, #B06000 on light tints | same accents on dark tints | #222222 accent, white fill |

Kind meanings hold in every theme: gray for internal calls, blue for tint a paths, pink for tint b paths, dash for failover, control plane or identity, deny for a blocked path. Keep Metro 1 and Region A blue and Metro 2 and Region B pink end to end.

Under wire:

- A legend entry's label is fixed per kind ("Solid blue", "Dashed red") and still reads that way next to a black swatch, so write the legend text as meaning ("request path"), never as color.
- Links have no end dots, so a blue and a pink link draw the same. Tell them apart by label.
- A Callout shows its kind only through the title you write: "Risk: ...", "Decision: ...".
