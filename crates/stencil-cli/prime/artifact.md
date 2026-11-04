# Publish a figure as a hosted page

`stencil render fig.json --out-dir out --html` writes out/fig.html beside the SVG, PNG, measured JSON and description: one file with the SVG inlined, its fonts embedded, the measured JSON and the description as data, and a viewer script. It makes no external request, so it works as a local file, a wiki or pull-request attachment, a GitHub Pages file or a Claude artifact. Add `--themes center,clear-dark` (two or more, a built-in name or a theme file each) for a theme menu; the first theme is the default and System follows the reader's color scheme.

# Claude artifact

Publish out/fig.html with the Artifact tool as the page, with no supporting files: everything the page needs is inside it. A design sheet with eight themes is about 3 MB, under the 16 MB limit. The tool wraps the file in its own skeleton; the page is a full document, and the parser drops the nested html, head and body tags, so it renders as written. The title sits in the first 8 KB, so the artifact takes the document's title; pass a one-word icon such as map and a one-sentence description. The artifact is private until shared from its page. A link ending in #<id> opens with that element pinned.

# What the reader gets

- Pan: drag, scroll or the minimap. Zoom: Ctrl-scroll, pinch, + and −. Width and Page fit the figure; 1 to 9 jump to the title block and the views of a sheet.
- Hover previews what an element refers to, with a tooltip of its text; click pins it, Shift-click pins several, Esc clears. The Selection tab lists the pins with Go to and Unpin. Tab walks the figure in reading order and Enter pins.
- Filters: each legend entry and actor dims what it refers to.
- Find (/) searches every text run; G goes to a grid cell such as B6 or B2-C5 on a sheet.
- Text: the description, with every pointer and grid reference as a button that pins or pans.
- The status line reports the hover rules, measured nodes and text runs it read, and turns to a warning when one came back empty: a page without pins is a defect, not a property of the figure.

# When to offer it

Hand the page over when the reader works in a browser and the figure is dense: a design sheet, a figure with more than a dozen lines, or a bundle whose themes the reader should choose between. The PNG stays the deliverable for a document or a slide, and the SVG for a page that embeds it.
