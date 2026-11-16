# Dokumentasi CAFramework

Indeks semua dokumen, diurutkan sesuai alur baca: konsep, cara pakai, referensi, lalu operasional.

## Urutan baca

| No. | Dokumen | Isi |
|---|---|---|
| 01 | [`01-architecture.md`](01-architecture.md) | Arsitektur, crate, registri, konvensi data |
| 02 | [`02-security.md`](02-security.md) | Model keamanan dan batasannya |
| 03 | [`03-duplicating-to-a-product.md`](03-duplicating-to-a-product.md) | Panduan membuat produk turunan (mulai di sini untuk produk baru) |
| 04 | [`04-app-toml-reference.md`](04-app-toml-reference.md) | Referensi semua key `app.toml` |
| 05 | [`05-creating-an-app.md`](05-creating-an-app.md) | Langkah teknis dari nol sampai aplikasi berjalan |
| 06 | [`06-adding-a-menu.md`](06-adding-a-menu.md) | Menambah halaman dari database sampai UI |
| 07 | [`07-billing-guide.md`](07-billing-guide.md) | Panduan penagihan (GCC) |
| 08 | [`08-error-codes.md`](08-error-codes.md) | Taksonomi kode error |
| 09 | [`09-backend-contract.md`](09-backend-contract.md) | Kontrak server (GCC) |
| 10 | [`10-android-build-guide.md`](10-android-build-guide.md) | Build Android |
| 11 | [`11-android-signing.md`](11-android-signing.md) | Penandatanganan Android |
| 12 | [`12-android-mobile-layout.md`](12-android-mobile-layout.md) | Tata letak mobile |
| 13 | [`13-porting-framework-fixes.md`](13-porting-framework-fixes.md) | Membawa perbaikan framework ke produk yang sudah ada |
| 14 | [`14-adding-an-entity.md`](14-adding-an-entity.md) | Entitas dan import/export data **(draf desain, fitur belum ada)** |
| 15 | [`15-adding-an-integration.md`](15-adding-an-integration.md) | Integrasi API **(draf desain, fitur belum ada)** |
| 16 | [`16-adding-a-job.md`](16-adding-a-job.md) | Job terjadwal **(draf desain, fitur belum ada)** |
| 17 | [`17-wizard.md`](17-wizard.md) | Wizard dan App Builder **(draf desain, fitur belum ada)** |

Folder pendukung (tidak diberi nomor):
- [`examples/`](examples/) — contoh siap salin, mis. `app.example.toml`
- [`contracts/`](contracts/) — spesifikasi kontrak yang bisa dibaca mesin (`gcc-billing-v1.yaml`)
- [`adr/`](adr/) — catatan keputusan arsitektur, bernomor sendiri (`0001-…`)

Di luar `docs/`: [`../README.md`](../README.md) (ringkasan), [`../STATUS.md`](../STATUS.md) (status terverifikasi), [`../AGENTS.md`](../AGENTS.md) (untuk AI agent, baca pertama).

## Berdasarkan tujuan

- **Mengenal CAFramework:** 01, 02, lalu `../STATUS.md`.
- **Membuat produk baru:** 03, 05, 04, lalu 13 untuk produk yang sudah berjalan.
- **Mengembangkan fitur:** 06, 08, 09. Draf desain fitur yang direncanakan: 14 sampai 17.
- **Android:** 10, 11, 12.
- **AI agent:** `../AGENTS.md` dulu, lalu dokumen yang ditunjuk tabel di dalamnya.

## Aturan penamaan

- Format: `NN-nama-dokumen.md`, dua digit, huruf kecil, kata dipisah tanda hubung.
- Nomor mengikuti urutan baca di tabel di atas. Saat menambah dokumen, sisipkan di posisi yang tepat, geser nomor yang terdampak, dan perbarui tabel ini dalam commit yang sama.
- Tautan antar-dokumen memakai nama bernomor. Setelah rename atau penambahan, jalankan pemeriksa tautan:

```bash
# dari root repo: cetak tautan .md yang menunjuk ke berkas yang tidak ada
for f in README.md STATUS.md AGENTS.md docs/*.md; do
  d=$(dirname "$f")
  grep -oE '\]\([^)#]+\.md' "$f" | sed 's/](//' | while read -r l; do
    [ -e "$d/$l" ] || echo "RUSAK: $f -> $l"
  done
done
```

## Dokumen yang direncanakan

Dokumen 14 sampai 17 baru berupa draf desain dari PRD; fiturnya belum ada di kode, dan isinya akan dilengkapi setelah fitur terbukti. Belum ditulis: versi pengguna akhir dari panduan penagihan.

Catatan penomoran: dokumen 14 sampai 17 diberi nomor kosong berikutnya agar tidak memicu rename massal. Urutan baca yang berlaku adalah urutan di tabel, bukan semata-mata urutan angka.
