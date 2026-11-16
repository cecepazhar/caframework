# Membuat aplikasi baru dari CAFramework

Panduan ini melengkapi [`03-duplicating-to-a-product.md`](03-duplicating-to-a-product.md). Di sini fokusnya pada urutan langkah teknis dari nol sampai aplikasi berjalan.

> **Peringatan: dokumen ini menjelaskan skema target (PRD v2.0), bukan yang diterima kode saat ini.** Berdasarkan pembacaan `crates/caf-xtask/src/main.rs` (belum dijalankan), perintah `check` masih memakai skema lama (`bundle_id`, `accent_color`, `[security]`, `[roles]`) dan akan menolak berkas bergaya v2.0 dengan galat `missing field`. Skema yang berlaku sekarang bisa dilihat di `app.toml` pada root repo. Perintah `caf-xtask` yang ada hari ini: `generate-icons`, `codegen`, `new-app`, `guard`, `check`, `gcc-mock`. `wizard`, opsi `--keys-dir`, dan validator penuh adalah **rencana** (paket kerja W03). Status terverifikasi ada di [`STATUS.md`](../STATUS.md).

---

## 1. Siapkan `app.toml`

Salin contoh dan sesuaikan:

```bash
cp docs/examples/app.example.toml ~/my-product.toml
```

Ganti minimal:
- `[app]`: `name`, `slug`, `code`, `identifier`
- `[brand]`: seluruh isi, dengan data Anda sendiri
- `[[menus]]`: halaman yang Anda butuhkan

Referensi lengkap ada di [`04-app-toml-reference.md`](04-app-toml-reference.md).

---

## 2. Validasi

```bash
cargo run -p caf-xtask -- check --config ~/my-product.toml
```

Perbaiki semua kesalahan yang dilaporkan. Validator menunjukkan path dan baris.

---

## 3. Generate

Pratinjau dulu tanpa menulis apa pun:

```bash
cargo run -p caf-xtask -- new-app --config ~/my-product.toml --out ~/Project/my-product --dry-run
```

Lalu generate sungguhan. Folder `--out` harus baru atau kosong, dan tidak boleh berada di dalam repo framework:

```bash
cargo run -p caf-xtask -- new-app --config ~/my-product.toml --out ~/Project/my-product
```

Tambahkan `--with-sample` jika ingin menyertakan menu contoh "Notes".

---

## 4. Verifikasi hasil

```bash
cd ~/Project/my-product
git init && git add -A && git commit -m "chore: initial generation" # hanya di repo produk Anda
cargo test --workspace --exclude my-product-app
cd frontend && npm ci && npm run check && npm run build && npm run test
cd ..
cargo check -p my-product-app
```

Ganti `my-product` dengan `slug` produk Anda. Jika ada perintah yang gagal, perbaiki sebelum melanjutkan.

---

## 5. Jalankan aplikasi dengan data sementara

Jangan pernah memakai data pengguna asli saat pengujian:

```bash
export CAF_DATA_DIR="$(mktemp -d)"
cargo tauri dev
```

`CAF_DATA_DIR` memisahkan database uji dari data nyata. Nama variabel bisa berbeda per produk (`<CODE>_DATA_DIR`); lihat `[app].code`.

---

## 6. Ganti logo dan ikon

```bash
# assets/logo.svg harus sudah diganti dengan logo Anda
cargo tauri icon assets/logo.svg
```

---

## 7. Kunci updater

**(rencana)** Generator membuat pasangan kunci minisign baru. Kunci privat disimpan di luar repo (default `~/.config/<slug>/updater/`) dengan kata sandi. **Jangan** commit kunci privat atau memakai kunci framework. Pada kode saat ini generator belum membuat kunci ini (validasi 4 Oktober 2026), jadi buat sendiri dengan `minisign -G` atau `cargo tauri signer generate`.

---

## 8. Sebelum rilis

Lihat checklist di [`03-duplicating-to-a-product.md`](03-duplicating-to-a-product.md#8-checklist-produk-baru).

---

## Masalah umum

| Gejala | Penyebab biasa | Solusi |
|---|---|---|
| `error: brand is mandatory` | `[brand]` belum ditulis. | Lengkapi `[brand]`. |
| Galat `missing field` saat `check` (mis. untuk `bundle_id`) | Berkas memakai skema v2.0, sedangkan `check` saat ini masih membaca skema lama. | Gunakan skema lama (`app.toml` di root) sampai W03 selesai, atau tunggu validator baru. |
| Generator menolak `--out` | Folder sudah berisi berkas atau berada di dalam repo. | Pilih folder kosong di luar repo. |
| Residue check gagal | Nama framework masih tersisa. | Ganti sesuai pesan, atau perbaiki tabel penggantian. |
| `cargo check -p <slug>-app` gagal di Linux | Dependensi WebKitGTK belum terpasang. | Pasang paket yang disebut di `03-duplicating-to-a-product.md`. |
