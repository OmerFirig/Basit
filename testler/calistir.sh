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
# repl/     : etkileşimli kip; .girdi borudan verilir, çıktı .beklenen ile karşılaştırılır

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
    # Windows'ta farklı olan çıktılar (ör. kütüphane dosya adı .dll): ad.windows.beklenen
    [ "${OS:-}" = Windows_NT ] && [ -f "${1%.kolay}.windows.beklenen" ] && exp="${1%.kolay}.windows.beklenen"
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

# repl/: etkileşimli kip (girdi borudan verilir)
for f in testler/repl/*.girdi; do
    exp="${f%.girdi}.beklenen"
    out=$(cd testler/repl && ../../$B < "$(basename "$f")" 2>&1)
    if [ $guncelle = 1 ]; then printf '%s' "$out" > "$exp"; continue; fi
    if [ -f "$exp" ] && [ "$out" = "$(cat "$exp")" ]; then gecen=$((gecen + 1)); else
        kalan=$((kalan + 1)); echo "BAŞARISIZ: $f"; diff <(printf '%s\n' "$out") <(cat "$exp" 2>/dev/null; echo) | head -10; fi
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
