# Membawa Perbaikan Framework ke Produk Turunan

Panduan untuk produk yang sudah dibuat dengan `new-app` dan ingin mengambil perbaikan, pembaruan keamanan, atau migrasi skema dari `caframework`.

> Produk turunan adalah salinan, bukan dependensi. Tidak ada pembaruan otomatis. Anda membawa perbaikan secara sadar, satu per satu.

## 1. Mengetahui versi yang dipakai

Generator mencatat versi framework di `[app].caframework_version` pada `app.toml` produk (target PRD v2.0; pada kode saat ini pastikan nilainya diisi manual). Versi ini adalah titik awal untuk melihat apa yang berubah.

## 2. Remote upstream

```bash
git remote add upstream https://github.com/cecepazhar/caframework.git
git fetch upstream
git log --oneline <versi-yang-dipakai>..upstream/main
```

Karena produk dibuat lewat generator (bukan fork), riwayat git keduanya tidak berhubungan. Gunakan `CHANGELOG.md` framework sebagai panduan, lalu terapkan perubahan dengan `git diff` per berkas atau `git cherry-pick` jika berkasnya tidak berubah identitasnya.

## 3. Catatan porting di CHANGELOG

Setiap rilis framework yang mengubah perilaku produk harus punya **catatan porting** di `CHANGELOG.md`: berkas apa yang disentuh, apakah ada migrasi, dan langkah manualnya. Baca catatan ini sebelum menyalin apa pun.

## 4. Yang perlu diperiksa

- **Migrasi:** lihat `crates/caf-core/src/migrations/` untuk definisi versi baru. Rentang nomor: framework `0001–0999`, produk `1000+`. Jangan menomori ulang migrasi produk.
- **Kode error:** lihat [`08-error-codes.md`](08-error-codes.md) untuk kategori baru.
- **Komponen antarmuka:** bandingkan `frontend/src/lib/components/` untuk pembaruan tata letak dan komponen keamanan (TitleBar, PINPad, Lockscreen).
- **Kebijakan command:** bandingkan `crates/caf-core/src/api/policy.rs` untuk izin yang berubah.

## 5. Setelah porting

1. Jalankan semua perintah verifikasi produk (`cargo test`, `clippy`, `fmt`, `npm run check`, `npm run build`).
2. Perbarui `caframework_version` di `app.toml` produk.
3. Catat apa yang dibawa dan apa yang sengaja dilewati.

Lihat juga [`03-duplicating-to-a-product.md`](03-duplicating-to-a-product.md).
