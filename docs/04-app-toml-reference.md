# Referensi `app.toml`

`app.toml` adalah satu-satunya sumber identitas sebuah aplikasi. Generator, validator, dan codegen membacanya. Setelah generate, kode produk boleh diubah bebas.

Contoh lengkap: [`docs/examples/app.example.toml`](examples/app.example.toml).

> **Peringatan: dokumen ini menjelaskan skema target (PRD v2.0), bukan yang diterima kode saat ini.** Berdasarkan pembacaan `crates/caf-xtask/src/main.rs` (belum dijalankan), perintah `check` masih memakai skema lama (`bundle_id`, `accent_color`, `[security]`, `[roles]`) dan akan menolak berkas bergaya v2.0 dengan galat `missing field`. Skema yang berlaku sekarang bisa dilihat di `app.toml` pada root repo. Perintah `caf-xtask` yang ada hari ini: `generate-icons`, `codegen`, `new-app`, `guard`, `check`, `gcc-mock`. `wizard`, opsi `--keys-dir`, dan validator penuh adalah **rencana** (paket kerja W03). Status terverifikasi ada di [`STATUS.md`](../STATUS.md).

---

## Aturan umum

- Semua key yang tidak dikenal **ditolak** (`deny_unknown_fields`).
- Validator melaporkan **semua** kesalahan sekaligus, dengan path TOML, baris, dan kolom.
- Nilai di `[brand]` wajib diisi dengan data Anda sendiri.

---

## `[app]` — identitas

| Key | Wajib | Aturan |
|---|---|---|
| `name` | ya | Nama tampilan. |
| `slug` | ya | `^[a-z][a-z0-9-]{1,30}$`. Dipakai untuk nama paket, folder data, dan prefix kunci penyimpanan. |
| `code` | ya | `^[A-Z]{2,6}$`. Prefix kode error, mis. `CNT-VAULT-001`. |
| `identifier` | ya | Reverse-DNS, mis. `com.example.app`. |
| `accent` | ya | Salah satu: `violet sky emerald amber rose cyan indigo teal orange pink`. |
| `locales` | ya | Himpunan bagian dari `{id, en}`. |
| `default_locale` | ya | Harus termasuk dalam `locales`. |
| `platforms` | ya | Himpunan bagian dari `{linux, windows, macos, android}`. |
| `caframework_version` | tidak | Diisi generator. Menyimpan versi framework asal. |

## `[brand]` — identitas developer (wajib)

| Key | Aturan |
|---|---|
| `developer` | Tidak boleh kosong. |
| `company` | Opsional. |
| `website`, `support_url` | Harus `https://`. `support_url` kosong = pakai `website`. |
| `email` | Format email valid. |
| `phone` | Format E.164, mis. `+6285220696117`. Ditampilkan sesuai locale. |
| `copyright` | Mendukung `{year}`. |
| `logo` | Path ke berkas logo; harus ada. |
| `show_on_lock_screen` | Default `true`. |

Tanpa `[brand]`, validasi gagal dengan pesan "brand is mandatory".

## `[window]` — opsional

| Key | Default |
|---|---|
| `width` / `height` | 1200 / 800 |
| `min_width` / `min_height` | 900 / 600 |
| `resizable` | `true` |

## `[modules]` — saklar fitur

| Key | Default | Catatan |
|---|---|---|
| `profiles` | `true` | Jika `false`, satu profil super otomatis dipakai. |
| `ai` | `true` | |
| `pro` | `true` | Selalu dibangkitkan. `false` hanya menyembunyikan UI Pro. |
| `feedback`, `updater`, `crash`, `integrations`, `scheduler` | `true` | Fitur jaringan tetap nonaktif jika endpoint kosong. |
| `support_page` | `false` | |

**Tidak boleh ditulis:** `backup` (selalu aktif). Validator menolaknya.

## `[rbac]` dan `[[rbac.roles]]` — peran dan izin

| Key | Aturan |
|---|---|
| `super_role` | Harus ada di daftar peran dan hanya memegang `["*"]`. Tidak bisa dihapus atau diganti nama. |
| `allow_runtime_roles` | Jika `true`, peran super bisa membuat dan mengubah peran saat aplikasi berjalan. |
| `[[rbac.roles]]` `key` | `^[a-z][a-z0-9_]{0,31}$`, unik. |
| `label_en`, `label_id` | Nama peran dalam dua bahasa. |
| `permissions` | Daftar izin `<resource>:<action>`, atau `*`. |

Aksi yang dikenali: `view`, `create`, `update`, `delete`, `export`, `import`, `manage`, `run`. Wildcard: `<resource>:*` dan `*`.

Resource: menu, entity, integrasi, dan job dari konfigurasi; plus area sistem `users`, `roles`, `settings`, `security`, `backup`, `data`, `integrations`, `scheduler`, `ai`, `pro`, `audit`.

## `[profiles]`

| Key | Aturan |
|---|---|
| `pin_required_for` | Himpunan peran yang wajib PIN. Harus bagian dari peran yang ada. |
| `age_stages` | Opsional. Daftar `{ key, min_age, max_age }` yang tidak saling tumpang tindih. |

## `[endpoints]` — kosong = nonaktif

| Key | Aturan |
|---|---|
| `gcc_base_url` | Kosong atau `https://…`. `http://localhost` hanya dengan peringatan. |
| `updater_manifest_url` | Sama seperti di atas. |

## `[pro]` dan `[pro.features]`

| Key | Default | Aturan |
|---|---|---|
| `gcc_product_id` | kosong | Kode produk di server lisensi. Kosong = fitur Pro terkunci dengan penjelasan. |
| `trial_days` | 14 | |
| `device_limit` | 3 | |
| `offline_grace_days` | 7 | Rentang 1–30. |
| `refresh_hours` | 6 | Rentang 1–24. |
| `currencies` | `["IDR","USD"]` | Himpunan bagian dari `{IDR, USD}`, tidak boleh kosong. |
| `[pro.features]` | — | Nilai harus `free` atau `pro`. |

## `[[integrations]]` — API eksternal

| Key | Keterangan |
|---|---|
| `key` | Unik, URL-safe. |
| `kind` | `gcc` atau `rest`. |
| `base_url` / `base_url_from` | URL tetap, atau referensi ke `[endpoints]`. |
| `user_configurable` | Jika `true`, URL diisi pengguna di Pengaturan. |
| `auth` | `none`, `api_key_header`, `api_key_query`, `bearer`, `basic`, `hmac_sha256`, `oauth2_client_credentials`. |
| `timeout_ms`, `retries`, `rate_limit_per_minute`, `concurrency`, `allowed_hosts` | Batas dan kebijakan. Semua punya default aman. |

Rahasia (kunci API, password) **tidak pernah** ditulis di `app.toml`. Rahasia disimpan di vault.

## `[[jobs]]` — jadwal latar belakang

| Key | Aturan |
|---|---|
| `key` | Unik. |
| `schedule` | Cron 5 kolom (waktu lokal), atau preset `hourly`, `daily`, `weekly`, `monthly`, atau `every <n> minutes` (minimal 5). |
| `enabled` | `true` / `false`. |

## `[[entities]]` — tabel untuk import/export

| Key | Keterangan |
|---|---|
| `key` | Unik. Harus punya tabel sinkronisasi yang sesuai. |
| `label_en`, `label_id` | |
| `natural_key` | Opsional. Kunci bisnis untuk upsert. |
| `formats` | Himpunan bagian dari `{csv, xlsx, json}`. |

Batas default: 50 000 baris atau 20 MB per berkas.

## `[[menus]]` — halaman sidebar

| Key | Aturan |
|---|---|
| `key` | `^[a-z][a-z0-9-]{0,31}$`, unik. Tidak boleh `settings`, `lock`, `_dev`, atau `api`. |
| `label_en`, `label_id` | Wajib. |
| `icon` | Path SVG 24×24 (garis), maksimal 2000 karakter. |
| `accent` | Opsional. Kosong = aksen aplikasi. |
| `group` | Kunci grup sidebar. |
| `permission` | Harus merujuk ke resource yang dikenal, default `<key>:view`. |

Minimal satu menu harus ada.

---

## Contoh kesalahan validasi (ilustrasi)

```text
error: unknown field `bundle_id`
  --> app.toml:12:1
error: brand is mandatory
error: [rbac] super role "super_admin" must hold exactly ["*"]
```

Pesan final bisa berbeda; yang dijamin adalah path dan lokasi disertakan.
