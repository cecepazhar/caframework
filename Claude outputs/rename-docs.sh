#!/usr/bin/env bash
# Rename docs/ ke format bernomor memakai git mv (riwayat git terjaga).
# Jalankan dari mana saja: bash rename-docs.sh /home/cecepazhar/Product/caframework
set -euo pipefail
REPO="${1:-.}"
cd "$REPO/docs"

MAP=(
  "architecture.md 01-architecture.md"
  "security.md 02-security.md"
  "duplicating-to-a-product.md 03-duplicating-to-a-product.md"
  "app-toml-reference.md 04-app-toml-reference.md"
  "creating-an-app.md 05-creating-an-app.md"
  "adding-a-menu.md 06-adding-a-menu.md"
  "billing-guide.md 07-billing-guide.md"
  "error-codes.md 08-error-codes.md"
  "backend-contract.md 09-backend-contract.md"
  "android-build-guide.md 10-android-build-guide.md"
  "android-signing.md 11-android-signing.md"
  "android-mobile-layout.md 12-android-mobile-layout.md"
  "porting-framework-fixes.md 13-porting-framework-fixes.md"
)

# Pemeriksaan awal: semua sumber ada, semua tujuan belum ada.
for pair in "${MAP[@]}"; do
  read -r old new <<<"$pair"
  [ -f "$old" ] || { echo "GAGAL: $old tidak ditemukan. Tidak ada yang diubah." >&2; exit 1; }
  [ ! -e "$new" ] || { echo "GAGAL: $new sudah ada. Tidak ada yang diubah." >&2; exit 1; }
done

for pair in "${MAP[@]}"; do
  read -r old new <<<"$pair"
  git mv "$old" "$new"
  echo "OK  $old -> $new"
done

echo
echo "Pemeriksa tautan (harus tanpa baris RUSAK):"
cd ..
for f in README.md STATUS.md AGENTS.md docs/*.md; do
  d=$(dirname "$f")
  grep -oE '\]\([^)#]+\.md' "$f" | sed 's/](//' | while read -r l; do
    [ -e "$d/$l" ] || echo "RUSAK: $f -> $l"
  done
done
echo "Selesai. Lanjut: git status"
