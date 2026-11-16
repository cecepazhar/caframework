# Keamanan CAFramework

Dokumen ini menjelaskan model keamanan **target v0.1.0** dan batasannya. Setiap klaim ditandai statusnya di [`STATUS.md`](../STATUS.md). Klaim yang belum terverifikasi tidak boleh dianggap sudah ada.

---

## 1. Dua level kunci

| Level | Melindungi | Cara buka | Status |
|---|---|---|---|
| **Vault** | Seluruh database di disk | Master password, atau recovery code 24 kata | Rencana penuh di v0.1.0 |
| **Profil** | Pemisahan pengguna di satu perangkat | PIN profil (4–8 digit) | Rencana penuh di v0.1.0 |

- Vault terkunci berarti tidak ada data yang bisa dibaca, dan core mengembalikan error `VaultLocked`. Tidak boleh ada file kunci plaintext di disk.
- Profil terkunci hanya menutup sesi profil itu. Vault tetap terbuka.

---

## 2. Hierarki kunci

```
Master password ──Argon2id──► KEK_pw ─┐
                                      ├─ membungkus ─► DEK (kunci database, 32 byte acak)
24 kata recovery ──HKDF──► KEK_rc ────┘                 └─► dipakai SQLCipher
```

- **DEK** dibuat acak sekali saat setup. Tidak pernah disimpan dalam bentuk plaintext.
- **Mengganti password** hanya membungkus ulang DEK. Database **tidak** di-rekey.
- **Regenerasi recovery code** mencabut kode lama.
- **Argon2id** untuk master password: memori 64 MiB, 3 iterasi, paralelisme 4.
- **Backup** memakai kunci backup (BK) terpisah. Backup terjadwal tidak butuh passphrase.

Detail desain: ADR `docs/adr/0001-key-hierarchy.md`.

---

## 3. Pemulihan (recovery)

- Saat setup, aplikasi menampilkan **24 kata** sekali saja. Pengguna harus mengonfirmasi tiga kata acak sebelum melanjutkan.
- Kata dan entropinya **tidak pernah** ditulis ke disk, log, atau crash report.
- Tanpa master password **dan** tanpa recovery code, data tidak bisa dipulihkan. Tidak ada pintu belakang.

---

## 4. Batas percobaan (back-off)

Berlaku untuk master password, recovery code, dan PIN (D-23 di PRD):

- 5 percobaan pertama tanpa jeda.
- Setelah itu, jeda 30 detik, digandakan sampai maksimal 15 menit.
- Keberhasilan mengatur ulang hitungan.
- 10 kegagalan PIN berturut-turut mengunci vault.

**Batasannya:** back-off hanya memperlambat tebakan lewat aplikasi. Tebakan offline dibatasi oleh biaya Argon2id, bukan oleh hitungan ini.

---

## 5. Profil di satu perangkat — batasan penting

- Pemisahan profil **diterapkan oleh aplikasi**, bukan dengan kunci terpisah per orang. Siapa pun yang tahu master password dapat membuka seluruh database.
- Peran super dapat membaca baris `private` milik profil lain. Setiap pembacaan seperti itu dicatat di log audit (`PRIVATE_READ`) dan ditandai di UI.
- Peran lain hanya melihat baris `shared`, dan agregat untuk `private_summary`.
- Dokumentasi produk harus menyatakan batasan ini kepada pengguna.

---

## 6. Izin dan visibilitas

- Setiap perintah diperiksa di **core**: keadaan vault, sesi, izin, modul, lalu fitur Pro.
- Frontend tidak pernah mengirim identitas atau peran.
- Baris yang tidak boleh dilihat dikembalikan sebagai "tidak ditemukan", tanpa menunjukkan bahwa baris itu ada.

---

## 7. Memori dan rahasia

- Kunci disimpan dalam buffer yang di-zeroize. **Batasan:** salinan di buffer IPC atau serde di luar kendali kami mungkin masih tersisa di memori sampai ditimpa.
- Kata sandi dan PIN tidak boleh masuk ke log, pesan error, crash dump, atau riwayat perintah.
- API key untuk asisten AI disimpan di vault dan tidak pernah dikirim ke frontend. Pengaturan hanya mengembalikan `has_api_key`.
- Kunci rahasia integrasi juga disimpan di vault.

---

## 8. Berkas dan izin akses

- Direktori data dibuat dengan mode `0700`. Berkas data dengan `0600` (Unix).
- Penulisan berkas dilakukan atomik: tulis ke berkas sementara, `fsync`, lalu ganti nama.
- Di Windows, perlindungan bergantung pada ACL profil pengguna.

---

## 9. Yang sengaja tidak ada

- **Telemetri:** nol secara default. Laporan crash dan umpan balik hanya dikirim jika pengguna mengizinkannya.
- **Pembayaran:** tidak diproses di aplikasi. Pembayaran dilakukan di server lisensi, dan aplikasi hanya menerima token yang ditandatangani.
- **Format lama:** vault dari rilis pra-rilis tidak dimigrasi atau dihapus otomatis. Aplikasi melaporkannya sebagai `LegacyFormat`.

---

## 10. Privasi AI

- Mode: nonaktif, BYO (endpoint OpenAI-compatible yang Anda tentukan), atau hosted (hanya jika Pro dan endpoint server dikonfigurasi).
- Data yang dikirim default berupa ringkasan tanpa nama dan tanpa teks bebas.
- Detail baru dikirim setelah pengguna menyetujui per percakapan, dengan pratinjau isi yang akan dikirim.
- Email, nomor kartu, nomor telepon, NIK, NPWP, dan token disamarkan sebelum dikirim.

---

## 11. Melaporkan kerentanan

Lihat [`SECURITY.md`](../SECURITY.md). Jangan melaporkan kerentanan lewat issue publik.
