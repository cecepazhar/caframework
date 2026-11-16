# Membuat produk baru dari CAFramework (duplikasi)

> Dokumen ini untuk **siapa pun** — developer, owner, atau AI agent — yang ingin membuat aplikasi baru dari CAFramework. Baca juga [`AGENTS.md`](../AGENTS.md) di root repo.

> **Peringatan: dokumen ini menjelaskan skema target (PRD v2.0), bukan yang diterima kode saat ini.** Berdasarkan pembacaan `crates/caf-xtask/src/main.rs` (belum dijalankan), perintah `check` masih memakai skema lama (`bundle_id`, `accent_color`, `[security]`, `[roles]`) dan akan menolak berkas bergaya v2.0 dengan galat `missing field`. Skema yang berlaku sekarang bisa dilihat di `app.toml` pada root repo. Perintah `caf-xtask` yang ada hari ini: `generate-icons`, `codegen`, `new-app`, `guard`, `check`, `gcc-mock`. `wizard`, opsi `--keys-dir`, dan validator penuh adalah **rencana** (paket kerja W03). Status terverifikasi ada di [`STATUS.md`](../STATUS.md).

---

## 1. Prinsip

- CAFramework adalah **template generik**. Produk dibuat dari satu berkas `app.toml`.
- Setelah generator berjalan, kode produk **boleh diubah bebas**. Framework tidak perlu ikut diubah.
- Produk tidak boleh mewarisi **identitas** framework: brand, kunci updater, endpoint, prefix error, dan identifier aplikasi.
- Perbaikan framework dibawa ke produk yang sudah jadi dengan **catatan port** (lihat bagian 7).

---

## 2. Prasyarat

| Kebutuhan | Keterangan |
|---|---|
| Rust | Toolchain `1.98.1` dari `rust-toolchain.toml` (akan dipasang otomatis oleh rustup). |
| Node.js | LTS, dengan npm. |
| Dependensi sistem Linux | Paket dev WebKitGTK 4.1, GTK 3, dan librsvg untuk membangun aplikasi Tauri. Contoh Ubuntu/Debian: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev patchelf`. |
| Ruang disk | Minimal 6 GB bebas untuk build pertama. |

---

## 3. Langkah ringkas

```bash
# 1. Ambil framework (repo ini) dan pastikan tree bersih
git clone https://github.com/cecepazhar/caframework.git
cd caframework
git status                       # harus kosong

# 2. Buat app.toml produk Anda (lihat bagian 4 dan docs/04-app-toml-reference.md)
cp docs/examples/app.example.toml ~/my-product.toml   # (contoh; sesuaikan)

# 3. Validasi konfigurasi
cargo run -p caf-xtask -- check --config ~/my-product.toml

# 4. Lihat rencana generate tanpa menulis apa pun
cargo run -p caf-xtask -- new-app --config ~/my-product.toml --out ~/Project/my-product --dry-run

# 5. Generate di folder BARU di luar repo framework
cargo run -p caf-xtask -- new-app --config ~/my-product.toml --out ~/Project/my-product

# 6. Verifikasi produk hasil generate
cd ~/Project/my-product
cargo test --workspace --exclude <slug>-app
cd frontend && npm ci && npm run check && npm run build && npm run test
cargo check -p <slug>-app
```

Catatan:
- `--out` **harus** folder yang belum ada atau kosong, dan **tidak boleh** berada di dalam repo framework.
- Pakai `--with-sample` jika ingin menyertakan menu contoh "Notes". Tanpa flag ini, contoh dihapus.
- **(rencana)** Generator membuat pasangan kunci minisign baru di luar repo, dan hanya kunci publik yang masuk ke produk. Validasi 4 Oktober 2026 menemukan generator saat ini belum melakukannya, jadi buat dan pasang kunci updater produk secara manual dan jangan memakai kunci framework.

**(rencana)** Wizard interaktif: `cargo run -p caf-xtask -- wizard --out my-product.toml`. Wizard akan mengisi `[brand]` dari `~/.config/caframework/brand.toml` dan memvalidasi setiap langkah.

---

## 4. Isi `app.toml` produk — urutan kerja

Ikuti urutan ini. Setiap langkah punya padanan di [`docs/04-app-toml-reference.md`](04-app-toml-reference.md).

1. **`[app]`** — `name`, `slug` (huruf kecil, tanda hubung), `code` (2–6 huruf kapital, dipakai sebagai prefix error seperti `KAS-VAULT-001`), `identifier` (reverse-DNS, unik untuk produk Anda), `accent`, `locales`, `default_locale`, `platforms`.
2. **`[brand]`** — **wajib**. Isi nama developer, website, email, telepon, dan teks hak cipta milik Anda sendiri. Jangan biarkan nilai framework.
3. **`[window]`** — opsional. Ukuran jendela awal.
4. **`[modules]`** — nyalakan atau matikan fitur. Backup dan import/export **selalu aktif** dan tidak bisa dimatikan.
5. **`[rbac]` dan `[[rbac.roles]]`** — peran dan izin. Format izin `<resource>:<action>`. Peran super selalu memegang `*`.
6. **`[profiles]`** — peran mana yang wajib PIN.
7. **`[endpoints]`** — kosongkan jika belum punya server. Fitur jaringan otomatis nonaktif.
8. **`[pro]` dan `[pro.features]`** — fitur gratis atau Pro. Isi `gcc_product_id` setelah produk didaftarkan di server lisensi.
9. **`[[integrations]]`** — API eksternal (opsional).
10. **`[[jobs]]`** — jadwal latar belakang (opsional).
11. **`[[entities]]`** — tabel data yang bisa di-import/export.
12. **`[[menus]]`** — halaman dan ikon sidebar. Minimal satu menu.

Setelah selesai, jalankan `check`. Validator melaporkan **semua** kesalahan sekaligus, lengkap dengan path dan baris.

---

## 5. Yang harus Anda ganti dan yang TIDAK boleh diwarisi

**Harus diganti sebelum rilis:**

| Item | Di mana |
|---|---|
| Nama, slug, identifier, prefix error | `[app]` |
| Brand (developer, website, email, telepon, hak cipta) | `[brand]` |
| Logo | `assets/logo.svg`, lalu jalankan `cargo tauri icon` |
| Kunci publik updater | dihasilkan generator; jangan pakai milik framework |
| Endpoint server (GCC, updater) | `[endpoints]` |
| Nama menu, label, dan ikon | `[[menus]]` |
| Konten halaman placeholder | `frontend/src/routes/m/<key>/` (ditandai `UI-ONLY placeholder`) |

**Tidak boleh diwarisi:**
- Kunci rahasia, token, atau keystore apa pun.
- Endpoint atau akun server milik framework.
- Data pengguna atau database contoh.
- Brand framework (developer, email, website) di UI atau installer produk.

Generator memeriksa sisa nama framework (*residue check*). Jika ditemukan, proses gagal dan Anda harus membersihkannya.

---

## 6. Keputusan yang sudah ditetapkan (ikuti, jangan diubah di produk)

- **Dua level kunci:** vault dengan master password, dan profil dengan PIN.
- **Pemulihan:** recovery code 24 kata (rencana penuh di `v0.1.0`; lihat `docs/02-security.md`).
- **Sinkronisasi siap:** setiap tabel data punya `id` (UUIDv7), `rev`, `created_at`, `updated_at`, `deleted_at`, `origin_device_id`, `owner_profile_id`, dan `visibility`. Jangan membuat tabel data tanpa kolom ini.
- **Tulis lewat helper sinkronisasi** (`sync.rs`), bukan SQL langsung. Helper ini mencatat `change_log` dalam transaksi yang sama.
- **Visibilitas:** `shared`, `private_summary`, `private`. Filter dilakukan di core, bukan di frontend.
- **Pro dan lisensi:** pembayaran diproses di server (GCC). Aplikasi tidak pernah memproses pembayaran sendiri.

---

## 7. Membawa perbaikan framework ke produk yang sudah jadi

1. Lihat `CHANGELOG.md` framework. Setiap perubahan yang memengaruhi kode hasil generate punya **catatan port**.
2. Ikuti catatan port per kelompok fitur (keamanan, RBAC, integrasi, scheduler, import/export, Pro, generator).
3. Setiap produk menyimpan `caframework_version` di `app.toml` untuk mengetahui versi asal.
4. Uji ulang produk setelah port: build, test, dan jalankan aplikasi dengan `CAF_DATA_DIR` sementara.

Lihat juga `docs/13-porting-framework-fixes.md`.

---

## 8. Checklist produk baru

- [ ] `app.toml` lolos `check`.
- [ ] `[brand]` berisi data Anda sendiri.
- [ ] Tidak ada nilai framework yang tersisa (lihat residue check).
- [ ] Kunci updater dibuat baru dan disimpan di luar repo.
- [ ] Endpoint diisi hanya jika server sudah siap.
- [ ] Setiap tabel data punya kolom sync wajib.
- [ ] Halaman placeholder sudah diganti atau ditandai di dokumentasi produk.
- [ ] `docs/` produk menjelaskan produk, bukan framework.
- [ ] Build, test, dan `npm run check` lulus di repo produk.
- [ ] Data pengujian memakai `CAF_DATA_DIR` sementara, bukan data asli.
- [ ] Tidak ada rahasia di repo produk atau riwayat git-nya.

---

## 9. Untuk AI agent

- Jangan mengubah repo framework saat tugas hanya membuat produk. Kerjakan di folder produk hasil generate.
- Sebelum melaporkan produk "siap", jalankan perintah verifikasi dan tempelkan outputnya.
- Jika tidak yakin apakah fitur sudah ada, cek `STATUS.md` dan kode, lalu tulis "NOT VERIFIED" dengan alasan.
- Jangan menyalin brand, endpoint, atau kunci dari framework ke produk tanpa diminta pemilik.
