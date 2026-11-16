# Kontrak Backend (IPC Tauri)

Dokumen ini menjelaskan kontrak komunikasi antara frontend Svelte dan backend Rust (`caf-app` dan `caf-core`).

**Sumber kebenaran adalah kode:** daftar command dan kebijakannya ada di `crates/caf-core/src/api/policy.rs` (`COMMAND_POLICIES`). Jika dokumen ini berbeda dengan file itu, file itu yang benar.

---

## 1. Kelas kebijakan

Setiap command terdaftar dengan satu kebijakan:

| Kebijakan | Arti |
|---|---|
| `Public` | Berjalan walau vault terkunci (pengaturan awal, unlock, status, aplikasi info). |
| `Unlocked` | Vault harus terbuka, tetapi belum perlu sesi profil. |
| `Session` | Vault terbuka **dan** ada sesi profil aktif. |
| `Requires("<resource>:<action>")` | Sesi aktif dengan izin RBAC tertentu. |

Pemeriksaan dilakukan di **core**. Frontend tidak pernah mengirim identitas atau peran sebagai argumen.

## 2. Command yang terdaftar saat ini (45)

| Kebijakan | Command |
|---|---|
| `Public` (10) | `window_minimize`, `window_maximize`, `window_close`, `window_start_dragging`, `is_vault_initialized`, `validate_vault_password`, `lock_vault`, `submit_feedback`, `get_pending_crash_report`, `dismiss_crash_report` |
| `Unlocked` (2) | `list_profiles`, `verify_pin` |
| `Session` (19) | `save_profile`, `list_notes`, `save_note`, `delete_note`, `get_ai_settings`, `ai_preview_context`, `ai_chat`, `ai_get_providers`, `ai_get_routing_matrix`, `ai_get_personas`, `ai_get_habits`, `ai_search_habits`, `ai_toggle_habit_pin`, `ai_delete_habit`, `ai_get_skills`, `ai_dispatch_task`, `ai_dispatch_task_with_skill`, `get_performance_prefs`, `set_performance_prefs` |
| `Requires("security:manage")` (2) | `change_master_password`, `reset_vault` |
| `Requires("settings:manage")` (1) | `save_ai_settings` |
| `Requires("ai:manage")` (9) | `set_ai_api_key`, `clear_ai_api_key`, `ai_save_provider`, `ai_delete_provider`, `ai_save_routing_rule`, `ai_save_persona`, `ai_delete_persona`, `ai_save_skill`, `ai_delete_skill` |
| `Requires("backup:export")` (1) | `export_encrypted_backup` |
| `Requires("backup:import")` (1) | `import_encrypted_backup` |

Catatan untuk peninjau: `save_profile` hanya meminta sesi aktif, tanpa izin RBAC di tingkat kebijakan. Pastikan pemeriksaan izin (mis. `users:manage`) ada di dalam core. Target PRD: setiap perubahan profil dan peran melalui izin.

## 3. Aturan daftar dan binding

- Command yang mengembalikan daftar ditandai `returns_list` di kebijakan. Target PRD: semua command seperti itu berpagination (kursor, maksimal 500 per halaman) dan dijaga tes arsitektur. Penegakan di kode belum diverifikasi.
- Frontend hanya memanggil **binding yang dibangkitkan** (`frontend/src/lib/generated/commands.ts`), tidak memanggil `invoke()` langsung. Alasan dan keputusannya ada di [ADR 0003](adr/0003-ipc-bindings.md).
- Command di `caf-app` tipis: urai argumen, panggil `caf-core`, kembalikan hasil.

## 4. Model error

Semua command mengembalikan error terstruktur sesuai taksonomi `CafError`. Daftar kodenya ada di [`08-error-codes.md`](08-error-codes.md).

## 5. Kontrak server (GCC)

Kontrak HTTP ke GCC (lisensi, paket, checkout, perangkat) ada di [`contracts/gcc-billing-v1.yaml`](contracts/gcc-billing-v1.yaml). Penjelasannya ada di [`07-billing-guide.md`](07-billing-guide.md).
