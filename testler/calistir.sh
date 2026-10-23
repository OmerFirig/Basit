#!/usr/bin/env bash
# Kolay test çalıştırıcısı
#
#   ./testler/calistir.sh              testleri çalıştırır
#   ./testler/calistir.sh --guncelle   beklenen çıktıları (.beklenen) yeniden yazar
#
# calisma/  : çalışması gereken programlar, çıktıları .beklenen ile karşılaştırılır
# hata/     : hata vermesi gereken programlar (numarayla başlayanlar), mesajlar .beklenen ile karşılaştırılır
# moduller/ : dahil et testleri (ana.kolay çalıştırılır)
# derleme/  : yorumlayıcı ve derleyici (kolay derle) aynı çıktıyı vermeli

set -u
cd "$(dirname "$0")/.."
cargo build --release -q || exit 1
B=./target/release/kolay
gecen=0
kalan=0
guncelle=0
[ "${1:-}" = "--guncelle" ] && guncelle=1

kontrol() {
    local f="$1" out="$2" exp="${1%.kolay}.beklenen"
    if [ $guncelle = 1 ]; then
        printf '%s' "$out" > "$exp"
        return
    fi
    if [ -f "$exp" ] && [ "$out" = "$(cat "$exp")" ]; then
        gecen=$((gecen + 1))
    else
        kalan=$((kalan + 1))
        echo "BAŞARISIZ: $f"
        diff <(printf '%s\n' "$out") <(cat "$exp" 2>/dev/null; echo) | head -10
    fi
}

for f in testler/calisma/*.kolay testler/hata/[0-9]*.kolay testler/moduller/ana.kolay; do
    kontrol "$f" "$($B "$f" 2>&1)"
done

tmp=$(mktemp -d)
for f in testler/derleme/*.kolay; do
    y=$($B "$f" 2>&1)
    if $B derle "$f" -o "$tmp/program" >/dev/null 2>"$tmp/hata"; then
        d=$("$tmp/program" 2>&1)
    else
        d="DERLEME HATASI: $(cat "$tmp/hata")"
    fi
    if [ "$y" = "$d" ]; then
        kontrol "$f" "$y"
    else
        kalan=$((kalan + 1))
        echo "BAŞARISIZ (yorumlayıcı ve derleyici farklı): $f"
        diff <(printf '%s\n' "$y") <(printf '%s\n' "$d") | head -10
    fi
done
rm -rf "$tmp"

if [ $guncelle = 1 ]; then
    echo "Beklenen çıktılar güncellendi."
    exit 0
fi
echo "Geçen: $gecen, başarısız: $kalan"
[ $kalan = 0 ]
