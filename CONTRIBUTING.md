# Contributing to KernPage

Thank you for helping. Please read this short page first.

## Licence of contributions
KernPage is available under the MIT License or the Apache License 2.0, at the user's option. Unless you explicitly state otherwise, any contribution you intentionally submit is dual licensed under both, without additional terms (the contribution clause in section 5 of the Apache License 2.0).

## Sign your commits (Developer Certificate of Origin)
Sign off each commit with `git commit -s`. This adds a `Signed-off-by` line with your name and email and means you agree to the Developer Certificate of Origin, the text of which is at https://developercertificate.org/.

## AI coding assistants and the task list
The repository includes configuration files for AI coding assistants (`.claude/`, `.codex/`, `.cursor/`, `.agents/`, and the `AGENTS.md` and `CLAUDE.md` files). They tell an assistant to use `bd` (the beads issue tracker) when it is installed. You do not need `bd` or any assistant to contribute.

The public task list is `.beads/issues.jsonl`. To load it into a local `bd` database, install `bd`, then from the repository root run:

```
bd init --prefix pw
bd import .beads/issues.jsonl
```

`bd import` creates new issues and updates existing ones from the file, so it is safe to run again after pulling a newer list.

## Research rules (important)
- Study Publisher only through its public automation interface, what it shows on screen and files you made yourself. Never look inside Microsoft program code, and never paste or translate code from one.
- Do not copy Microsoft's icons, artwork, templates, clip art, help text, dialog text, screenshots or colour-scheme names. Write feature names and descriptions in your own words (plain command names like "Bold" are fine).
- Learn about the `.pub` format only from files you created yourself and from Microsoft's published Open Specifications (for example [MS-CFB] and [MS-ODRAW]). Do not use `.pub` files you do not have the right to share.
- Do not add code or data from other projects unless its licence is compatible with MIT OR Apache-2.0, and record it in THIRD_PARTY_NOTICES.
- Do not add network calls, telemetry or crash reporting without discussing it first (see PRIVACY.md).
- AI tools: you are responsible for what you submit. Do not submit AI output you cannot show is yours to license, and say in the pull request if AI wrote a significant part.

## Names and trademarks
Use KernPage's own feature names (for example Data tab, Contact Details, Check Publication); see FEATURES.md "Names in KernPage" ([docs/FEATURES.md](docs/FEATURES.md#names-in-kernpage)).

"Microsoft", "Office" and "Publisher" are Microsoft trademarks, used only to name the file format. Do not use them in names, icons or logos.

## Practicalities
Run the tests for the area you changed (`cargo test -p <crate>`); the full suite must pass before a release. Security problems go to SECURITY.md, not to public issues.
