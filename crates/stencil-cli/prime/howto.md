# How-tos

A HowTo shows what an operator types: a View child holding 1 to 6 items, numbered `<view>.<n>` on the sheet. Items lie in equal columns, as many to a row as fit their widest terminal line and heading on the sheet's width, the rows balanced (four that fit three to a row lie two and two).

- Heading: the number, the title in capitals, and right of it the command line: up to 5 `commands`, each a string drawn as a chip or {words} drawn as plain text between chips, set on one line.
- Body: `text` above the terminal, or beside it with `beside: true`, which needs both.
- Terminal: up to 12 lines, each a `kind` and its text. A command line draws a `$` prompt and a comment a `#`; output and ok lines draw none and are told apart by colour: output in the terminal's text ink, ok in its prompt ink, a comment in its comment ink. `indent` (0 to 8) sets a line in by two spaces a level; the text itself stays trimmed. Lines are Inter Regular, so columns of output do not align.
- Nothing wraps on the command line or in the terminal. A line wider than its column, or a command line that does not fit beside the title, is a text-fits-box defect: shorten it, or give the HowTo a wider parent.
- The terminal and chips take the theme's `terminal` role: fill, text, prompt and comment. A theme without it fills with `fact.fill` and draws in the page inks.
- The description writes each item as a section with its commands, its text and its terminal in a fence, a line led by its kind's prefix.

{{vocabulary:HowTo,HowToItem,TerminalLine,TerminalKind}}

# Vet

| Rule | Defect at | Message |
|---|---|---|
| needs-sheet | the node | how-to is drawn on a design sheet only; add sheet or remove it |
| how-to-item-empty | the item | a how-to says or shows something |
| how-to-beside-alone | beside | beside sets text beside a terminal; the item needs both |
| terminal-indent-out-of-range | indent | indent N is above 8 |
| list-empty | items | how-to-items is empty |
| list-too-long | the list | N how-to-items, above 6 (command-runs 5, terminal-lines 12) |
