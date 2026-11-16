# CAFramework

Starter template aplikasi desktop (Linux, macOS, Windows) dan mobile (Android) yang aman, modular, dan siap sinkronisasi. Dibuat dengan **Rust (edisi 2024)**, **Tauri 2**, **SQLCipher (Argon2id)**, **Svelte 5**, dan **Tailwind CSS 4**.

CAFramework bersifat **generik**. Aplikasi baru (keuangan, kantor, inventori, catatan, dan lainnya) dibuat dari satu berkas `app.toml`, jadi framework ini dirancang untuk diduplikasi ke banyak produk.

> **Status: pra-rilis (`v0.1.0-dev`).** Jangan menganggap sebuah fitur sudah siap produksi sebelum [`STATUS.md`](STATUS.md) menyatakannya terverifikasi.

---

## Yang ditawarkan (target v0.1.0)

- Penyimpanan lokal terenkripsi SQLCipher dengan kunci dari Argon2id, dan recovery code 24 kata.
- Multi-profil dengan PIN, peran (RBAC), kepemilikan, dan visibilitas data.
- Skema siap sinkronisasi (UUIDv7, `rev`, tombstone, `change_log`). Mesin sinkronisasi belum ada.
- Backup terenkripsi (wajib) dan import/export data per entitas (CSV, XLSX, JSON).
- Integrasi API dengan adapter server lisensi, penjadwal tugas, dan paket Pro dengan pembatasan fitur.
- Asisten AI dengan tingkat privasi dan penyamaran data pribadi.
- Antarmuka Svelte 5 dengan tema gelap dan terang, aksen yang bisa diganti, dan terjemahan Indonesia/Inggris.
- Generator `cargo xtask new-app` dan wizard (dalam pengerjaan).

Detail dan status per fitur: [`STATUS.md`](STATUS.md) dan [`docs/01-architecture.md`](docs/01-architecture.md).

---

## Mulai cepat

### Prasyarat
- Rust toolchain `1.98.1` (dipasang otomatis oleh rustup dari `rust-toolchain.toml`)
- Node.js LTS dan npm
- Linux: paket dev WebKitGTK 4.1 dan GTK 3 (lihat [`docs/03-duplicating-to-a-product.md`](docs/03-duplicating-to-a-product.md#2-prasyarat))

### Jalankan untuk pengembangan

```bash
git clone https://github.com/cecepazhar/caframework.git
cd caframework

# Gunakan data sementara agar tidak menyentuh data asli
export CAF_DATA_DIR="$(mktemp -d)"

cargo run -p caf-xtask -- codegen
cargo tauri dev
```

### Buat produk turunan

```bash
cargo run -p caf-xtask -- check --config my-product.toml
cargo run -p caf-xtask -- new-app --config my-product.toml --out ~/Project/my-product --dry-run
cargo run -p caf-xtask -- new-app --config my-product.toml --out ~/Project/my-product
```

> **Catatan.** `check` saat ini masih membaca skema `app.toml` lama (lihat `app.toml` di root). Skema v2.0 di dokumentasi adalah target dan belum diterima kode. Lihat [`STATUS.md`](STATUS.md).

Panduan lengkap: [`docs/03-duplicating-to-a-product.md`](docs/03-duplicating-to-a-product.md).

---

## Dokumentasi

Indeks lengkap: [`docs/README.md`](docs/README.md).

| Dokumen | Isi |
|---|---|
| [`AGENTS.md`](AGENTS.md) | Aturan untuk AI agent dan kontributor |
| [`docs/03-duplicating-to-a-product.md`](docs/03-duplicating-to-a-product.md) | Membuat produk baru dari framework |
| [`docs/04-app-toml-reference.md`](docs/04-app-toml-reference.md) | Referensi `app.toml` |
| [`docs/01-architecture.md`](docs/01-architecture.md) | Arsitektur |
| [`docs/02-security.md`](docs/02-security.md) | Model keamanan |
| [`docs/06-adding-a-menu.md`](docs/06-adding-a-menu.md) | Menambah halaman |
| [`docs/10-android-build-guide.md`](docs/10-android-build-guide.md) | Build Android |

---

## Kontribusi dan keamanan

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md) — laporkan kerentanan secara privat

## Lisensi

MIT. Lihat [`LICENSE`](LICENSE). Atribusi proyek asal ada di [`NOTICE`](NOTICE).
