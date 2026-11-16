# Wizard dan App Builder

> **Status: rencana.** Perintah `wizard` **belum ada** di `caf-xtask` (subcommand yang ada: `generate-icons`, `codegen`, `new-app`, `guard`, `check`, `gcc-mock`). Dokumen ini mencatat desain target (PRD FR-19). Lihat [`STATUS.md`](../STATUS.md).

Wizard membantu membuat `app.toml` langkah demi langkah lalu, bila diminta, menjalankan `new-app`. Sampai wizard ada, tulis `app.toml` dengan tangan memakai [`04-app-toml-reference.md`](04-app-toml-reference.md) dan [`examples/app.example.toml`](examples/app.example.toml).

---

## 1. Wizard di terminal (target)

Perintah target: `cargo run -p caf-xtask -- wizard`. Langkahnya:

1. Identitas (nama, lalu usulan slug, kode, dan identifier).
2. Brand (terisi otomatis dari berkas konfigurasi pengembang; nilainya bisa diganti).
3. Aksen warna dengan pratinjau.
4. Bahasa dan platform.
5. Modul (backup ditampilkan sebagai wajib).
6. Peran dan preset izin.
7. Menu (kunci, label, ikon dari set bawaan, grup, izin).
8. Entitas (bidang). Lihat [`14-adding-an-entity.md`](14-adding-an-entity.md).
9. Integrasi. Lihat [`15-adding-an-integration.md`](15-adding-an-integration.md).
10. Job. Lihat [`16-adding-a-job.md`](16-adding-a-job.md).
11. Fitur Pro (`free` atau `pro` per fitur).

Setiap langkah divalidasi seketika. Layar tinjauan menampilkan `app.toml` yang dihasilkan; wizard menulisnya dan, bila diminta, menjalankan `new-app`. **Jika dihentikan di tengah jalan, tidak ada yang ditulis.**

## 2. Mode non-interaktif (target)

`--answers answers.toml` untuk agent dan CI. Tiga berkas jawaban contoh (aplikasi catatan satu pengguna, aplikasi tim dengan empat peran, aplikasi keuangan dengan fitur Pro dan dua integrasi) menjadi fixture uji yang semuanya harus lolos verifikasi (PRD kriteria sukses 6).

## 3. App Builder (target)

Halaman `/_dev/builder` hanya ada di build dev CAFramework sendiri (fitur Cargo `builder`), bukan di aplikasi hasil generate. Isinya langkah yang sama dalam bentuk formulir dengan pratinjau langsung (sidebar, aksen, matriks peran, footer layar kunci), dan menghasilkan berkas lewat pustaka yang sama dengan CLI.

## 4. Satu skema

JSON Schema dibangkitkan dari tipe Rust dan dipakai CLI, GUI, dan validator, jadi ketiganya tidak bisa berbeda aturan. Pustaka itu (`caf-template`) juga **belum ada** di workspace.

## 5. Bukti yang harus ada saat fitur selesai

- Tiga fixture jawaban menghasilkan aplikasi yang lolos semua perintah verifikasi tanpa suntingan manual.
- Uji bahwa menghentikan wizard tidak meninggalkan berkas.
- Uji bahwa `app.toml` hasil wizard identik dengan hasil mode non-interaktif untuk jawaban yang sama.
