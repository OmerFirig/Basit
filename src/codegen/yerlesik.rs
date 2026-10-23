// Hazır metotlar (liste, sözlük, yazı, ondalık, aralık) ve değerleri yazıya çevirme.

use super::Gen;
use crate::diag::CResult;
use crate::tir::*;
use crate::types::{Type, TAM64};

impl<'p> Gen<'p> {
    /// Değeri yazıya çevirir. Yazı ise kendisini döndürür (sahiplik durumu değişmez); diğer
    /// tipler için yeni (sahipli) yazı oluşur, girdi bırakılmaz.
    pub(super) fn to_str(&mut self, v: &str, ty: &Type) -> CResult<String> {
        Ok(match ty {
            Type::Str => v.to_string(),
            Type::Hic | Type::Never => self.str_const("Hiç"),
            Type::Bool => {
                let (d, y) = (self.str_const("doğru"), self.str_const("yanlış"));
                let t = self.tmp();
                self.emit(&format!("{t} = select i1 {v}, ptr {d}, ptr {y}"));
                t
            }
            Type::Int { bits: Some(_), .. } => {
                let (bits, mode) = self.int128_form(v, ty);
                let t = self.tmp();
                self.emit(&format!("{t} = call ptr @kolay_tam_yazi(i128 {bits}, i32 {mode})"));
                self.own(t, &Type::Str)
            }
            Type::Int { bits: None, .. } => {
                let t = self.tmp();
                self.emit(&format!("{t} = call ptr @kolay_esnek_yazi(i64 {v})"));
                self.own(t, &Type::Str)
            }
            Type::Float(b) => {
                let d = self.to_double(v, *b);
                let t = self.tmp();
                self.emit(&format!("{t} = call ptr @kolay_ondalik_yazi(double {d}, i32 {b})"));
                self.own(t, &Type::Str)
            }
            _ => {
                let p = self.spill(v, ty);
                let tip = self.tip(ty);
                let t = self.tmp();
                self.emit(&format!("{t} = call ptr @kolay_yaziya(ptr {p}, ptr {tip})"));
                self.own(t, &Type::Str)
            }
        })
    }

    fn is_mutating(m: BM) -> bool {
        matches!(m, BM::Ekle | BM::ArayaEkle | BM::Sil | BM::Cikar | BM::Sirala | BM::Ters | BM::Temizle | BM::DSil | BM::DTemizle)
    }

    pub(super) fn bmethod(&mut self, m: BM, recv: &Recv, args: &[TExpr], ret: &Type, loc: Loc) -> CResult<String> {
        // argümanlar önce hesaplanır (yorumlayıcıdaki gibi)
        let mut a = Vec::new();
        for x in args {
            a.push(self.expr(x)?);
        }
        let r = if Self::is_mutating(m) {
            self.mutating_bmethod(m, recv, args, &a, loc)?
        } else {
            let (v, rty) = match recv {
                Recv::Value(x) => (self.expr(x)?, x.ty.clone()),
                Recv::Place(p) => {
                    let v = self.place_value(p, loc)?;
                    (v, self.place_type(p))
                }
            };
            let r = self.value_bmethod(m, &v, &rty, args, &a, ret, loc)?;
            if r != v {
                self.drop_temp(&v);
            }
            r
        };
        for x in &a {
            if *x != r {
                self.drop_temp(x);
            }
        }
        Ok(r)
    }

    /// Yerin statik tipi
    fn place_type(&self, p: &TPlace) -> Type {
        match p {
            TPlace::Var(s) => self.slot_ty(*s).cloned().expect("tip"),
            TPlace::Bu => Type::Named(self.f.owner.expect("tür")),
            TPlace::Field(b, f) => {
                let Type::Named(s) = self.place_type(b) else { unreachable!() };
                self.field_type(s, *f)
            }
            TPlace::ListIndex(b, _) => match self.place_type(b) {
                Type::List(e, _) => *e,
                _ => unreachable!(),
            },
            TPlace::DictKey(b, _) => match self.place_type(b) {
                Type::Dict(_, v) => *v,
                _ => unreachable!(),
            },
        }
    }

    fn mutating_bmethod(&mut self, m: BM, recv: &Recv, args: &[TExpr], a: &[String], loc: Loc) -> CResult<String> {
        // değiştirilecek nesnenin yeri: değişken/alan/eleman ya da geçici bir yer
        let (addr, ty, reg, temp) = match recv {
            Recv::Place(p) => {
                let (addr, ty, reg) = self.place_mut_addr(p, false, loc)?;
                (addr, ty, reg, false)
            }
            Recv::Value(x) => {
                let v = self.expr(x)?;
                self.ensure_owned(&v, &x.ty);
                self.disown(&v);
                let al = self.alloca("ptr");
                self.emit(&format!("store ptr {v}, ptr {al}"));
                (al, x.ty.clone(), true, true)
            }
        };
        let c = self.make_unique(&addr, &ty, reg);
        let (k, kd) = self.where_(loc);
        let mut result = String::new();
        match m {
            BM::Ekle => {
                let Type::List(e, _) = &ty else { unreachable!() };
                let e = (**e).clone();
                self.keep_ref(&a[0], &e);
                let p = self.spill(&a[0], &e);
                let n = self.tmp();
                self.emit(&format!("{n} = call ptr @kolay_liste_ekle(ptr {c}, ptr {p})"));
                self.store_raw_ptr(&addr, &n);
            }
            BM::ArayaEkle => {
                let Type::List(e, _) = &ty else { unreachable!() };
                let e = (**e).clone();
                self.keep_ref(&a[1], &e);
                let p = self.spill(&a[1], &e);
                let n = self.tmp();
                self.emit(&format!("{n} = call ptr @kolay_liste_araya(ptr {c}, i64 {}, ptr {p}, ptr {k}, ptr {kd})", a[0]));
                self.store_raw_ptr(&addr, &n);
            }
            BM::Sil => self.emit(&format!("call void @kolay_liste_sil(ptr {c}, i64 {}, ptr {k}, ptr {kd})", a[0])),
            BM::Cikar => {
                let Type::List(e, _) = &ty else { unreachable!() };
                let e = (**e).clone();
                let st = self.stor(&e);
                let out = self.alloca(&st.ll);
                self.emit(&format!("call void @kolay_liste_cikar(ptr {c}, ptr {out}, ptr {k}, ptr {kd})"));
                let v = self.load_mem(&out, &e);
                result = self.own(v, &e);
            }
            BM::Sirala => self.emit(&format!("call void @kolay_liste_sirala(ptr {c})")),
            BM::Ters => self.emit(&format!("call void @kolay_liste_ters(ptr {c})")),
            BM::Temizle => self.emit(&format!("call void @kolay_liste_temizle(ptr {c})")),
            BM::DSil => {
                let p = self.spill(&a[0], &args[0].ty);
                self.emit(&format!("call void @kolay_sozluk_sil(ptr {c}, ptr {p}, ptr {k}, ptr {kd})"));
            }
            BM::DTemizle => self.emit(&format!("call void @kolay_sozluk_temizle(ptr {c})")),
            _ => unreachable!(),
        }
        if temp {
            let v = self.tmp();
            self.emit(&format!("{v} = load ptr, ptr {addr}"));
            self.emit(&format!("call void @kolay_birak(ptr {v})"));
        }
        let _ = reg;
        Ok(result)
    }

    /// Liste büyürken yer değiştirebilir: yeni adres yerine yazılır (eski nesne artık yok).
    fn store_raw_ptr(&mut self, addr: &str, v: &str) {
        self.emit(&format!("store ptr {v}, ptr {addr}"));
    }

    #[allow(clippy::too_many_arguments)]
    fn value_bmethod(&mut self, m: BM, v: &str, rty: &Type, args: &[TExpr], a: &[String], ret: &Type, loc: Loc) -> CResult<String> {
        let (k, kd) = self.where_(loc);
        let t = self.tmp();
        Ok(match m {
            BM::Yaziya => return self.to_str(v, rty),
            BM::FTama => return self.olarak(v, rty, &TAM64, loc, true),
            BM::Uzunluk => {
                let lp = self.tmp();
                self.emit(&format!("{lp} = getelementptr i8, ptr {v}, i64 16"));
                self.emit(&format!("{t} = load i64, ptr {lp}{}", super::BASLIK));
                t
            }
            BM::Icerir => {
                let p = self.spill(&a[0], &args[0].ty);
                self.emit(&format!("{t} = call zeroext i1 @kolay_liste_icerir(ptr {v}, ptr {p})"));
                t
            }
            BM::Bul => {
                let p = self.spill(&a[0], &args[0].ty);
                self.emit(&format!("{t} = call i64 @kolay_liste_bul(ptr {v}, ptr {p})"));
                // sonuç: Tam(64) veya Hiç (bulunamazsa Hiç)
                let Type::Union(ms) = ret else { unreachable!() };
                let ti = ms.iter().position(|x| *x == TAM64).unwrap();
                let hi = ms.iter().position(|x| *x == Type::Hic).unwrap();
                let (neg, tag) = (self.tmp(), self.tmp());
                self.emit(&format!("{neg} = icmp slt i64 {t}, 0"));
                self.emit(&format!("{tag} = select i1 {neg}, i64 {hi}, i64 {ti}"));
                let (x, y, z) = (self.tmp(), self.tmp(), self.tmp());
                self.emit(&format!("{x} = insertvalue %veya undef, i64 {tag}, 0"));
                self.emit(&format!("{y} = insertvalue %veya {x}, i64 {t}, 1"));
                self.emit(&format!("{z} = insertvalue %veya {y}, i64 0, 2"));
                z
            }
            BM::DUzunluk => {
                self.emit(&format!("{t} = call i64 @kolay_sozluk_uzunluk(ptr {v})"));
                t
            }
            BM::DIcerir => {
                let p = self.spill(&a[0], &args[0].ty);
                self.emit(&format!("{t} = call zeroext i1 @kolay_sozluk_icerir(ptr {v}, ptr {p})"));
                t
            }
            BM::Anahtarlar | BM::Degerler => {
                let tip = self.tip(ret);
                let f = if m == BM::Anahtarlar { "kolay_sozluk_anahtarlar" } else { "kolay_sozluk_degerler" };
                self.emit(&format!("{t} = call ptr @{f}(ptr {v}, ptr {tip})"));
                self.own(t, ret)
            }
            BM::SUzunluk => {
                self.emit(&format!("{t} = call i64 @kolay_yazi_uzunluk(ptr {v})"));
                t
            }
            BM::BuyukHarf | BM::KucukHarf | BM::Kirp => {
                let f = match m {
                    BM::BuyukHarf => "kolay_buyuk_harf",
                    BM::KucukHarf => "kolay_kucuk_harf",
                    _ => "kolay_kirp",
                };
                self.emit(&format!("{t} = call ptr @{f}(ptr {v})"));
                self.own(t, &Type::Str)
            }
            BM::SIcerir => {
                self.emit(&format!("{t} = call zeroext i1 @kolay_icerir(ptr {v}, ptr {})", a[0]));
                t
            }
            BM::Karakter => {
                self.emit(&format!("{t} = call ptr @kolay_karakter(ptr {v}, i64 {}, ptr {k}, ptr {kd})", a[0]));
                self.own(t, &Type::Str)
            }
            BM::Bol => {
                let tip = self.tip(ret);
                self.emit(&format!("{t} = call ptr @kolay_yazi_bol(ptr {v}, ptr {}, ptr {tip})", a[0]));
                self.own(t, ret)
            }
            BM::Tama => {
                self.emit(&format!("{t} = call i64 @kolay_yazi_tama(ptr {v}, ptr {k}, ptr {kd})"));
                t
            }
            BM::Ondaliga => {
                self.emit(&format!("{t} = call double @kolay_yazi_ondaliga(ptr {v}, ptr {k}, ptr {kd})"));
                t
            }
            BM::Adim => {
                let n = &a[0];
                let bad = self.tmp();
                self.emit(&format!("{bad} = icmp sle i64 {n}, 0"));
                let (badl, ok) = (self.new_label(), self.new_label());
                self.br_cold(&bad, &badl, &ok);
                self.block(&badl);
                let (once, sonra) = (self.cstr("Adım pozitif olmalı, ama "), self.cstr(" verildi."));
                self.emit(&format!("call void @kolay_hata_sayi(ptr {once}, i64 {n}, ptr {sonra}, ptr {k}, ptr {kd})"));
                self.term("unreachable");
                self.block(&ok);
                self.emit(&format!("{t} = insertvalue %aralik {v}, i64 {n}, 2"));
                t
            }
            _ => unreachable!("yerleşik metot {m:?}"),
        })
    }
}

