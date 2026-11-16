# Arsitektur CAFramework

**Status:** dokumen ini menjelaskan **desain target v0.1.0**. Bagian yang sudah ada dan yang belum ada ditandai di [`STATUS.md`](../STATUS.md). Jika ada yang berbeda dengan kode, kode yang benar.

---

## 1. Gambaran umum

```
app.toml
   │  (caf-template [rencana]: validasi + codegen)
   ▼
kode yang dibangkitkan (frontend/lib/generated, crates/caf-core/generated)
   │
Frontend (SvelteKit SPA, Svelte 5 runes)
   │  memanggil binding yang dibangkitkan saja
   ▼
caf-app (Tauri 2): command tipis + policy class
   │
   ▼
caf-core (logika, tanpa dependensi GUI)
   ├─ vault & keyring        — kunci, master password, recovery
   ├─ db                     — SQLCipher, migrasi, integrity check
   ├─ sync                   — helper tulis tunggal + change_log
   ├─ session, rbac, visibility
   ├─ scheduler, backup
   ├─ http, pii, integrations (registri + adapter GCC, outbox)
   ├─ pro                    — lisensi, hak akses, gating
   ├─ ai                     — mode, context provider, guardrail
   ├─ data_io                — import/export CSV, XLSX, JSON
   ├─ crash, feedback, audit, paths, error
   └─ notes                  — slice contoh (bisa dihapus generator)
```

---

## 2. Crate dalam workspace

| Crate | Isi | Catatan |
|---|---|---|
| `caf-core` | Seluruh logika aplikasi. | Tidak boleh bergantung pada GUI. Tidak boleh panik di kode non-test. |
| `caf-app` | Shell Tauri 2: registrasi command, jendela, izin (ACL). | Command hanya parsing argumen lalu memanggil `caf-core`. |
| `caf-cli` | Alat baris perintah (`caframeworkctl`). | Rahasia tidak boleh lewat argumen command line. |
| `caf-xtask` | Validasi, codegen, generator, wizard. | Target: membaca `app.toml` lewat `caf-template`. |
| `caf-migrator` | Alat migrasi skema (ada di workspace; `lib.rs` masih kosong). | Perannya terhadap `caf-core/src/migrations.rs` perlu diputuskan: satu mekanisme saja. |
| `caf-template` **(rencana, belum ada di workspace)** | Tipe konfigurasi, validasi, JSON Schema, codegen. | Target: satu-satunya tempat tipe `app.toml`. |

Crate lain hanya boleh ditambahkan jika ada tugas yang memintanya. Daftar crate yang benar-benar ada: lihat `members` di `Cargo.toml` akar.

---

## 3. Lapisan dan aturan dependensi

1. Frontend hanya memanggil **binding yang dibangkitkan**, tidak memanggil `invoke()` langsung.
2. Command di `caf-app` tipis (maksimal sekitar 15 baris) dan punya **policy class**: izin, modul, dan fitur Pro.
3. Pemeriksaan izin dan sesi dilakukan di **core**. Frontend tidak pernah mengirim identitas atau peran sebagai argumen.
4. `caf-core` tidak tahu tentang Tauri atau UI.

Arsitektur ini dijaga oleh tes arsitektur (`tests/arch.rs`).

---

## 4. Registri — cara memperluas tanpa mengubah framework

Produk menambah fitur lewat registri, bukan dengan mengedit halaman framework:

| Registri | Isi |
|---|---|
| Menu | Halaman sidebar dari `[[menus]]`. |
| Pengaturan | Bagian pengaturan dari modul (`registerSettingsSection`). |
| Izin | Katalog `<resource>:<action>` dari `[rbac]` dan menu. |
| Entity | Tabel untuk import/export dari `[[entities]]`. |
| Integrasi | Konektor API dari `[[integrations]]`. |
| Job | Tugas terjadwal dari `[[jobs]]` dan modul. |
| Context AI | Penyedia konteks untuk asisten AI. |

---

## 5. Konvensi data siap sinkronisasi

Setiap tabel data pengguna punya kolom berikut:

| Kolom | Tipe | Keterangan |
|---|---|---|
| `id` | TEXT | UUIDv7. |
| `created_at`, `updated_at` | INTEGER | Unix milidetik UTC. |
| `deleted_at` | INTEGER, nullable | Tombstone. Hapus adalah soft delete. |
| `rev` | INTEGER | Naik setiap perubahan. |
| `origin_device_id` | TEXT | ID perangkat asal. |
| `owner_profile_id` | TEXT | Profil pemilik. |
| `visibility` | TEXT | `shared`, `private_summary`, atau `private`. |

**Aturan penulisan:** setiap INSERT, UPDATE, atau DELETE lewat `sync.rs`. Helper ini memperbarui `rev` dan menulis baris `change_log` dalam **transaksi yang sama**. Pilihan helper dibanding trigger SQLite dijelaskan di ADR (lihat `docs/adr/`).

Tes konvensi memindai `sqlite_master` dan gagal jika ada tabel data tanpa kolom wajib. Tabel teknis (misalnya `schema_version`, `change_log`, `app_kv`) dikecualikan secara eksplisit dengan alasan.

---

## 6. Penyimpanan dan migrasi

- Database SQLCipher dengan kunci data acak (DEK). Kunci DEK dibungkus oleh kunci dari master password dan oleh recovery code. Lihat [`docs/02-security.md`](02-security.md).
- Migrasi bernomor, dijalankan di dalam koneksi terenkripsi. Setiap migrasi punya checksum. Migrasi yang gagal tidak mengubah versi skema.
- Rentang nomor: framework `0001–0999`, produk `1000+`, contoh Notes `0500–0599` (agar bisa dihapus generator).

---

## 7. Lapisan frontend

- SvelteKit 2 dengan adapter statis, Svelte 5 runes (`$state`, `$derived`, `$props`), Tailwind CSS 4.
- Terjemahan bertipe: `en.ts` sebagai acuan, `id.ts` harus mengikutinya.
- Kelas warna aksen ditulis literal per warna (tidak dibangun dari template string).
- Setiap halaman data punya empat keadaan: memuat, kosong, error, dan backend tidak tersedia.

---

## 8. Generator dan wizard

- `cargo run -p caf-xtask -- new-app --config app.toml --out <dir>` menyalin framework dari daftar berkas yang dilacak git (`git ls-files`), mengganti identitas lewat tabel penggantian eksplisit, menghapus contoh kecuali `--with-sample`, dan membuat kunci updater baru di luar repo.
- Generator menjalankan pemeriksaan sisa nama framework. Jika ada sisa, proses gagal.
- **(rencana)** Wizard interaktif berbagi skema dan validator yang sama dengan CLI.

Panduan lengkap: [`docs/03-duplicating-to-a-product.md`](03-duplicating-to-a-product.md).

---

## 9. Lihat juga

- [`docs/04-app-toml-reference.md`](04-app-toml-reference.md) — semua key konfigurasi.
- [`docs/02-security.md`](02-security.md) — model keamanan dan batasannya.
- [`docs/06-adding-a-menu.md`](06-adding-a-menu.md) — menambah halaman dari awal sampai akhir.
- [`docs/08-error-codes.md`](08-error-codes.md) — daftar kode error.
- [`AGENTS.md`](../AGENTS.md) — aturan untuk kontributor dan agent.
- [`DESIGN.md`](../DESIGN.md) — sistem desain antarmuka (token warna, tipografi).
