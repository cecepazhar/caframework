# CAFramework PRD Audit — Vault prd.md (FR-1..19) + DESIGN.md/ARCHITECTURE.md vs Implementasi

> Audit: 2026-10-11 · Base: `/home/cecepazhar/Product/caframework` · Metode: statis

## Hasil: 15/19 FR (79%) · Design Compliance: PASS (CADS v1.0 dogfood)

### Gap

| FR | Gap | Detail | Severity |
|---|---|---|---|
| FR-16 | Scheduler TIDAK ADA | PRD v2.0 detail: cron 5-field, presets, catch-up policy, retention 500 runs, tray mode, 5 built-in jobs (`backup.scheduled`, `backup.overdue_reminder`, `integrations.outbox_flush`, `pro.licence_refresh`, `audit.prune`). Absen semua. Mandatory §4, blokir DoD §9. | P0 |
| FR-17 | Data import/export TIDAK ADA | PRD detail: CSV/XLSX/JSON per entity, column mapping, preview, dry-run, all-or-nothing, template download, 50k rows/20MB limit, formula injection neutralise. Absen. Mandatory §4. | P0 |
| FR-12 | `caf-template` lib ≠ spec | `caf-xtask` `run_new_app` meng-copy framework dirinya sendiri via `git ls-files` + string-replace (CAFramework→nama, caframework→slug, com.fathforce.caframework→bundle_id, CAF→error_prefix, + var CARGO_MANIFEST_DIR). Bukan crate `caf-template` terpisah. Berfungsi (dogfooding: CAMark/CACash/CAStudio/CACalm semua di-generate begini). | P1 |

### Bukti Implementasi (key)
- Core (30+ file): vault.rs + keyring/, profiles.rs, rbac.rs, visibility.rs, sync.rs, ai/ + ai_context.rs, pro.rs, billing/ (GCC/Mayar gate: client, gate, state, store, token), http.rs, pii.rs, feedback.rs, crash.rs, backup.rs, dual_split.rs (Global Split + In-Pane Split), db.rs + migrations/.
- Generator: `caf-xtask` Commands: GenerateIcons, Check, Codegen (ts_bindings), NewApp (config/out/with_sample/dry_run), Guard (guard.rs), GccMock.
- Frontend: routes settings/* (ai-routing, ai-personas, ai-habits, ai-skills) + Notes slice. Notes = vertical slice sample.
- Android: caf-app/gen/android + APK rilis (build universalRelease ada).
- CADS: DESIGN.md (CADS v1.0) di repo + CAUI (caui/) shared UI 40+ komponen; VARIANT_SPEC.md mengikat variant/size/a11y.

### Rekomendasi
1. P0: FR-16 scheduler + FR-17 data I/O — keduanya mandatory; blocker DoD v0.1.0 dan blokir CAMark/CACash (inherit via generator).
2. P1: FR-12 — keputusan arsitektur: refactor `caf-template` crate vs amend PRD (self-copy template). Catat risiko: prefix exclude (`dist/`, `build/bin/`, `gen/`) perlu regression test.
3. P2: Audit `run_new_app` exclude-list lengkap — cek `frontend/build`, `.svelte-kit`, evidence tidak ikut ter-copy.