# Noto Sans (fallback interface font)

KernPage draws its own interface text with Segoe UI from the Windows fonts folder. When Segoe UI is
missing it uses this font instead. Only the Regular weight is shipped (the interface never asks for a
bold face: egui's "strong" text is a colour change, not a heavier font).

| | |
|---|---|
| Font | Noto Sans Regular, version 2.015 (Latin, Greek, Cyrillic) |
| File | `NotoSans-Regular.ttf`, the **unhinted** static TrueType build, unmodified |
| Source | https://github.com/notofonts/latin-greek-cyrillic/releases/tag/NotoSans-v2.015 |
| Download | https://github.com/notofonts/latin-greek-cyrillic/releases/download/NotoSans-v2.015/NotoSans-v2.015.zip (zip SHA-256 `0C34DF072A3FA7EFBB7CBF34950E1F971A4447CFFE365D3A359E2D4089B958F5`), entry `NotoSans/unhinted/ttf/NotoSans-Regular.ttf` |
| Font file SHA-256 | `F3961A9CDE016D41A4879AECDA1474D3A36D6BF54FA0E4643DE029CC2248B0E8` |
| `OFL.txt` SHA-256 | `CEE9892F9F0CC8FE882C9E9537EE6A89621D86EE7CEAF70B02E2B2B1C25C061A` (the file at the root of the same zip, identical to `OFL.txt` on the repository's main branch) |
| Copyright | Copyright 2022 The Noto Project Authors (https://github.com/notofonts/latin-greek-cyrillic) |
| Licence | SIL Open Font License 1.1 (`OFL.txt`), no Reserved Font Name |

The font is **not** compiled into the KernPage programs. The installer puts it, with `OFL.txt`, in the `fonts`
folder next to `kernpage.exe`; the programs read it from there at start-up. Do not modify the file: a
modified version would have to be renamed under the OFL.

To check the file: `Get-FileHash NotoSans-Regular.ttf` (PowerShell) and compare with the value above.
