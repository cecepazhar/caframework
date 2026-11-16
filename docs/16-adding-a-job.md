# Menambah job terjadwal

> **Status: rencana.** Scheduler (PRD FR-16) **belum ada di kode**. Dokumen ini mencatat desain target. Contoh kode Rust akan ditambahkan setelah fiturnya terbukti. Lihat [`STATUS.md`](../STATUS.md).

Job adalah tugas yang berjalan sesuai jadwal di dalam aplikasi, misalnya cadangan berkala atau pengosongan outbox. Modul framework dan produk mendaftarkan job-nya sendiri.

---

## 1. Mendeklarasikan jadwal di `app.toml` (target)

```toml
[[jobs]]
key = "backup.scheduled"
schedule = "0 2 * * 1"             # cron 5 bidang (waktu lokal) atau preset: hourly | daily | weekly | monthly
enabled = true
```

Format jadwal yang diterima: cron 5 bidang dalam waktu lokal, preset (`hourly`, `daily`, `weekly`, `monthly`), atau `every <n> minutes` dengan n minimal 5. Setiap jadwal punya deskripsi yang terbaca manusia dalam Indonesia dan Inggris (misalnya "Setiap Senin pukul 02.00"). Ekspresi cron yang tidak valid ditolak saat validasi.

## 2. Atribut job saat didaftarkan (target)

| Atribut | Arti |
|---|---|
| `key` | Pengenal unik dan aman untuk URL. |
| judul | Nama yang tampil di Pengaturan → Jadwal. |
| jadwal bawaan | Dipakai jika `app.toml` tidak menimpanya. |
| izin yang dibutuhkan | Izin RBAC untuk melihat atau menjalankan manual. |
| butuh brankas terbuka | Jika ya, job menunggu selama brankas terkunci. |
| kebijakan catch-up | `Skip` atau `RunOnce` untuk jadwal yang terlewat. |
| waktu maksimum | Batas lama satu kali jalan. |

## 3. Perilaku saat berjalan (target)

- Tiap job bisa diaktifkan, dinonaktifkan, diubah, dan dijalankan sekarang dari Pengaturan → Jadwal.
- Riwayat eksekusi dengan status dan pesan (bawaan menyimpan 500 eksekusi terakhir).
- Satu instans per job pada satu waktu.
- Job yang gagal atau panik **tidak boleh** menjatuhkan aplikasi.
- Job yang butuh brankas menunggu sampai brankas terbuka.

## 4. Job bawaan framework

`backup.scheduled`, `backup.overdue_reminder`, `integrations.outbox_flush`, `pro.licence_refresh`, `audit.prune`.

## 5. Per platform

- **Desktop:** mode tray (tutup ke tray) dan mulai saat login bersifat opsional dan **mati secara bawaan**. Eksekusi yang terlewat dikejar saat aplikasi dibuka berikutnya sesuai kebijakan job.
- **Android:** job berjalan saat aplikasi dipakai, ditambah catch-up. Keterbatasannya didokumentasikan di [`10-android-build-guide.md`](10-android-build-guide.md).
- Job tidak berjalan saat aplikasi tertutup. Ini keterbatasan yang harus disampaikan ke pengguna.

## 6. Bukti yang harus ada saat fitur selesai

- Uji ekspresi cron dan preset, termasuk zona waktu lokal dan perubahan jam musim.
- Uji job yang panik: aplikasi tetap hidup dan riwayat mencatat kegagalan.
- Uji catch-up `Skip` dan `RunOnce`.
- Uji ketahanan (soak) 30 menit tanpa galat dan dengan pertumbuhan memori di bawah 10% (PRD bagian 6).
