# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/frits-v/stencil/compare/v0.1.0...v0.1.1) - 2026-11-09

### Fixed

- *(layout)* spread link ends along a shared side and report shared legs on every flat page ([#151](https://github.com/frits-v/stencil/pull/151))
- *(layout)* keep links and tags off the label of the Box that holds them ([#148](https://github.com/frits-v/stencil/pull/148))

## [0.1.0](https://github.com/frits-v/stencil/releases/tag/v0.1.0) - 2026-11-05

### Added

- *(render)* write a viewer page with --html ([#143](https://github.com/frits-v/stencil/pull/143))
- *(render)* write one SVG that carries several themes with --themes ([#139](https://github.com/frits-v/stencil/pull/139))
- *(examples)* give the hero, transit and stress figures their own subjects ([#137](https://github.com/frits-v/stencil/pull/137))
- *(examples)* make the one-pager and the design brief a library hold-and-pickup service ([#136](https://github.com/frits-v/stencil/pull/136))
- *(render)* set how-to terminals and command chips in JetBrains Mono NL ([#132](https://github.com/frits-v/stencil/pull/132))
- *(render)* write a portable SVG with text as glyph outlines and icons inlined ([#129](https://github.com/frits-v/stencil/pull/129))
- *(render)* fade whole units, delay the hover fade and pin a highlight on click ([#130](https://github.com/frits-v/stencil/pull/130))
- *(examples)* draw the aws and azure figures as a food delivery backend and a hotel reservation platform ([#131](https://github.com/frits-v/stencil/pull/131))
- *(text)* bundle JetBrains Mono NL Regular and Bold beside Inter ([#125](https://github.com/frits-v/stencil/pull/125))
- *(examples)* give the hub-and-spoke, platform and hybrid figures their own subjects ([#127](https://github.com/frits-v/stencil/pull/127))
- *(render)* highlight what any element refers to on hover in the written SVG ([#119](https://github.com/frits-v/stencil/pull/119))
- *(grammar)* add an oss grammar with open-source project logos as its pack ([#120](https://github.com/frits-v/stencil/pull/120))
- *(render)* highlight an item and its links on hover in the written SVG ([#117](https://github.com/frits-v/stencil/pull/117))
- *(grammar)* add an azure grammar with the Azure architecture icons as its pack ([#114](https://github.com/frits-v/stencil/pull/114))
- *(grammar)* add an aws grammar with the AWS Architecture Icons as its pack ([#113](https://github.com/frits-v/stencil/pull/113))
- *(model)* scope icons by pack, one directory per pack ([#112](https://github.com/frits-v/stencil/pull/112))
- *(examples)* move the one-pager and the design brief to the plain grammar ([#107](https://github.com/frits-v/stencil/pull/107))
- [**breaking**] make plain the default grammar and gcp a named one ([#105](https://github.com/frits-v/stencil/pull/105))
- *(cli)* add prime onepager and route one-pagers to the design-sheet template ([#99](https://github.com/frits-v/stencil/pull/99))
- *(layout)* add junctions-plain, a template check on the shape lines make together ([#98](https://github.com/frits-v/stencil/pull/98))
- show what an operator types in how-tos with a terminal ([#92](https://github.com/frits-v/stencil/pull/92))
- escalate through a detail ladder framed in the workflow ([#91](https://github.com/frits-v/stencil/pull/91))
- assembly outlines, rails, valves, step badges and planned lines on the design sheet ([#90](https://github.com/frits-v/stencil/pull/90))
- marks, a parts list and general notes on the design sheet ([#89](https://github.com/frits-v/stencil/pull/89))
- *(render)* let lanes and steps of a swimlane go transparent ([#87](https://github.com/frits-v/stencil/pull/87))
- actors and a Phases swimlane on the design sheet ([#85](https://github.com/frits-v/stencil/pull/85))
- *(render)* lighten the design sheet and fit the title block ([#84](https://github.com/frits-v/stencil/pull/84))
- draw a page as a design sheet with a title block and numbered views ([#83](https://github.com/frits-v/stencil/pull/83))
- *(text)* measure a body run that continues a bold lead's line ([#82](https://github.com/frits-v/stencil/pull/82))
- *(render)* describe every figure in Markdown ([#80](https://github.com/frits-v/stencil/pull/80))
- *(model)* [**breaking**] replace the required canvas with an optional free-text badge ([#78](https://github.com/frits-v/stencil/pull/78))
- *(render)* draw a routed link as one smooth curve with bend: spline ([#74](https://github.com/frits-v/stencil/pull/74))
- *(render)* round routed link bends with a page or line-kind corner ([#73](https://github.com/frits-v/stencil/pull/73))
- *(cli)* render the PNG at an exact width with --png-width ([#72](https://github.com/frits-v/stencil/pull/72))
- *(iso)* an untargeted pipe end on a zone of devices is a defect, slots aim at attach boxes, and transit draws one Interconnect from the router to the hub ([#62](https://github.com/frits-v/stencil/pull/62))
- *(iso)* a block casts a route obstacle grown by its height, and the global product list keeps only the hub ([#61](https://github.com/frits-v/stencil/pull/61))
- *(gcp)* serverless and global products sit outside the VPC and the region; examples drawn and worded as systems ([#60](https://github.com/frits-v/stencil/pull/60))
- *(iso)* a deny line ends in a stop plate and carries no ring; on-prem trunks land on their VLANs ([#59](https://github.com/frits-v/stencil/pull/59))
- *(iso)* iso-labels-clear reads the drawn geometry, and link groups sit half a block back on hidden sides ([#58](https://github.com/frits-v/stencil/pull/58))
- *(iso)* iso-links-clear reports detours, jogs and legs over foreign blocks ([#57](https://github.com/frits-v/stencil/pull/57))
- *(iso)* iso-links-apart, link ends spread along a shared side, bundles side by side, and a tube comes out of its ring ([#56](https://github.com/frits-v/stencil/pull/56))
- *(iso)* iso-link-ends reads every drawn link end, and a link lands on a sprite's hull ([#55](https://github.com/frits-v/stencil/pull/55))
- *(iso)* items stand as blocks with floor text; stubs stop short of each other ([#50](https://github.com/frits-v/stencil/pull/50))
- *(iso)* stubs stop at the nearest obstacle, zone labels clear the tallest solid, transit example (iso v2 step 4) ([#47](https://github.com/frits-v/stencil/pull/47))
- *(iso)* band pipes, flat wide arrows on the floor (iso v2 step 3c) ([#46](https://github.com/frits-v/stencil/pull/46))
- *(iso)* figure, laptop and phone sprites; person and device kinds (iso v2 step 3b) ([#44](https://github.com/frits-v/stencil/pull/44))
- *(iso)* pipes and tee spines as tubes with flange rings and cones (iso v2 step 3a) ([#42](https://github.com/frits-v/stencil/pull/42))
- *(iso)* shape vocabulary (iso v2 step 2) ([#41](https://github.com/frits-v/stencil/pull/41))
- *(iso)* approach stubs, open floor behind and left of blocks, line rules, frame tier, dusk inks ([#40](https://github.com/frits-v/stencil/pull/40))
- *(iso)* everything lies on the isometric plane (iso v2 step 1) ([#39](https://github.com/frits-v/stencil/pull/39))
- *(cli)* base16 theme import and the imported theme tier (step b2) ([#37](https://github.com/frits-v/stencil/pull/37))
- *(render)* break opportunities, isometric shadows and link exits (step e) ([#36](https://github.com/frits-v/stencil/pull/36))
- *(cli)* print-fit, icon matching, chrome none and pipe targets (step d) ([#33](https://github.com/frits-v/stencil/pull/33))
- *(render)* themes as data with six built-in themes (step b) ([#31](https://github.com/frits-v/stencil/pull/31))
- *(layout)* lanes with ordered links and plain grammar figures (step c) ([#30](https://github.com/frits-v/stencil/pull/30))
- *(model)* core vocabulary and grammars as data (step a) ([#29](https://github.com/frits-v/stencil/pull/29))
- *(render)* isometric projection as a render option ([#25](https://github.com/frits-v/stencil/pull/25))
- *(cli)* gallery subcommand, docs embeds and release publishing ([#24](https://github.com/frits-v/stencil/pull/24))
- *(cli)* add stencil prime, an authoring briefing for agents
- *(render)* draw document blocks and links in every theme
- *(cli)* run the link checks and report not-applicable checks
- *(cli)* add --theme to render and check
- *(cli)* vet, render, check, schema and the g7 golden test

### Fixed

- *(icons)* draw every oss icon as its project's symbol ([#133](https://github.com/frits-v/stencil/pull/133))
- *(render)* give the written SVG's text a sans fallback stack where the faces are blocked ([#123](https://github.com/frits-v/stencil/pull/123))
- *(layout)* keep link legs clear of every drawn outline on a flat page ([#97](https://github.com/frits-v/stencil/pull/97))
- end a blocked line in a stop plate on a flat page and keep repeated parts apart in the measured JSON ([#96](https://github.com/frits-v/stencil/pull/96))
- *(examples)* draw the onepager's steps opaque on transparent lanes and close its gaps to the target ([#95](https://github.com/frits-v/stencil/pull/95))
- *(render)* mark a terminal's ok lines so they read apart from output in every theme ([#93](https://github.com/frits-v/stencil/pull/93))
- *(layout)* straight approach stub at arrowed link ends ([#79](https://github.com/frits-v/stencil/pull/79))
- *(render)* embed the Inter faces a figure uses in its SVG ([#71](https://github.com/frits-v/stencil/pull/71))
- *(iso)* a pipe end lands on the nearest solid across its center, and pipes-land reads the run axis ([#53](https://github.com/frits-v/stencil/pull/53))
- *(render)* iso link routing, zone tabs and weights for the hero ([#27](https://github.com/frits-v/stencil/pull/27))
- *(render)* iso links, label placement and slab paint for the hero ([#26](https://github.com/frits-v/stencil/pull/26))
- *(layout)* split the g7 on-prem column and add the pipes-land check
- bound legend walks, guard output directories and cap input size
- *(cli)* harden input decoding and output writes
- *(render)* bound the PNG pixmap and name unresolved font lookups

### Other

- release pull requests, tags and archives through release-plz and dist ([#145](https://github.com/frits-v/stencil/pull/145))
- *(examples)* keep the block one-pager as design-brief ([#81](https://github.com/frits-v/stencil/pull/81))
- *(prime)* export the field a figure binds its page to, not g7's customer ([#77](https://github.com/frits-v/stencil/pull/77))
- Write each link's drawn path into the measured JSON under iso ([#69](https://github.com/frits-v/stencil/pull/69))
- Land links on a laptop's base plate, thick enough to read, and report a block on a slab's lip ([#68](https://github.com/frits-v/stencil/pull/68))
- Keep managed data products with a private IP out of the VPC and draw the platform database beside it ([#67](https://github.com/frits-v/stencil/pull/67))
- Redraw the on-prem example around a compact firewall and let tags and landing pipes pass their checks ([#66](https://github.com/frits-v/stencil/pull/66))
- Move a Z's step off the stub and report steps and hooks in iso-links-clear ([#65](https://github.com/frits-v/stencil/pull/65))
- Key legend consistency on the pipe form and draw a bar swatch for a band entry ([#63](https://github.com/frits-v/stencil/pull/63))
- *(prime)* an iso topic with the shapes, pipes, links, text, size and check rows ([#54](https://github.com/frits-v/stencil/pull/54))
- *(examples)* remove engagement framing from example figures ([#20](https://github.com/frits-v/stencil/pull/20))
- cover pipes-land in the prime briefing and pass the PR number to the land task
- *(prime)* list themes one per line and complete the part names
- *(render)* format and silence constant assertions
- add contract spec, API notes, CUE schema, fonts and g7 fixtures
- scaffold workspace with icons, fonts and empty crates
