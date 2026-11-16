# AGENTS.md — Instructions for AI agents and contributors

> Read this file first. It is written for AI coding agents and for any developer who starts a new product from CAFramework or changes this repository.

## 1. What this repository is

CAFramework is an **open-source (MIT) starter template** for desktop and mobile apps. It is built with Rust, Tauri 2, SQLCipher (Argon2id), and a Svelte 5 frontend (Tailwind CSS 4).

The template is **product-agnostic**. A new product is generated from one file, `app.toml`, with `cargo run -p caf-xtask -- new-app`. The framework must never contain product-specific business terms (for example "family", "cash", or "patient") outside examples and the sample "Notes" menu.

- Owner and author: Cecep Saeful Azhar Hidayat, ST.
- Full requirements: the PRD (product requirements document) kept in the owner's private notes. Public summary: `docs/01-architecture.md`.
- Current verified status: **`STATUS.md`**. Trust it over any older claim in the code, in commit messages, or in other docs.

## 2. Read before you change anything

| Read this | When |
|---|---|
| `STATUS.md` | Before claiming anything is "done", "working", or "production ready". |
| `docs/03-duplicating-to-a-product.md` | When the task creates or updates a derivative product. |
| `docs/04-app-toml-reference.md` | When the task touches `app.toml`, codegen, or the generator. |
| `docs/01-architecture.md` | When the task adds a crate, a module, a command, or a registry entry. |
| `docs/02-security.md` | When the task touches the vault, keys, sessions, permissions, backup, or AI data. |
| `CONTRIBUTING.md`, `SECURITY.md` | Before opening a change or reporting a vulnerability. |

**The code and command output are the source of truth.** If a document and the code disagree, the code wins. Fix the document in the same change.

## 3. Hard rules

1. **Stay product-agnostic.** Do not hard-code a brand, a product name, a role name, or an endpoint in framework code. Brand data comes from generated brand files. Endpoints come from `[endpoints]` in `app.toml`.
2. **Use registries to extend.** Menus, settings sections, permissions, entities, integrations, jobs, and AI context providers are added through their registries. Do not edit framework pages to add a product feature.
3. **No secrets, keys, or personal data in the repository, its history, logs, or evidence.** This includes keystores, updater private keys, `.env` files, real API keys, personal photos, and personal documents.
4. **No internal material in the public repository.** Plans, audits, internal ledgers, and task notes belong in the owner's private notes, not in this repo.
5. **Every commit must compile.** A change that breaks `cargo build` on the branch is not acceptable.
6. **Tests must be able to fail.** Do not write placeholder tests (`expect(true)`), tests that assert on a file they wrote themselves, or "negative" tests that only check success. A guard counts only after a committed fixture shows it failing.
7. **Do not weaken checks.** Do not delete, skip, or `#[ignore]` a test, lint, guard, or CI step to make a change pass. Explain any exception in the pull request.
8. **Claims need evidence.** Do not write "done", "verified", "secure", or "green" without the command and its real output. If you could not run something, say so: "NOT VERIFIED (reason)".
9. **Branch and push policy (A-06).** Work directly on `main`. Pushing to `main` is permitted only after every green checkpoint (`scripts/ci-local.sh` exits 0). Creating, moving, or deleting tags, force-pushing, amending pushed commits, and rewriting git history are strictly forbidden (owner-only actions). Release decisions belong to the owner.
10. **Stage with explicit paths.** Use `git add <paths>`, not `git add .` or `git add -A`. Review `git status` before every commit.
11. **Keep to the stack.** Rust edition 2024, toolchain `1.98.1` (see `rust-toolchain.toml`), Tauri 2, Svelte 5 runes, Tailwind 4. Do not add a crate or npm package without a clear reason, and record its licence.

## 4. Language

- Instructions, code, comments, and commit messages: **English**.
- Human-facing docs in `docs/` and reports to the owner: **Bahasa Indonesia** (the owner's language). Keep technical identifiers in English.

## 5. Verification commands

Run the commands that exist in your checkout. `STATUS.md` says which ones are reliable.

```bash
# Rust (pinned toolchain comes from rust-toolchain.toml)
cargo build --workspace --exclude caf-app --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --exclude caf-app -- -D warnings
cargo test --workspace --exclude caf-app
cargo deny check

# Frontend
cd frontend && npm ci && npm run check && npm run build && npm run test

# Generated code and guards
cargo run -p caf-xtask -- codegen --check
cargo run -p caf-xtask -- guard

# App crate (needs Tauri system libraries, e.g. webkit2gtk-4.1)
cargo check -p caf-app
```

When you claim a change works, paste the relevant output in the pull request or in the evidence file.

## 6. Where things go

| You want to add… | Put it here | Guide |
|---|---|---|
| A menu / page | `app.toml` `[[menus]]`, then `frontend/src/routes/` | `docs/06-adding-a-menu.md` |
| A data table / entity | migration, core module through `sync.rs`, `[[entities]]` | `docs/14-adding-an-entity.md` (design draft; the feature is not built yet) |
| A Tauri command | `crates/caf-app/src/commands.rs` (thin), logic in `caf-core`, permission in the policy registry | `docs/01-architecture.md` |
| An external API | `[[integrations]]` in `app.toml`, adapter in `caf-core` | `docs/15-adding-an-integration.md` (design draft; the feature is not built yet) |
| A background job | the scheduler job registry | `docs/16-adding-a-job.md` (design draft; the feature is not built yet) |
| A new product | `app.toml` + `cargo run -p caf-xtask -- new-app` | `docs/03-duplicating-to-a-product.md` |

Guides 14 to 17 are design drafts for features that do not exist in the code yet. Do not describe those features as working. Mark a guide "(planned)" in your own change only if it truly does not exist yet.

## 7. Working on a derivative product

If the task is to build a **product** from this template, do not change the framework repository to make the product work. Put product code in the generated product repository. If you find a framework bug, fix it in the framework, then port the fix with the port notes described in `docs/03-duplicating-to-a-product.md`.

## 8. Reporting

- Say what changed, which files, and which commands you ran with their results.
- List what you could not verify and why.
- Name any new dependency, its licence, and why it is needed.
