# Penandatanganan Aplikasi Android (Signing)

Panduan membuat dan memakai keystore untuk menandatangani build rilis Android.

> **Aturan utama.** Keystore **tidak boleh** ada di repo, dan **setiap produk memakai keystore sendiri**. Detail keystore nyata (lokasi, sidik jari sertifikat, kata sandi) disimpan di catatan **privat** pemilik, bukan di dokumen publik ini. Jika keystore hilang, aplikasi yang sudah terpasang tidak bisa diperbarui (tanda tangan tidak cocok).

---

## 1. Membuat keystore

Jalankan di komputer Anda, di luar folder repo. Ganti nilai dalam `< >` dengan milik produk Anda.

```bash
mkdir -p ~/.android
keytool -genkeypair -v \
  -keystore ~/.android/<slug>-release.keystore \
  -alias <slug>-release \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -dname "CN=<nama pengembang>, O=<organisasi>, C=<kode negara>"
chmod 600 ~/.android/<slug>-release.keystore
```

`keytool` akan meminta kata sandi. Pilih yang kuat dan simpan di pengelola kata sandi.

Setelah dibuat, catat sidik jari SHA-256 sertifikat di catatan privat Anda:

```bash
keytool -list -v -keystore ~/.android/<slug>-release.keystore -alias <slug>-release
```

---

## 2. Penyimpanan dan pencadangan

1. **Lokal:** `chmod 600`, di luar repo.
2. **Cadangan:** simpan salinan keystore dan kata sandinya di pengelola kata sandi atau vault terenkripsi, terpisah satu sama lain bila memungkinkan.
3. **CI:** simpan keystore sebagai secret berformat base64 dan kata sandinya sebagai secret terpisah. Jangan pernah mencetak keduanya di log.
   ```bash
   base64 -w 0 ~/.android/<slug>-release.keystore
   ```
   Contoh nama secret: `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`.

---

## 3. Menandatangani

### Opsi A: manual dengan `apksigner`

Berikan kata sandi lewat variabel lingkungan, bukan sebagai argumen biasa, supaya tidak terlihat di daftar proses:

```bash
export KS_PASS='<kata sandi keystore>'   # jangan simpan di berkas
apksigner sign \
  --ks ~/.android/<slug>-release.keystore \
  --ks-key-alias <slug>-release \
  --ks-pass env:KS_PASS \
  --key-pass env:KS_PASS \
  --out app-signed.apk \
  app-unsigned.apk
unset KS_PASS
```

Verifikasi:

```bash
apksigner verify --verbose --print-certs app-signed.apk
```

### Opsi B: GitHub Actions

```yaml
- name: Decode keystore
  run: echo "${{ secrets.ANDROID_KEYSTORE_BASE64 }}" | base64 -d > "$RUNNER_TEMP/release.keystore"

- name: Sign APK
  env:
    KS_PASS: ${{ secrets.ANDROID_KEYSTORE_PASSWORD }}
  run: |
    "$ANDROID_HOME/build-tools/<versi>/apksigner" sign \
      --ks "$RUNNER_TEMP/release.keystore" \
      --ks-key-alias "${{ secrets.ANDROID_KEY_ALIAS }}" \
      --ks-pass env:KS_PASS --key-pass env:KS_PASS \
      --out dist/app-signed.apk path/to/unsigned.apk
```

Ganti `<versi>` dengan versi build-tools yang terpasang di runner Anda.

---

## 4. Daftar periksa

- [ ] Keystore ada di luar repo dan berizin `600`.
- [ ] Ada cadangan keystore dan kata sandi di tempat terpisah dari komputer kerja.
- [ ] Sidik jari SHA-256 tercatat di catatan privat.
- [ ] Keystore ini tidak dipakai produk lain.
- [ ] Tidak ada APK, AAB, atau `.idsig` terlacak di git.
