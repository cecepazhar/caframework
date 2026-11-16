# Menambah integrasi API

> **Status: rencana.** Lapisan konektor dan adapter GCC generik (PRD FR-15) **belum ada di kode**. Yang ada hari ini hanya klien billing GCC khusus (`caf-core/src/billing/`, lihat [`07-billing-guide.md`](07-billing-guide.md)). Dokumen ini mencatat desain target; contoh kode Rust akan ditambahkan setelah fiturnya terbukti. Lihat [`STATUS.md`](../STATUS.md).

Integrasi menghubungkan produk ke layanan luar melalui satu lapisan konektor di core. GCC adalah adapter bawaan; layanan lain didaftarkan lewat `app.toml` dan satu berkas Rust per adapter, **tanpa mengubah kode framework**.

---

## 1. Mendeklarasikan integrasi (target)

```toml
[[integrations]]
key = "gcc"
kind = "gcc"                       # gcc | rest
base_url_from = "endpoints.gcc_base_url"

[[integrations]]
key = "weather"
kind = "rest"
base_url = ""                      # kosong + user_configurable = diatur di Pengaturan
user_configurable = true
auth = "api_key_header"            # none | api_key_header | api_key_query | bearer | basic | hmac_sha256 | oauth2_client_credentials
auth_header = "X-Api-Key"
timeout_ms = 15000
retries = 2
```

Selain di atas, tiap integrasi punya batas laju, batas konkurensi, dan daftar host yang diizinkan (diperiksa pada setiap permintaan dan setiap redirect). Kunci integrasi harus unik dan aman untuk URL.

## 2. Rahasia

Kunci API dan kredensial disimpan di **brankas**, diatur lewat Pengaturan → Integrasi oleh profil dengan izin `integrations:manage`. Frontend tidak pernah menerimanya, dan hanya mendapat status "sudah diisi".

## 3. Perilaku konektor (target)

- Uji koneksi dari Pengaturan.
- Log permintaan dengan redaksi: header dan isi tidak disimpan.
- Retry dengan back-off **hanya** untuk permintaan idempoten atau yang membawa kunci idempotensi.
- Batas ukuran respons.
- Outbox offline untuk penulisan, dikosongkan oleh job terjadwal `integrations.outbox_flush` (lihat [`16-adding-a-job.md`](16-adding-a-job.md)).
- Klien HTTP tunggal di core: HTTPS saja (kecuali loopback), batas waktu, User-Agent `<slug>/<versi> (+<website brand>)`, dan galat jaringan yang bertipe.
- Pembersih PII yang sama dipakai untuk log integrasi.

## 4. Adapter GCC bawaan

Mencakup registrasi perangkat, lisensi, paket Pro, AI terhosting, umpan balik, laporan crash, penunjuk manifes pembaruan, kontrak dorong/tarik data generik, dan waktu server. Setiap panggilan ditandatangani dan membawa `X-CA-App-Id`, `X-CA-App-Version`, dan `X-CA-Device-Id`. Kontraknya: [`09-backend-contract.md`](09-backend-contract.md) dan [`contracts/`](contracts/).

## 5. Bukti yang harus ada saat fitur selesai

- Uji dengan server mock: sukses, timeout, 5xx, redirect ke host lain (harus ditolak), respons terlalu besar.
- Uji bahwa rahasia tidak muncul di log, galat, atau dump crash.
- Uji retry tidak mengulang permintaan non-idempoten.
- Uji outbox: tulis saat offline, kirim saat online, tanpa duplikasi.
