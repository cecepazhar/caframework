# Menambah entitas (import/export data)

> **Status: rencana.** Registri entitas dan import/export generik (PRD FR-17, paket kerja W-import) **belum ada di kode**. Dokumen ini mencatat desain target supaya produk bisa merencanakan datanya. Nama fungsi dan tipe Rust sengaja belum ditulis; bagian itu akan dilengkapi setelah fiturnya selesai dan terbukti. Lihat [`STATUS.md`](../STATUS.md).

Entitas adalah satu jenis data pengguna (misalnya catatan, produk, transaksi) yang didaftarkan agar otomatis mendapat **import dan export** dalam CSV, XLSX, dan JSON. Import/export data per entitas bersifat **wajib** di framework dan tidak bisa dimatikan.

Baca dulu [`06-adding-a-menu.md`](06-adding-a-menu.md) (irisan vertikal satu halaman) dan konvensi data di [`01-architecture.md`](01-architecture.md).

---

## 1. Prasyarat: tabel yang siap sinkronisasi

Tabel entitas harus memenuhi konvensi data (PRD FR-5): `id` (UUIDv7), `created_at`, `updated_at`, `deleted_at`, `rev`, `origin_device_id`, serta `owner_profile_id` dan `visibility` bila profil aktif. Semua penulisan lewat helper sinkronisasi agar `change_log` ikut tercatat dalam transaksi yang sama. Tes konvensi akan menggagalkan build jika ada tabel yang melanggar.

## 2. Langkah (target)

1. **Migrasi.** Buat tabel di rentang nomor produk (`1000+`). Rentang `0001–0999` milik framework.
2. **Daftarkan di `app.toml`.**
   ```toml
   [[entities]]
   key = "notes"                  # unik, aman untuk URL
   label_en = "Notes"
   label_id = "Catatan"
   natural_key = []               # opsional: kunci bisnis unik untuk upsert
   formats = ["csv", "xlsx", "json"]
   ```
3. **Deskripsi bidang di Rust.** Entitas dilengkapi deskripsi bidangnya (nama, tipe, wajib atau tidak) agar importer bisa memetakan kolom dan memvalidasi baris. *(Bentuk API ditentukan saat fitur dibangun.)*
4. **Izin.** Entitas menambah sumber daya izin dengan aksi `view`, `create`, `update`, `delete`, `export`, `import`. Berikan izin itu ke peran yang berhak di `[[rbac.roles]]`.
5. **Format.** Pilih format per entitas lewat `formats`. Ekspor XLSX bisa dijadikan fitur Pro lewat `[pro.features]` (`data_export_xlsx = "pro"`).

## 3. Perilaku import (target)

1. Pilih berkas.
2. Pemetaan kolom otomatis, bisa diubah.
3. Pratinjau dengan validasi per baris.
4. Pilih mode: hanya tambah (insert), atau upsert berdasarkan `id` atau `natural_key`.
5. Ringkasan dry-run.
6. Eksekusi dalam **satu transaksi**: semuanya atau tidak sama sekali, atau lewati baris yang tidak valid bila pengguna memilih.
7. Laporan, dengan berkas baris gagal beserta alasannya yang bisa diunduh.

Baris yang diimpor melewati helper sinkronisasi dan aturan visibilitas, dan pengimpor menjadi pemiliknya.

## 4. Perilaku export (target)

- Mengikuti izin dan visibilitas pengguna.
- Ekspor spreadsheet menetralkan injeksi rumus.
- Templat impor per entitas bisa diunduh.
- Batas terdokumentasi: bawaan 50.000 baris atau 20 MB per berkas.

## 5. Bukti yang harus ada saat fitur selesai

- Uji bolak-balik: ekspor, hapus, impor, data identik.
- Uji bahwa baris `private` milik profil lain tidak ikut terekspor oleh peran non-super.
- Uji injeksi rumus (`=`, `+`, `-`, `@` di awal sel) dan batas ukuran.
- Uji parser berkas (properti atau fuzz) untuk CSV, XLSX, dan JSON.
- Benchmark: impor 50.000 baris ≤ 30 detik, ekspor ≤ 15 detik (PRD bagian 6).
