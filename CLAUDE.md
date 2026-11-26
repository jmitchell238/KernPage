# Project Instructions for AI Agents

This file provides instructions and context for AI coding agents working on this project.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:1105d646 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/core-concepts/sync-concepts.md for details and anti-patterns.

## Agent Context Profiles

The managed Beads block is task-tracking guidance, not permission to override repository, user, or orchestrator instructions.

- **Conservative (default)**: Use `bd` for task tracking. Do not run git commits, git pushes, or Dolt remote sync unless explicitly asked. At handoff, report changed files, validation, and suggested next commands.
- **Minimal**: Keep tool instruction files as pointers to `bd prime`; use the same conservative git policy unless active instructions say otherwise.
- **Team-maintainer**: Only when the repository explicitly opts in, agents may close beads, run quality gates, commit, and push as part of session close. A current "do not commit" or "do not push" instruction still wins.

## Session Completion

This protocol applies when ending a Beads implementation workflow. It is subordinate to explicit user, repository, and orchestrator instructions.

1. **File issues for remaining work** - Create beads for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Handle git/sync by active profile**:
   ```bash
   # Conservative/minimal/default: report status and proposed commands; wait for approval.
   git status

   # Team-maintainer opt-in only, unless current instructions forbid it:
   git pull --rebase
   git push
   git status
   ```
5. **Hand off** - Summarize changes, validation, issue status, and any blocked sync/commit/push step

**Critical rules:**
- Explicit user or orchestrator instructions override this Beads block.
- Do not commit or push without clear authority from the active profile or the current user request.
- If a required sync or push is blocked, stop and report the exact command and error.
<!-- END BEADS INTEGRATION -->


## Build & Test

```bash
cargo build --workspace --release
cargo test --workspace
cargo run --release -p kernpage -- samples/kern/showcase-newsletter.kern
cargo run --release -p kernpage-launcher
cargo run -p pubdump -- samples/hello.pub
cargo run -p pub2pdf -- samples/two-boxes.pub two-boxes.pdf
```

Windows is the main platform (Rust 1.100.0, pinned in `rust-toolchain.toml`, on the GNU host: run `rustup set default-host x86_64-pc-windows-gnu` once; MSYS2 UCRT64 gcc on the `PATH`).
Only some `.pub` samples are in the repository. Tests that need a missing one print `skipped: <file>` and pass. To run them,
set the environment variable `KERNPAGE_EXTRA_SAMPLES` to a folder that mirrors `samples/`.

## Architecture Overview

- `crates/kernpage`: the desktop editor (egui): ribbon, canvas, File screen, dialogs.
- `crates/kernpage-core`: editable document model, commands with undo/redo, and the save planner; no GUI code.
- `crates/kernpage-launcher`: opens KernPage and recent files, installs signed updates.
- `crates/kernpage-release`: release tool that signs the update list (not shipped in the installer).
- `crates/pub-format`: reads `.pub` files (Publisher 2003 to 2021).
- `crates/pub-write`: writes changes back into `.pub` files.
- `crates/pub2pdf`: converts a publication to PDF (library and command-line tool).
- `crates/pubdump`: command-line tool that prints what is inside a `.pub` file.
- `crates/metafile`: reads WMF/EMF pictures from their public specifications.
- `crates/kernpage-samples`, `crates/kernpage-samples-macros`: test helpers that find sample files in the repo or in `KERNPAGE_EXTRA_SAMPLES`.
- Icons come from the `kernicons` crate on crates.io. Docs are in `docs/`, the Windows installer in `packaging/windows/`, samples in `samples/`.

## Conventions & Patterns

- Research rule: we learn the `.pub` format only from Publisher's public automation interface, what Publisher shows on screen, and files we made ourselves. We never look inside Microsoft program code.
- The UI uses no Microsoft feature names; write feature names and descriptions in our own words.
