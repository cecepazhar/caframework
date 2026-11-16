# Menambah menu atau fitur baru (vertical slice)

Panduan ini menunjukkan urutan dari database sampai halaman. Contoh acuannya adalah menu **Notes**. Ikuti pola yang sama untuk fitur Anda.

**Status:** urutan di bawah adalah pola target. Beberapa bagian (misalnya registri pengaturan dan binding yang dibangkitkan) masih dikerjakan; lihat [`STATUS.md`](../STATUS.md).

---

## Ringkasan 14 langkah

1. Migrasi (kolom sinkronisasi wajib dan indeks)
2. Tipe dan validasi di core
3. Penyimpanan lewat `sync.rs` dan helper visibilitas
4. Tes core, termasuk kasus negatif dan izin
5. Kode error bernomor
6. Command tipis di `caf-app` dengan policy class
7. Binding yang dibangkitkan dan pembungkus API
8. Store (hanya jika state dipakai bersama)
9. Halaman (`PageHeader`, `PageContainer`, empat keadaan)
10. Menu, palet perintah, dan bagian pengaturan (lewat registri)
11. Terjemahan `en` dan `id`
12. Pendaftaran entity, integrasi, atau job jika perlu
13. Bukti: data tetap ada setelah aplikasi dimulai ulang
14. Dokumentasi

---

## Langkah 1 — Daftarkan menu di `app.toml`

```toml
[[menus]]
key = "tasks"
label_en = "Tasks"
label_id = "Tugas"
icon = "M5 13l4 4L19 7"       # path SVG 24x24 (garis)
group = "daily"
permission = "tasks:view"
```

Jalankan codegen:

```bash
cargo run -p caf-xtask -- codegen
```

Hasilnya (navigasi, label, dan izin) ditulis ke berkas yang diberi header `@generated`. **Jangan mengedit berkas itu langsung.**

---

## Langkah 2 — Migrasi dan modul core

1. Tambahkan migrasi bernomor (rentang produk `1000+`) yang membuat tabel dengan **semua kolom wajib**: `id`, `created_at`, `updated_at`, `deleted_at`, `rev`, `origin_device_id`, `owner_profile_id`, `visibility`. Buat indeks untuk `owner_profile_id`, `visibility`, `updated_at`, dan `deleted_at`.
2. Buat `crates/caf-core/src/tasks.rs` dengan tipe dan validasi. Gunakan error khusus (bukan error generik).
3. Semua penulisan lewat `sync.rs`. Jangan menulis SQL INSERT/UPDATE/DELETE langsung ke tabel data.
4. Semua pembacaan lewat helper visibilitas, dengan paginasi (cursor, maksimal 500 per halaman).
5. Ekspor modul di `crates/caf-core/src/lib.rs`.

Tes konvensi akan gagal jika kolom wajib atau indeks kurang.

---

## Langkah 3 — Command di `caf-app`

Command **tidak** menerima identitas atau peran dari frontend. Sesi dan izin diambil dari core.

```rust
// crates/caf-app/src/commands.rs — tipis: parsing + panggilan ke core
#[tauri::command]
pub async fn list_tasks(state: tauri::State<'_, AppCore>, page: PageRequest)
    -> Result<Page<TaskRecord>, AppError>
{
    state.guarded(policy::TASKS_VIEW, |ctx| caf_core::tasks::list(ctx, page)).await
}
```

Lalu:
- Daftarkan di `generate_handler!`, di `build.rs`, dan di berkas ACL.
- Tambahkan entri policy (izin, modul, fitur Pro jika ada).
- Jalankan codegen agar binding TypeScript ikut diperbarui.

---

## Langkah 4 — Halaman

1. Buat `frontend/src/routes/tasks/+page.svelte` dengan Svelte 5 runes.
2. Panggil API lewat pembungkus di `frontend/src/lib/api/`, bukan `invoke()` langsung.
3. Bungkus dengan `PageContainer` dan `PageHeader`.
4. Tangani empat keadaan: memuat, kosong, error (`errorText(err)`), dan backend tidak tersedia.
5. Tampilkan tombol aksi hanya jika `can('tasks:create')` dan sejenisnya. Core tetap menolak jika izin tidak ada.

---

## Langkah 5 — Terjemahan

Tambahkan kunci di `en.ts` (acuan) dan `id.ts`. Tidak boleh ada teks yang ditulis langsung di markup.

---

## Langkah 6 — Bukti bahwa semuanya terhubung

Sebuah fitur hanya dianggap selesai jika rantai berikut terbukti:

core → tes core → command terdaftar → binding → pembungkus API → pemanggil di UI → data tetap ada setelah aplikasi dimulai ulang.

Halaman yang hanya tampil tanpa backend yang bekerja adalah **UI-ONLY** dan harus dilaporkan sebagai itu.

---

## Hal yang sering salah

- Membuat tabel tanpa `rev` atau `deleted_at`.
- Menulis SQL langsung ke tabel data, melewati `sync.rs`.
- Membangun kelas Tailwind dari template string (kelas hilang di build produksi).
- Mengirim `caller_profile_id` atau peran dari frontend.
- Menulis `String(err)` alih-alih `errorText(err)`.
- Mengedit berkas `@generated`.
