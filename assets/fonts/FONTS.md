# Bundled fonts

Inter 4.1 static instances from https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip (`extras/ttf/`), unmodified. License: SIL Open Font License 1.1, `OFL.txt` from the same archive.

JetBrains Mono NL 2.304 from https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip (`fonts/ttf/`), unmodified; the archive's SHA-256 is `6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf`. NL is the build without programming ligatures, so `--` and `->` draw as the two characters typed. License: SIL Open Font License 1.1, the archive's `OFL.txt` as `JetBrainsMono-OFL.txt`.

| File | Family | Weight | SHA-256 |
|---|---|---|---|
| `Inter-Regular.ttf` | Inter | 400 | `40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82` |
| `Inter-SemiBold.ttf` | Inter | 600 | `78a843fade9d4612a5567302fb595b56976eb5fcebf4fea5a5912d638bafcde3` |
| `Inter-Bold.ttf` | Inter | 700 | `288316099b1e0a47a4716d159098005eef7c0066921f34e3200393dbdb01947f` |
| `Inter-ExtraBold.ttf` | Inter | 800 | `e6756ad5690b77606aa62249a7b420d9902d45cae4b0048a24911fd4324b0a22` |
| `JetBrainsMonoNL-Regular.ttf` | JetBrains Mono NL | 400 | `fb3b2575d7b0657359707993288f12a7360344d39387bb26050e276d61f6bd2a` |
| `JetBrainsMonoNL-Bold.ttf` | JetBrains Mono NL | 700 | `0198e841824025f8876e5c297f0b9b497ee8d6eb9969710a3328e1303f996ec3` |

`stencil render` embeds the faces a figure's text uses in its SVG as base64 `data:` URIs, unmodified; each file's `name` table carries the copyright and OFL notices.
