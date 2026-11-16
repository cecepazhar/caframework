# Tata Letak Mobile dan Kepatuhan Target Android

> **Status.** Dokumen ini mencatat aturan **target**. Validasi 4 Oktober 2026 menemukan shell UI (tema, aksen, title bar) belum sepenuhnya bekerja, jadi perilaku di bawah belum boleh dianggap terbukti. Lihat [`STATUS.md`](../STATUS.md).

## Aturan tata letak

1. **Title bar tersembunyi.** Di Android title bar desktop tanpa bingkai disembunyikan, dideteksi lewat lingkungan Tauri dan lebar layar.
2. **Navigasi.** Layar sempit memakai drawer atau bottom sheet agar mudah dijangkau ibu jari. Sidebar menjadi drawer responsif.
3. **Target sentuh.** Semua elemen interaktif (tombol, input, saklar, tombol PIN) minimal **44x44 px**, mengikuti WCAG 2.5.5 dan pedoman Material.
4. **PIN pad.** Komponen PIN pad responsif untuk layar sentuh.
5. **Brankas terkunci di latar belakang.** Saat aplikasi masuk latar belakang, brankas dikunci (FR-13).

## Cara membuktikan

- Uji di perangkat Android nyata, bukan hanya emulator.
- Tes aksesibilitas otomatis (axe) tanpa pelanggaran serius atau kritis, dan `svelte-check` tanpa peringatan a11y (PRD bagian 6).
- Lampirkan tangkapan layar dan keluaran perintah sebagai bukti, bukan ringkasan.
