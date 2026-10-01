# Sources of the imported themes

Each `<name>.base16.yaml` is a byte-for-byte copy of a scheme from the tinted-theming base16 schemes repository, `https://github.com/tinted-theming/schemes`, at commit `d70255b752ac8328ee3d549c72a1a55ce5fc794f`. None was taken from a scheme's own upstream. `<name>.json` and `<name>.report.txt` are the output of `stencil theme import --base16 <name>.base16.yaml --name <name> -o <name>.json` (section 13.9 of `docs/SPEC.md`); `mise run import-themes` re-imports all seven, and the CLI tests fail when a committed file differs from a re-import.

| Theme | Source URL | SHA-256 of the YAML |
|---|---|---|
| tokyo-night | https://raw.githubusercontent.com/tinted-theming/schemes/d70255b752ac8328ee3d549c72a1a55ce5fc794f/base16/tokyo-night-dark.yaml | `98e88daa15855821c156441c1db963e97fbf89d40912b10d11ee2226c9f48755` |
| solarized-light | https://raw.githubusercontent.com/tinted-theming/schemes/d70255b752ac8328ee3d549c72a1a55ce5fc794f/base16/solarized-light.yaml | `9e9ab6cfab64904e85250ae097a52dfd2de7bde320d28a64997e86b1fe42e306` |
| solarized-dark | https://raw.githubusercontent.com/tinted-theming/schemes/d70255b752ac8328ee3d549c72a1a55ce5fc794f/base16/solarized-dark.yaml | `22d8250ac9958985dddcff9f438435a5c0429a672ff86f6fc984332c991bc5a9` |
| material-dark | https://raw.githubusercontent.com/tinted-theming/schemes/d70255b752ac8328ee3d549c72a1a55ce5fc794f/base16/material-darker.yaml | `b427528303ad3b625e0b40b721f7c72f09c9732c831140dcbc53dc0861c5e0ef` |
| gruvbox-dark | https://raw.githubusercontent.com/tinted-theming/schemes/d70255b752ac8328ee3d549c72a1a55ce5fc794f/base16/gruvbox-dark.yaml | `b17930d08392c161d609ba5490d9f52c174ae38321557192d871d5d9fe61d6cc` |
| dracula | https://raw.githubusercontent.com/tinted-theming/schemes/d70255b752ac8328ee3d549c72a1a55ce5fc794f/base16/dracula.yaml | `f6f5a7f3a28a3c305a021328b32a8849ca76cfce3da340dc2c1e69f3cbd8bc17` |
| nord | https://raw.githubusercontent.com/tinted-theming/schemes/d70255b752ac8328ee3d549c72a1a55ce5fc794f/base16/nord.yaml | `bf0620d47f2326576d9f7f28302c9acd67fb4c753c5d496986d34687027c717b` |
