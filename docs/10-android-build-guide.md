# Panduan Build Android

Panduan konfigurasi dan build aplikasi CAFramework (dan produk turunannya) untuk Android lewat Tauri 2.

> **Status.** Build Android sudah pernah dicoba (ada proyek `gen/android` dan sebuah APK uji), tetapi **belum diverifikasi ulang** terhadap PRD v2.0 dan belum diuji di perangkat untuk semua fitur. Tabel platform di akhir mencatat keterbatasan yang sudah diketahui.

---

## 1. Gambaran

- **Mesin:** Tauri 2 (backend Rust, antarmuka Svelte 5 di WebView).
- **Identifier aplikasi:** diambil dari `app.toml` dan `tauri.conf.json`. Setiap produk punya identifier sendiri, jangan memakai identifier framework.
- **Database:** SQLCipher dikompilasi silang untuk ABI Android (`arm64-v8a`, `armeabi-v7a`, `x86_64`).
- **Kunci:** kata sandi master di-hash dengan Argon2id. Brankas terkunci saat aplikasi masuk latar belakang (target FR-13).

---

## 2. Prasyarat

1. Target Rust untuk Android:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   ```
2. Android SDK dan NDK:
   - `ANDROID_HOME` atau `ANDROID_SDK_ROOT` menunjuk ke folder Android SDK.
   - NDK versi 26 atau lebih baru.
   - JDK 17 atau 21 (`JAVA_HOME`).
3. Tauri CLI. Pasang versi yang cocok dengan versi `tauri` di `Cargo.lock`, bukan `@latest`:
   ```bash
   cargo install tauri-cli --version "^2" --locked
   ```

---

## 3. Inisialisasi proyek Android

Jalankan sekali dari folder crate aplikasi (`crates/caf-app`, atau `crates/<slug>-app` di produk turunan):

```bash
cargo tauri android init
```

Perintah ini membuat `gen/android/` berisi proyek Android Studio dan skrip Gradle. Folder itu hasil bangkitan, jangan menyunting isinya secara manual untuk logika produk.

---

## 4. Signing

Build rilis harus ditandatangani. **Keystore dibuat per produk, disimpan di luar repo, dan tidak dibagi antarproduk.** Prosedur dan pencadangannya ada di [`11-android-signing.md`](11-android-signing.md).

Setelah keystore dibuat, berikan lokasinya lewat variabel lingkungan (nilai di bawah hanya contoh):

```bash
export ANDROID_KEYSTORE_PATH="$HOME/.android/<slug>-release.keystore"
export ANDROID_KEY_ALIAS="<slug>-release"
# kata sandi: berikan lewat variabel lingkungan atau secret CI, jangan ditulis di berkas
```

---

## 5. Menjalankan dan membangun

Mode pengembangan di emulator atau perangkat:

```bash
cargo tauri android dev
```

Build rilis:

```bash
cargo tauri android build --apk     # APK
cargo tauri android build --aab     # App Bundle untuk Google Play
```

Hasil build ada di dalam `gen/android/app/build/outputs/`. **Jangan commit APK, AAB, atau berkas `.idsig` ke repo.** Guard repo menolak berkas biner terlacak (FR-14). Bagikan artefak lewat rilis, bukan lewat git.

---

## 6. Keterbatasan platform

Hal berikut berbeda di Android dan harus punya cadangan perilaku yang terdokumentasi (FR-13):

| Area | Keterbatasan |
|---|---|
| Updater | Hanya desktop. Android memakai distribusi toko atau instalasi manual. |
| Scheduler | Job berjalan saat aplikasi dipakai, ditambah catch-up saat dibuka. Tidak berjalan penuh di latar belakang. |
| Folder backup | Pemilih folder Android berbeda dari desktop. |
| Biometrik | Tidak ada di PRD v2.0. Jangan menjanjikannya sebelum diimplementasikan. |
