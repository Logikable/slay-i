# Development notes (fork-only)

This fork (`origin` = `Logikable/slay-i`) tracks `upstream`
(`aeubanks/slay-i`) and adds tooling for building a computer player on top of
the simulator: a combat fuzzer now, and later search/heuristics, learned
models, and integration hooks.

## What is and isn't for upstream

- **Upstream-able:** bug fixes and simulator-correctness improvements. These go
  to `upstream` as small, self-contained PRs.
- **Fork-only (do NOT upstream):** the fuzzer (`src/fuzz.rs` + its
  `#[cfg(test)] mod fuzz;` line in `src/main.rs`), any player/search/model code,
  trained-model and data artifacts, and CI/automation hooks.

## How we keep fork-only work out of upstream

1. **Generated artifacts are git-ignored** — models, data, and run logs (see
   `.gitignore`) are never tracked, so they can't be pushed anywhere.
2. **Branch discipline is the real guard.** `origin/main` is our working branch
   and contains everything (fixes + fork-only tooling). PRs are never opened
   from `main`. Instead, each fix gets its own branch cut from
   `upstream/main` with only that fix cherry-picked:

   ```sh
   git fetch upstream
   git switch -c fix/<slug> upstream/main
   git cherry-pick <fix-commit>
   git push -u origin fix/<slug>
   # open PR: Logikable:fix/<slug> -> aeubanks:main
   ```

   Because fork-only commits never get cherry-picked into a `fix/*` branch,
   they cannot reach upstream.

Keep fork-only source clearly separated (its own files/modules) so it's obvious
what must stay behind.
