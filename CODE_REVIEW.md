# Code Review: `vers` v0.5.0

- **Review date:** 2026-11-04
- **Commit reviewed:** `94d3eee` (`main`)
- **Scope:** 21 Rust source files, ~1,990 lines
- **Toolchain used for verification:** Rust/Cargo 1.70.0

---

## Verdict

**A cleanly structured but early-stage prototype — roughly 4/10 as production software, 6–7/10 as a learning/portfolio project.**

The core idea (per-environment symlink farms over a shared GitHub-release tool cache) is coherent, the codebase is small and readable, and `cargo fmt` is clean. A normal `.tar.gz` install works end-to-end on macOS.

However, several core commands are broken in ways a user will hit immediately: `remove` leaves dangling symlinks and ignores `--link-only`, `sync` is effectively a no-op, raw-binary releases report success while installing nothing, failed installs exit `0`, and `--file-filter` does not behave as documented. Dependencies are three years stale with active advisories, and the CI configuration cannot pass on stable Rust.

| Area                    | Score | Notes                                                                       |
| ----------------------- | ----: | --------------------------------------------------------------------------- |
| Core functionality      |  4/10 | Happy path works; `remove`, `sync`, raw assets, and error signalling do not |
| Architecture            |  7/10 | Good module separation; persistence and path model are weak                 |
| Error handling / UX     |  2/10 | Panics, false successes, debug backtraces, inconsistent exit codes          |
| Security / supply chain |  3/10 | No verification; vulnerable archive crates; unsafe path handling            |
| Testing                 |  3/10 | 31 tests, but only `version` and `dirs`; no core-flow tests                 |
| Maintenance / release   |  2/10 | Last commit Sep 2023; broken CI; dead flags; no license files               |
| Documentation           |  4/10 | Good conceptual README, but several examples are wrong                      |

---

## What is done well

- Small, understandable modules with one responsibility each (`actions/`, `archiver/`, `environment.rs`, `github.rs`, etc.).
- Custom `thiserror` error enums with structured context fields.
- `Archiver` trait plus runtime detection is a good extension point.
- Thorough table-driven unit tests for `Version` parsing (`src/version.rs`).
- Sensible CLI design using `clap` derive, shell completions, verbosity flags, and JSON/text/table list output.
- No `unsafe` code.
- The multi-environment + shared version store concept is the real differentiator versus `asdf`/`mise`, `ubi`, or `eget`.

Verified working install:

```text
vers add BurntSushi/ripgrep@13.0.0 --alias rg -f rg
# → symlink created and `rg --version` runs successfully
```

---

## Verification performed

| Check                                                                     | Result                                          |
| ------------------------------------------------------------------------- | ----------------------------------------------- |
| `cargo fmt --check`                                                       | Pass                                            |
| `cargo test --workspace`                                                  | Pass — 31 tests                                 |
| `cargo clippy --workspace --all-targets --all-features -- -D clippy::all` | Fail — 4 lints                                  |
| End-to-end `.tar.gz` install (ripgrep 13.0.0, macOS)                      | Pass                                            |
| Raw-binary install (`jq-macos-amd64`)                                     | Fail — reports success, installs nothing        |
| `--file-filter '^rg$'`                                                    | Fail — treated as a literal filename            |
| `remove` / `remove --link-only`                                           | Fail — dangling symlink remains, assets deleted |
| `sync` on missing files                                                   | Fail — no-op                                    |
| Failed install exit code                                                  | Fail — exits `0` and dumps a backtrace          |
| Relative `--data-dir`                                                     | Fail — broken relative symlink                  |
| OSV scan of `Cargo.lock` (352 registry crates)                            | 30 packages with advisories (106 advisory IDs)  |

---

## Confirmed high-impact defects

### 1. `remove` never unlinks the tool, and `--link-only` does the opposite

`src/actions/remove.rs:14` builds the link path from `env_tool.name`:

```rust
let link_path = dirs::get_tool_link_path(env_path, &env_tool.name);
```

The actual link is created from the **alias** in `Environment::add_tool`. Since names are always `owner/repo`, this resolves to `envs/global/owner/repo` instead of `envs/global/<alias>`, so the real link is never found. The version directory is then deleted, leaving a dangling symlink on `PATH`.

`--link-only` is parsed but discarded (`src/cli.rs:173`, `link_only: _link_only`) and never reaches `remove_tool`. Reproduced behavior: `remove --link-only` deleted the downloaded assets and left the link intact — the opposite of the documented behavior.

Also, removing one version removes the entire `Tool` entry, orphaning other installed versions.

### 2. Raw-binary releases silently “succeed” without installing anything

`determine_possible_extractors` only recognises `.tar.gz`, `.tar`, `.zip`, and (incorrectly) `.tar.gz` again. For a raw binary asset, the extractor loop is empty, then `Environment::add_tool` falls through to `Ok(())` (`src/environment.rs:224`). `add_new_tool` prints “Installation … complete” and exits `0`.

Reproduced:

```text
vers add jqlang/jq@jq-1.7.1 --alias jq \
  --asset-pattern '^jq-macos-amd64$' -f jq

Installation of tool jqlang/jq complete.   # exit 0
# no symlink, empty env.tools, downloaded file never linked
```

### 3. `--file-filter` is documented as a regex but is a literal filename match

`src/environment.rs:269` does:

```rust
.find(|entry| entry.file_name() == bin_name)
```

The README documents this as a regular expression and gives `'^lg$'` as an example, which cannot work. `-f '^rg$'` fails with `UnableToFindBinaryError`; `-f rg` works.

### 4. Failures return exit code `0`, and errors dump internal backtraces

`add_new_tool` logs install errors and returns `Ok(())` (`src/actions/add.rs:96`). The same pattern exists in `update` and `sync`.

A failed install of a nonexistent version printed dozens of lines of `octocrab`/`snafu` backtrace and **exited 0**. Conversely, `vers list` on an empty environment exits `1` with `Error: ActionError(EmptyEnvironment("global"))`, a `Debug` dump rather than the user-facing `Display` message.

### 5. `sync` is effectively a no-op

`sync_tools` calls `handle_tool_install(..., Some(parse_version(current_version)))`. `handle_tool_install` short-circuits when `tool.current_version == version.as_tag()` (`src/actions/mod.rs:78`). Those are equal by construction for configs written by the current code, so `sync` never checks whether files exist and never repairs a broken installation. Confirmed: exits `0` without creating the tools directory.

### 6. Relative `--data-dir` produces broken symlinks

Symlinks use the computed path as-is (`src/environment.rs:253`). With `--data-dir ./data`, the target is `./data/tools/...` stored inside `./data/envs/global`, resolving relative to the link directory. Reproduced: the link exists but cannot execute.

### 7. User input and runtime conditions panic instead of erroring

- `vers add foo` → index-out-of-bounds at `src/actions/add.rs:37`.
- Invalid `--asset-pattern` → `panic!` at `src/github.rs:111`.
- Cancelled/failed skim selector → `.unwrap()` at `src/github.rs:158–159`.
- Non-symlink at the alias path → `read_link` panic at `src/environment.rs:241`.
- Unknown OS/architecture → `panic!` in `src/system.rs`.
- Elvish shell requested → `todo!()` in `src/actions/env.rs`.
- Non-UTF-8 paths → `expect`/`unwrap` in `environment.rs` and `main.rs`.

The symlink replacement check compares `read_link(dest)` to `dest` instead of `src`, so it nearly always deletes and recreates, and panics if `dest` is a regular file.

### 8. Read-only commands mutate storage

`Environment` writes its JSON in `Drop` (`src/environment.rs:57`). `list` on a fresh data directory creates `envs/global.json` before failing, and `env --bare-path` also writes it. This causes unnecessary writes, lost updates under concurrency, and corruption risk from non-atomic writes.

### 9. Shared tool deletion breaks other environments

`tools/` is shared by all environments, but `remove --all` deletes `tools/<owner>/<repo>` and the default path deletes a specific version directory. Another environment may link to that version, so removing a tool from one environment can silently break others.

### 10. Automatic interactive asset selection is unsafe in scripts

When more than one asset matches the platform, `get_platform_specific_asset` launches `skim` (`src/github.rs:130–159`) even without `--show`, with no non-TTY fallback, and panics if selection is cancelled or skim fails. Release listing also uses `.per_page(100)` with no pagination (`src/github.rs:29`).

### 11. Windows is not actually supported

`create_symlink` contains `unimplemented!()` for Windows and unconditionally references `std::os::unix::fs::symlink`, so the crate does not compile on Windows despite the OS detection enum. The CD matrix correctly omits Windows, so this should be documented as Unix-only or implemented.

### 12. Version semantics are incomplete and string-based

`Version::Lts` and `Version::Stable` are parsed and tested but `get_specific_release_for_repo` only understands `Latest` or an exact tag; `@stable` tries to fetch a tag literally named `stable`. `@latest` installs into a directory named `latest`, then a later `update` downloads the real tag into a second directory. `Tool` stores versions as strings, inviting comparison bugs.

---

## Design and robustness concerns

- **Persistence model:** explicit `Environment::save()` with atomic temp-file-plus-rename and a lock file would be safer than `Drop`.
- **Path model:** `get_tool_download_dir` infers the tools root via `.parent().parent().unwrap()` (`src/dirs.rs:38–41`). It should receive the application root explicitly. `base_dir` should be a `PathBuf`, not a serialized `String`.
- **Download hardening:** `reqwest::get` ignores the GitHub token (private repos fail), buffers the entire asset in memory, has no timeout/retry, and performs no checksum or signature verification.
- **Extraction:** `.tgz`, `.tar.xz`, `.tar.bz2`, `.7z`, and raw binaries are unsupported. `GzipArchiver` matches `.tar.gz` rather than `.gz`, so it is shadowed by `TarGzipArchiver`. Intermediate `.tar` files are left behind. `tar` and `zip` code contains panics on malformed archives.
- **Async mismatch:** archivers are `async` but perform blocking `std::fs` I/O; `async-std` is deprecated and mixed with `reqwest`/Tokio.
- **Unvalidated filesystem inputs:** tool names, aliases, and versions are joined directly into paths. A `..` component can escape the intended directories.
- **Dead surface:** `--local` is parsed but unused; `Shells` is dead; `anyhow`, `toml`, `once_cell`, and `is_executable` are unused dependencies; `test-case` is a normal dependency instead of a dev-dependency.

---

## Security and dependencies

`Cargo.lock` is from June 2023. An OSV scan returned advisories for **30 packages** (106 advisory IDs, including GHSA/RUSTSEC duplicates), including direct dependencies:

- `zip 0.6.3` — 6 advisories, including extraction path/overwrite and RCE-class issues.
- `tar 0.4.38` — 5 advisories, including `unpack_in` chmod/path issues.
- `async-std 1.12.0` — unmaintained/deprecated.
- `tracing-subscriber 0.3.16` — 2 advisories.
- Transitive TLS/HTTP stack (`openssl`, `ring`, `rustls`, `rustls-webpki`, `h2`, `tokio`) also has multiple advisories.

The manual `enclosed_name` check in `ZipArchiver` mitigates basic zip-slip but not parser-level issues or the `tar` advisories. There is no artifact integrity verification.

---

## Testing, CI, and release

- `cargo fmt --check` passes.
- `cargo test --workspace` passes: **31 tests**.
- Coverage is limited to `version` parsing and `dirs` path helpers. No tests for install, remove, sync, update, environment serialization, archivers, asset matching, or CLI behavior.
- `cargo clippy ... -D clippy::all` fails with four lints: two `unit_arg`, one `unnecessary_mut_passed`, one `redundant_clone`.
- The CI test command `cargo test ... -- -Z unstable-options --include-ignored` fails on stable Rust (`the option Z is only accepted on the nightly compiler`). The matrix includes stable and beta, so those jobs cannot pass.
- CI/CD still uses archived `actions-rs` actions and v2-era GitHub actions.
- CD sets `continue-on-error: true` on the build job, publishes with an unpinned `cargo publish`, and references `RELEASE.md`, which does not exist.
- `Cargo.toml` declares `MIT OR Apache-2.0`, but there are no `LICENSE` files.
- `Cargo.lock` is tracked but also listed in `.gitignore`.
- `Cargo.toml` says `0.5.0`, while the newest local tag is `v0.2.3`; release/tag state is inconsistent.

---

## Documentation drift

- `vers add cli/cli --alias --show` omits the alias value.
- `--pattern`/`-p` is documented, but the real flag is `--asset-pattern`/`-a`.
- `--file-filter` is described as a regex but is literal.
- `--local` appears in help but is unimplemented.
- Windows is implied by the platform model but is not buildable.

---

## Action items

### P0 — Correctness and safety

- [x] **P0-1** Fix `remove` to resolve the symlink using the tool **alias**, not the `owner/repo` name.
- [x] **P0-2** Implement `--link-only` for `remove` (remove only the environment link, preserve downloaded assets).
- [x] **P0-3** Make `remove` update `installed_versions` and keep the `Tool` entry while other versions remain.
- [x] **P0-4** Change shared-asset deletion semantics: default to link-only removal; require an explicit purge for `tools/<owner>/<repo>`.
- [x] **P0-5** Return errors and non-zero exit codes when `add` / `update` / `sync` fail.
- [x] **P0-6** Replace internal `Debug`/backtrace output with concise `Display` error chains and actionable context.
- [x] **P0-7** Support raw binary release assets (create the link directly when no archive extractor applies).
- [x] **P0-8** Add `.tgz` (and ideally `.tar.xz` / `.tar.bz2`) support, and reject unsupported assets loudly instead of returning `Ok(())`.
- [x] **P0-9** Ensure all-extractors-failed and binary-not-found paths return errors, not silent success.
- [x] **P0-10** Rework `sync` to verify files and links exist, and reinstall/repair when missing instead of short-circuiting on version equality.
- [ ] **P0-11** Replace panics with `Result` for: invalid tool names, invalid regexes, skim cancel/failure, symlink errors, unknown OS/arch, Elvish, non-UTF-8 paths.
- [x] **P0-12** Fix the symlink replacement check to compare the existing target against the **source**, and handle a non-symlink destination without panicking.
- [ ] **P0-13** Update `tar`, `zip`, and the TLS/HTTP dependency stack to current patched versions.
- [ ] **P0-14** Add `cargo-audit` or `cargo-deny` to CI and resolve reported advisories.
- [x] **P0-15** Canonicalize the data directory and make symlinks correct for relative `--data-dir` paths.
- [x] **P0-16** Validate and sanitize tool names, aliases, and versions before joining them into filesystem paths.

### P1 — Design and UX

- [x] **P1-1** Replace `Drop`-based persistence with explicit, atomic saves (temp file + rename) guarded by a lock file.
- [x] **P1-2** Avoid writing environment files for read-only commands (`list`, `env`, `completions`).
- [ ] **P1-3** Pass the application root explicitly into path helpers instead of using `.parent().parent().unwrap()`.
- [ ] **P1-4** Change `Environment::base_dir` from `String` to `PathBuf`.
- [ ] **P1-5** Make asset selection deterministic and non-interactive by default; only prompt with an explicit flag or on a TTY.
- [ ] **P1-6** Handle cancelled/empty selections gracefully in `add --show` and automatic asset selection.
- [ ] **P1-7** Paginate GitHub release listing beyond the first 100 results.
- [ ] **P1-8** Authenticate asset downloads so private repositories work.
- [x] **P1-9** Stream downloads to disk and add timeouts/retries.
- [ ] **P1-10** Add checksum/signature verification for downloaded artifacts.
- [ ] **P1-11** Turn `--file-filter` into an actual regex match and verify the matched file is executable.
- [ ] **P1-12** Simplify async: use Tokio with `spawn_blocking`, or make extraction synchronous and keep network I/O async.
- [x] **P1-13** Clean up leftover intermediate archives after extraction.
- [ ] **P1-14** Implement or remove `--local`.
- [ ] **P1-15** Implement `Lts` / `Stable` semantics or reject them with a clear error.
- [x] **P1-16** Normalize `@latest` handling so it does not create a duplicate `latest` version directory.
- [x] **P1-17** Make `list` on an empty environment a successful empty result rather than an error (or document/justify the error).

### P2 — Project hygiene

- [ ] **P2-1** Add integration tests using `tempfile` and a local HTTP fixture for add/remove/update/sync.
- [ ] **P2-2** Add unit tests for archivers, system asset matching, environment JSON round-tripping, and symlink behavior.
- [ ] **P2-3** Add CLI integration tests (e.g. `assert_cmd`) including exit-code assertions.
- [x] **P2-4** Fix the CI test invocation for stable Rust (`--include-ignored` only where supported / use `-Z unstable-options` only on nightly).
- [ ] **P2-5** Update CI/CD actions away from archived `actions-rs` and v2-era actions.
- [ ] **P2-6** Remove `continue-on-error: true` from the release build job.
- [ ] **P2-7** Create the missing `RELEASE.md` or remove `body_path` from the release action.
- [ ] **P2-8** Add `LICENSE-MIT` and `LICENSE-APACHE` files.
- [ ] **P2-9** Remove `Cargo.lock` from `.gitignore` (it is a binary crate and the lockfile is tracked).
- [ ] **P2-10** Reconcile package version/tags (Cargo `0.5.0` vs newest tag `v0.2.3`).
- [ ] **P2-11** Remove unused dependencies: `anyhow`, `toml`, `once_cell`, `is_executable`.
- [ ] **P2-12** Move `test-case` to `[dev-dependencies]`.
- [ ] **P2-13** Remove or wire up the dead `Shells` enum.
- [ ] **P2-14** Document Unix-only support or implement Windows symlink/link handling.
- [ ] **P2-15** Correct the README examples (`--alias --show`, `--pattern`/`-p`, regex `--file-filter`, `--local`).
- [x] **P2-16** Add a `rust-version` field and consider modernizing the pinned `1.70` toolchain.

---

## Suggested implementation order

1. **Stop the bleeding:** P0-1 through P0-5.
2. **Make installs reliable:** P0-6 through P0-10, P1-11.
3. **Harden inputs and dependencies:** P0-11 through P0-16.
4. **Redesign persistence and paths:** P1-1 through P1-4, P0-15.
5. **Improve UX and network behavior:** P1-5 through P1-17.
6. **Add tests and repair CI/release:** P2-1 through P2-16.

---

## Bottom line

This is a promising, well-organized skeleton with a genuinely useful idea, but it is not ready for regular use. The happy path is convincing enough to show the concept works; the edge cases — removal, repair, raw assets, failure signalling, concurrent writes, and dependency security — are where it currently falls apart. Most P0 items are localized and fixable; a focused week or two could turn this into a reliable small utility.
