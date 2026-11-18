# 18. Build Variants & Feature Modules (A-01, A-02, A-03)

Dokumen ini menjelaskan arsitektur varian build (*build variants*) dan modul fitur (*feature modules*) pada CAFramework `v0.1.0`.

---

## 1. Arsitektur Modul Fitur (A-01)

Satu repositori produk dapat mendefinisikan beberapa modul independen di bawah folder `modules/<key>/module.toml`.

### Format Manifest `module.toml`

```toml
[module]
key = "orders"                         # ^[a-z][a-z0-9_]{1,31}$, unik
label_en = "Orders"
label_id = "Pesanan"
cargo_feature = "mod-orders"           # Harus dideklarasikan di Cargo.toml
depends_on = ["catalog"]               # Ketergantungan modul lain
conflicts_with = []                    # Konflik antar modul

[[module.features]]                    # Saklar fitur runtime (A-03)
key = "reservation"
default = true
locked_when = "has_open_reservation"   # Predikat core untuk proteksi pematian

[module.provides]                      # Digabungkan ke app registries saat codegen
menus = ["orders"]
permissions = ["orders:view", "orders:create", "orders:manage"]
entities = ["orders", "order_lines"]
jobs = []
integrations = []
settings_sections = ["orders"]
ai_context_providers = []

[module.extra]                         # Tabel ekstensi bebas untuk produk
custom_table_rate = 12.5
```

---

## 2. Arsitektur Varian Build (A-02)

Satu repositori produk dapat menghasilkan beberapa aplikasi siap install (*variants*) yang berbeda nama, aksen, ikon, identifier, dan modul aktif.

### Format Konfigurasi `variants/<key>.toml`

```toml
[variant]
key = "lite"                           # ^[a-z][a-z0-9-]{1,30}$
extends = "app.toml"                   # Konfigurasi dasar yang ditimpa

[variant.app]                          # Override subset [app]
name = "Example Lite"
identifier = "com.example.app.lite"    # WAJIB BERBEDA dari base app.identifier
accent = "sky"
platforms = ["windows", "android"]
icon = "assets/variants/lite/icon.png"

[variant.modules]
include = ["catalog", "orders"]        # Modul produk yang dikompilasi
framework = { ai = false, scheduler = true } # Override modul framework

[variant.defaults]                     # Nilai default saklar runtime (A-03)
"orders.reservation" = false

[variant.locked]                       # Saklar yang dikunci untuk pengguna akhir
keys = ["orders"]

[variant.pro]                          # Override lisensi per-varian
gcc_product_id = "gcc-prod-lite-2026"

[variant.pro.features]
ai_assistant = "pro"
```

---

## 3. Isolasi Data Directory & Lisensi (P15.3)

Setiap varian mendapatkan direktori data OS yang sepenuhnya terisolasi berdasarkan `identifier`:
- **Linux**: `~/.local/share/<identifier>`
- **Android**: `/data/data/<identifier>/files/caframework`
- **Windows**: `%APPDATA%/<identifier>`

Dengan demikian, varian `lite` dan varian `pro` dapat diinstal secara berdampingan (*side-by-side*) pada satu perangkat tanpa bentrok database atau token kunci.

Lisensi GCC Billing Hub (`gcc_product_id`) juga dapat ditentukan berbeda per varian, memungkinkan aktivasi dan monetisasi tier produk yang terpisah.

---

## 4. Perintah CLI (`caf-xtask variant`)

CAFramework menyediakan sub-perintah `variant` melalui `caf-xtask`:

### 1. `variant validate [--variant <key>|--all]`
Memvalidasi sintaks dan grafik dependensi varian:
```bash
cargo run -p caf-xtask -- variant validate --all
```

### 2. `variant list`
Menampilkan daftar seluruh varian yang terdaftar:
```bash
cargo run -p caf-xtask -- variant list
```

### 3. `variant diff <a> <b>`
Membandingkan perbedaan konfigurasi, modul, dan Cargo features antara dua varian (atau antara `base` dan varian):
```bash
cargo run -p caf-xtask -- variant diff lite pro
```

### 4. `variant matrix --format md`
Menghasilkan tabel matriks Modul × Varian dalam format Markdown:
```bash
cargo run -p caf-xtask -- variant matrix --format md
```

### 5. `variant build --variant <key> --target <target> [--release]`
Membangun varian dengan codegen terisolasi dan Cargo features yang sesuai:
```bash
cargo run -p caf-xtask -- variant build --variant lite --target linux
```
*Garansi Arsitektur*: Proses build varian **tidak pernah meninggalkan working tree kotor**. Seluruh file codegen yang dimutasi untuk varian otomatis dikembalikan ke kondisi awal setelah proses build selesai.

---

## 5. Contoh Referensi (`docs/examples/variants/`)

Struktur contoh mandiri tersedia di `docs/examples/variants/`:
```
docs/examples/variants/
├── app.toml
├── variants/
│   ├── lite.toml
│   └── pro.toml
├── modules/
│   ├── catalog/
│   │   └── module.toml
│   ├── orders/
│   │   └── module.toml
│   └── analytics/
│       └── module.toml
└── assets/
    ├── logo.svg
    └── variants/
        ├── lite/icon.png
        └── pro/icon.png
```
Contoh ini diverifikasi secara otomatis oleh tes gate `VS-V` (`cargo test -p caf-xtask --test variants`).
