# CAFramework — Status (v0.1.0-dev)

**Terakhir diperbarui:** 16 November 2026 (menyesuaikan dengan validasi kode 4 Oktober 2026).

> **Baca ini dengan benar.** Dokumen ini sebelumnya menyatakan banyak fitur "Production Ready". Validasi langsung terhadap kode pada 4 Oktober 2026 tidak mendukung klaim itu. Tabel di bawah hanya memuat apa yang **terbukti**, apa yang **sebagian**, dan apa yang **belum ada**. Jalankan ulang perintah verifikasi sebelum mempercayai status apa pun, karena kode terus berubah.

> **Sejak validasi 4 Oktober, kode berubah.** Beberapa berkas ditulis setelah tanggal itu (mis. `keyring.rs`, `migrations.rs`, `billing/`, `caf-migrator`), jadi beberapa baris di bawah ditandai **Kode ada, belum diverifikasi**. Artinya berkasnya ada di repo, tetapi belum ada build, tes, atau bukti perintah yang saya lihat. Tabel ini belum dijalankan ulang terhadap kode terbaru.

Legenda: **Terbukti** = ada bukti perintah dan output. **Sebagian** = sebagian kriteria terbukti. **Belum** = belum ada atau tidak terbukti. **Belum diverifikasi** = perlu dijalankan di mesin pemilik.

## 1. Fondasi

| Komponen | Status | Catatan |
|---|---|---|
| Workspace Rust 2024 | Sebagian | `clippy -D warnings` hijau untuk `caf-core` dan `caf-cli` (4 Okt). Seluruh workspace belum bisa dikompilasi pada validasi itu karena kesalahan di `caf-xtask`. |
| SQLCipher + Argon2id | Terbukti (statis) | Parameter m=64 MiB, t=3, p=4 ada di kode. |
| Kunci data (DEK) dan recovery code | Kode ada, belum diverifikasi | Validasi 4 Okt: belum ada. Setelahnya muncul `keyring.rs`, [ADR 0001](docs/adr/0001-key-hierarchy.md), dan catatan di `CHANGELOG.md`. Perlu diuji (round-trip, ganti password, recovery). |
| Fallback `vault.key` plaintext | Belum diverifikasi ulang | Tercatat di audit 2 Oktober; harus dihapus. Periksa dengan pencarian di `crates/caf-core`. |
| Migrasi database | Kode ada, belum diverifikasi | Ada `caf-core/src/migrations.rs`, `migrations/0001_baseline_schema.sql`, dan crate `caf-migrator`. Dua mekanisme yang tumpang tindih; pilih satu. |
| Skema siap sinkronisasi | Sebagian | Tabel `notes` dan `profiles` punya kolom sinkron. `change_log` baru ditulis oleh `notes`. |
| Multi-profil dan PIN | Sebagian | Hash PIN Argon2id ada. Alur PIN di UI dan batas percobaan di core belum ada. |
| RBAC | Belum | Identitas dan izin masih ditentukan frontend. Harus dipindah ke core. |
| Visibilitas data | Belum | Filter masih bisa dilewati. |

## 2. Frontend

| Komponen | Status | Catatan |
|---|---|---|
| Svelte 5 runes, Tailwind 4 | Terbukti | `npm run check` 0 error, `npm run build` sukses (4 Okt). |
| Vitest | Sebagian | Runner berjalan; satu tes adalah placeholder dan akan dihapus. |
| Tema, aksen, shell, title bar | Belum | Tema terang dan aksen dinamis belum bekerja di shell. |
| i18n | Sebagian | Banyak kunci dipakai di kode tetapi belum ada di kamus. |
| Binding IPC | Belum | Puluhan command dipanggil frontend tanpa terdaftar di Rust. |

## 3. Fitur layanan

| Komponen | Status | Catatan |
|---|---|---|
| Backup dan restore | Belum diverifikasi ulang | Validasi 4 Okt: implementasi lama tidak memulihkan data. Ada `backup.rs` dan [ADR 0002](docs/adr/0002-backup-format.md); ADR itu belum selaras dengan PRD FR-9 (kunci cadangan dibungkus keyring). |
| Asisten AI | Sebagian | Klien OpenAI-compatible ada. Mode, privasi bertingkat, dan penyamaran lengkap belum. |
| Pro dan lisensi | Kode ada, belum diverifikasi | Ada `caf-core/src/billing/` (client, gate, state, store, token) dan server mock GCC (`gcc-mock`). Terhadap server GCC yang asli: belum diverifikasi. Pembayaran ditangani GCC; aplikasi hanya menerima token bertanda tangan. |
| Integrasi API, scheduler | Belum | Direncanakan. |
| Import/export per entitas | Belum | Direncanakan. |
| Laporan crash, umpan balik | Sebagian | Panic hook dan penyamaran sebagian ada. Pengiriman belum bekerja. |

## 4. Alat dan generator

| Komponen | Status | Catatan |
|---|---|---|
| Subcommand `caf-xtask` | Sebagian | Yang ada: `generate-icons`, `codegen`, `new-app`, `guard`, `check`, `gcc-mock`. **Belum ada:** `validate` (namanya `check`), `wizard`, opsi `--keys-dir`. Perlu dipastikan ulang setelah build pulih. |
| `new-app` | Sebagian | Ada, tetapi belum memenuhi kontrak (skema tidak sesuai, `--with-sample` diabaikan, kunci updater tidak dibuat baru). |
| Validator `app.toml` (`check`) | Skema lama | Dari pembacaan kode: masih mewajibkan `bundle_id`, `accent_color`, `[security]`, `[roles]`, `menus.href`. Belum menerima kontrak v2.0 di [`docs/04-app-toml-reference.md`](docs/04-app-toml-reference.md). Belum dijalankan. |
| Wizard dan App Builder | Belum | Direncanakan. |
| Android | Belum diverifikasi | Panduan build ada. Perlu diuji di perangkat. |

## 5. Dokumentasi

| Dokumen | Status |
|---|---|
| `README.md`, `AGENTS.md`, `docs/` (indeks, arsitektur, keamanan, duplikasi, referensi `app.toml`) | Diperbarui 16 Nov 2026 |
| `docs/14-adding-an-entity.md`, `15-adding-an-integration.md`, `16-adding-a-job.md`, `17-wizard.md` | Draf desain dari PRD (fiturnya belum ada di kode) |
| `docs/07-billing-guide.md`, `09-backend-contract.md`, `10`–`13` (Android, porting) | Ditulis ulang 16 Nov 2026; diselaraskan dengan kode (belum dijalankan) |
| Dokumentasi pengguna akhir | Belum |

## 6. Prioritas berikutnya

1. Pulihkan build yang hijau dan stabil di seluruh workspace dengan toolchain `1.98.1`.
2. Tutup celah keamanan inti: hapus `vault.key` plaintext, pindahkan sesi, izin, dan visibilitas ke core.
3. Selesaikan kontrak `app.toml`, validator, dan generator, lalu tes di tiga aplikasi contoh.
4. Lengkapi fitur layanan (backup, integrasi, scheduler, Pro, AI, import/export).
5. Verifikasi independen sebelum rilis `v0.1.0`.

## 7. Cara memverifikasi sendiri

```bash
cargo build --workspace --exclude caf-app --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --exclude caf-app -- -D warnings
cargo test --workspace --exclude caf-app
cargo deny check
(cd frontend && npm ci && npm run check && npm run build && npm run test)
```

Tempelkan output asli saat melaporkan hasil. Klaim tanpa bukti dianggap belum terverifikasi.
