# Panduan Billing Hub (GCC)

Panduan ini menjelaskan bagaimana aplikasi yang dibuat dari CAFramework terhubung ke **GCC Billing Hub** untuk lisensi dan paket Pro.

> **Status.** Isi bagian 2 dan 3 sesuai **kode saat ini** (`[billing]` di `app.toml`, subcommand `gcc-mock`, kontrak `contracts/gcc-billing-v1.yaml`). PRD v2.0 menargetkan bentuk konfigurasi yang berbeda (bagian 4). Perilaku server GCC yang sebenarnya **belum diverifikasi**: semua yang di bawah ini diuji hanya terhadap server mock.

---

## 1. Gambaran umum

- **Satu hub.** Aplikasi tidak pernah memanggil Mayar.id atau Ko-fi secara langsung. Pembayaran, akun, batas perangkat, dan hak akses dikelola oleh GCC. Pembelian dibuka di browser lewat halaman checkout GCC; aplikasi tidak memproses pembayaran.
- **Token bertanda tangan.** Setelah akun terverifikasi, GCC menerbitkan token Ed25519 (JWS ringkas, `EdDSA`). Aplikasi memverifikasinya secara lokal dengan kunci publik, tanpa koneksi.
- **Batas offline.** Hak akses tetap berlaku offline hingga **7 hari** sejak pemeriksaan online terakhir.
- **Dua cara menyelesaikan checkout** (menurut kontrak):
  - `poll`: aplikasi membuka URL checkout lalu menanyakan status ke GCC sampai pembayaran selesai.
  - `license_key`: pembeli menerima kunci lisensi lewat email dan memasukkannya di aplikasi.
- **Tanpa konfigurasi GCC, fitur Pro tetap terkunci** dengan pesan penjelasan.

Kontrak lengkap: [`contracts/gcc-billing-v1.yaml`](contracts/gcc-billing-v1.yaml) (OpenAPI 3.1). Kontrak IPC aplikasi: [`09-backend-contract.md`](09-backend-contract.md).

---

## 2. Konfigurasi saat ini (`[billing]`)

Bagian ini dibaca oleh kode hari ini. Nilai di bawah adalah contoh.

```toml
[billing]
enabled = true
product_code = "KODEPRODUK"              # harus sama dengan kode produk yang terdaftar di GCC
gcc_base_url = "https://gcc.example.com/api/billing"
offline_days = 7
refresh_hours = 6
currencies = ["IDR", "USD"]

[[billing.public_keys]]
kid = "gcc-2026-10"                      # id kunci, harus cocok dengan token
ed25519 = "<kunci-publik-base64>"        # kunci publik, bukan rahasia
```

Catatan:
- Yang disimpan di `app.toml` hanya **kunci publik**. Kunci privat penandatangan ada di server GCC dan tidak pernah masuk repo.
- Jangan membawa kunci publik milik mock ke rilis produksi. Ganti dengan kunci publik GCC yang asli, dan jangan pasang `gcc_base_url` loopback di build rilis.

---

## 3. Menjalankan server mock untuk pengembangan

```bash
cargo run -p caf-xtask -- gcc-mock --port 8888
```

Perilaku mock (diverifikasi dari `crates/caf-xtask/src/gcc_mock.rs`):

1. **Membuat pasangan kunci baru setiap dijalankan** dan mencetak kunci publiknya (`kid` `gcc-2026-10`). Salin kunci publik yang dicetak ke `[[billing.public_keys]]`. Kunci di `app.toml` repo tidak akan cocok dengan mock yang baru dinyalakan.
2. **OTP email** tidak dikirim. OTP dicetak ke konsol dan nilainya selalu `123456`.
3. Menyediakan endpoint `/v1/auth/otp/*`, `/v1/licenses/activate`, `/v1/entitlement`, `/v1/plans`, `/v1/checkout`, `/v1/devices`, dan `/.well-known/gcc-billing-keys`.
4. Menyediakan endpoint admin untuk simulasi: `/admin/set-plan`, `/admin/issue-license`, `/admin/simulate-paid/{id}`, dan `/admin/scenario`.

Mock hanya untuk pengembangan lokal. Jangan membukanya ke jaringan.

---

## 4. Target PRD v2.0 (belum diterapkan di kode)

PRD v2.0 mengganti `[billing]` dengan bagian yang lebih kecil dan memindahkan kunci publik ke build:

| Saat ini | Target v2.0 |
|---|---|
| `[billing].gcc_base_url` | `[endpoints].gcc_base_url` |
| `[billing].product_code` | `[pro].gcc_product_id` |
| `[billing].offline_days` | `[pro].offline_grace_days` (default 7) |
| `[[billing.public_keys]]` di `app.toml` | kunci publik tertanam saat build |
| tidak ada | `[pro].trial_days`, `[pro].device_limit`, dan `[pro.features]` (`free` atau `pro` per fitur) |

Dua hal yang tetap sama: pembayaran hanya lewat GCC, dan lisensi diverifikasi offline. Sampai kode mengikuti PRD, gunakan bentuk di bagian 2. Lihat [`04-app-toml-reference.md`](04-app-toml-reference.md) untuk skema target.
