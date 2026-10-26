// Hazır metotlar (liste, sözlük, yazı, ondalık, aralık) ve değerleri yazıya çevirme.

use super::Gen;
use crate::diag::CResult;
use crate::tir::*;
use crate::types::{Type, TAM64};
use crate::tir::is_hof;

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
                let bits = self.i128_ptr(&bits);
                let t = self.tmp();
                self.emit(&format!("{t} = call ptr @kolay_tam_yazi(ptr {bits}, i32 {mode})"));
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
        matches!(m, BM::Ekle | BM::ArayaEkle | BM::Sil | BM::Cikar | BM::Sirala | BM::TersCevir | BM::Temizle | BM::DSil | BM::DTemizle)
    }

    pub(super) fn bmethod(&mut self, m: BM, recv: &Recv, args: &[TExpr], ret: &Type, loc: Loc) -> CResult<String> {
        if is_hof(m) || matches!(m, BM::Toplam | BM::EnBuyuk | BM::EnKucuk) {
            return self.list_hof(m, recv, args, ret, loc);
        }
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

    /// Listenin her elemanı için `body` (elemanın değeri ödünç; çıkış etiketine dallanılabilir)
    fn liste_dongu(&mut self, l: &str, et: &Type, mut body: impl FnMut(&mut Self, &str, &str) -> CResult<()>) -> CResult<()> {
        let ctr = self.alloca("i64");
        self.emit(&format!("store i64 0, ptr {ctr}"));
        let (cond, bodyl, exit) = (self.new_label(), self.new_label(), self.new_label());
        self.block(&cond);
        let (lp, len, i, c) = (self.tmp(), self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{lp} = getelementptr i8, ptr {l}, i64 16"));
        self.emit(&format!("{len} = load i64, ptr {lp}{}", super::BASLIK));
        self.emit(&format!("{i} = load i64, ptr {ctr}"));
        self.emit(&format!("{c} = icmp slt i64 {i}, {len}"));
        self.term(&format!("br i1 {c}, label %{bodyl}, label %{exit}"));
        self.block(&bodyl);
        let st = self.stor(et);
        let (dp, ep) = (self.tmp(), self.tmp());
        self.emit(&format!("{dp} = getelementptr i8, ptr {l}, i64 32"));
        self.emit(&format!("{ep} = getelementptr {}, ptr {dp}, i64 {i}", st.ll));
        let x = self.load_mem(&ep, et);
        body(self, &x, &exit)?;
        let inc = self.tmp();
        self.emit(&format!("{inc} = add i64 {i}, 1"));
        self.emit(&format!("store i64 {inc}, ptr {ctr}"));
        self.term(&format!("br label %{cond}"));
        self.block(&exit);
        Ok(())
    }

    /// Fonksiyon değerini çağırır (ilk argüman nesnenin kendisi). Dönen değer sahiplidir.
    pub(super) fn closure_call(&mut self, fv: &str, args: &[(String, Type)], ret: &Type) -> String {
        let (fpp, fp) = (self.tmp(), self.tmp());
        let off = self.cagri_adresi_ofset();
        self.emit(&format!("{fpp} = getelementptr i8, ptr {fv}, i64 {off}"));
        self.emit(&format!("{fp} = load ptr, ptr {fpp}"));
        let mut all = vec![format!("ptr {fv}")];
        for (v, t) in args {
            all.push(format!("{} {v}", self.llty(t)));
        }
        if *ret == Type::Hic {
            self.emit(&format!("call void {fp}({})", all.join(", ")));
            return String::new();
        }
        let lt = self.llty(ret);
        let r = self.tmp();
        self.emit(&format!("{r} = call {lt} {fp}({})", all.join(", ")));
        r
    }

    /// dönüşüm, filtre, hepsi, herhangi, sırala(göre:), sıralı(göre:), toplam, en_büyük, en_küçük
    /// (yorumlayıcıdaki Interp::list_hof ile aynı kurallar)
    fn list_hof(&mut self, m: BM, recv: &Recv, args: &[TExpr], ret: &Type, loc: Loc) -> CResult<String> {
        let fv = match args.first() {
            Some(x) => Some((self.expr(x)?, x.ty.clone())),
            None => None,
        };
        // liste tutulur: fonksiyon onu tutan değişkeni değiştirse bile döngü bu liste üzerinde döner
        let (l, lty) = match recv {
            Recv::Value(x) => {
                let v = self.expr(x)?;
                self.ensure_owned(&v, &x.ty);
                (v, x.ty.clone())
            }
            Recv::Place(p) => (self.place_value(p, loc)?, self.place_type(p)),
        };
        let Type::List(et, _) = &lty else { unreachable!() };
        let et = (**et).clone();
        let (f, fty) = fv.clone().unwrap_or((String::new(), Type::Hic));
        let fret = match &fty {
            Type::Func(_, r) => (**r).clone(),
            _ => Type::Hic,
        };
        let result = match m {
            BM::Donusum | BM::SiralaGore | BM::SiraliGore => {
                // sonuçlar (sıralamada anahtarlar) yeni bir listeye
                let lt_ = Type::List(Box::new(fret.clone()), None);
                let tip = self.tip(&lt_);
                let (lp, n) = (self.tmp(), self.tmp());
                self.emit(&format!("{lp} = getelementptr i8, ptr {l}, i64 16"));
                self.emit(&format!("{n} = load i64, ptr {lp}{}", super::BASLIK));
                let out = self.tmp();
                self.emit(&format!("{out} = call ptr @kolay_liste_yeni(ptr {tip}, i64 {n})"));
                let ctr = self.alloca("i64");
                self.emit(&format!("store i64 0, ptr {ctr}"));
                let st = self.stor(&fret);
                self.liste_dongu(&l, &et, |me, x, _| {
                    let r = me.closure_call(&f, &[(x.to_string(), et.clone())], &fret);
                    let i = me.tmp();
                    me.emit(&format!("{i} = load i64, ptr {ctr}"));
                    let (dp, ep, inc) = (me.tmp(), me.tmp(), me.tmp());
                    me.emit(&format!("{dp} = getelementptr i8, ptr {out}, i64 32"));
                    me.emit(&format!("{ep} = getelementptr {}, ptr {dp}, i64 {i}", st.ll));
                    me.store_mem(&ep, &r, &fret); // dönen değerin sahipliği listeye geçer
                    me.emit(&format!("{inc} = add i64 {i}, 1"));
                    me.emit(&format!("store i64 {inc}, ptr {ctr}"));
                    Ok(())
                })?;
                if m == BM::Donusum {
                    self.own(out, ret)
                } else {
                    let s = self.tmp();
                    self.emit(&format!("{s} = call ptr @kolay_liste_anahtarla(ptr {l}, ptr {out})"));
                    self.emit(&format!("call void @kolay_birak(ptr {out})"));
                    if m == BM::SiraliGore {
                        self.own(s, ret)
                    } else {
                        if let Recv::Place(p) = recv {
                            let (addr, pty, reg) = self.place_mut_addr(p, false, loc)?;
                            self.own(s.clone(), &pty);
                            self.store_at(&addr, &s, &pty, reg);
                        } else {
                            self.emit(&format!("call void @kolay_birak(ptr {s})"));
                        }
                        String::new()
                    }
                }
            }
            BM::Filtre => {
                let tip = self.tip(ret);
                let out = self.alloca("ptr");
                let n0 = self.tmp();
                self.emit(&format!("{n0} = call ptr @kolay_liste_yeni(ptr {tip}, i64 0)"));
                self.emit(&format!("store ptr {n0}, ptr {out}"));
                self.liste_dongu(&l, &et, |me, x, _| {
                    let b = me.closure_call(&f, &[(x.to_string(), et.clone())], &Type::Bool);
                    let (yes, no) = (me.new_label(), me.new_label());
                    me.term(&format!("br i1 {b}, label %{yes}, label %{no}"));
                    me.block(&yes);
                    if me.is_rc(&et) {
                        me.retain(x, &et);
                    }
                    let p = me.spill(x, &et);
                    let (cur, nl) = (me.tmp(), me.tmp());
                    me.emit(&format!("{cur} = load ptr, ptr {out}"));
                    me.emit(&format!("{nl} = call ptr @kolay_liste_ekle(ptr {cur}, ptr {p})"));
                    me.emit(&format!("store ptr {nl}, ptr {out}"));
                    me.block(&no);
                    Ok(())
                })?;
                let r = self.tmp();
                self.emit(&format!("{r} = load ptr, ptr {out}"));
                self.own(r, ret)
            }
            BM::Hepsi | BM::Herhangi => {
                let aranan = m == BM::Herhangi;
                let res = self.alloca("i1");
                self.emit(&format!("store i1 {}, ptr {res}", !aranan));
                self.liste_dongu(&l, &et, |me, x, exit| {
                    let b = me.closure_call(&f, &[(x.to_string(), et.clone())], &Type::Bool);
                    let (hit, cont) = (me.new_label(), me.new_label());
                    let c = me.tmp();
                    me.emit(&format!("{c} = icmp eq i1 {b}, {aranan}"));
                    me.term(&format!("br i1 {c}, label %{hit}, label %{cont}"));
                    me.block(&hit);
                    me.emit(&format!("store i1 {aranan}, ptr {res}"));
                    me.term(&format!("br label %{exit}"));
                    me.block(&cont);
                    Ok(())
                })?;
                let r = self.tmp();
                self.emit(&format!("{r} = load i1, ptr {res}"));
                r
            }
            BM::Toplam => {
                let lt = self.llty(&et);
                let acc = self.alloca(&lt);
                let zero = match &et {
                    Type::Float(b) => super::ifade::float_const(0.0, *b),
                    t if super::duzen::is_esnek(t) => "1".into(), // etiketli 0
                    _ => "0".into(),
                };
                self.emit(&format!("store {lt} {zero}, ptr {acc}"));
                self.liste_dongu(&l, &et, |me, x, _| {
                    let cur = me.tmp();
                    me.emit(&format!("{cur} = load {lt}, ptr {acc}"));
                    let nv = me.arith(crate::ast::BinOp::Add, &cur, x, &et, loc)?;
                    if me.is_rc(&et) {
                        me.disown(&nv);
                        me.release(&cur, &et);
                    }
                    me.emit(&format!("store {lt} {nv}, ptr {acc}"));
                    Ok(())
                })?;
                let r = self.tmp();
                self.emit(&format!("{r} = load {lt}, ptr {acc}"));
                if self.is_rc(&et) { self.own(r, &et) } else { r }
            }
            _ => {
                // en_büyük / en_küçük: boşsa hata; TanımsızSayı en büyük sayılır (sırala ile aynı)
                let buyuk = m == BM::EnBuyuk;
                let (lp, n, z) = (self.tmp(), self.tmp(), self.tmp());
                self.emit(&format!("{lp} = getelementptr i8, ptr {l}, i64 16"));
                self.emit(&format!("{n} = load i64, ptr {lp}{}", super::BASLIK));
                self.emit(&format!("{z} = icmp eq i64 {n}, 0"));
                let (bad, ok) = (self.new_label(), self.new_label());
                self.br_cold(&z, &bad, &ok);
                self.block(&bad);
                self.error_call_tur(hata_turu::INDEKS, if buyuk { "Liste boş; en büyük eleman yok." } else { "Liste boş; en küçük eleman yok." }, loc);
                self.block(&ok);
                let lt = self.llty(&et);
                let best = self.alloca(&lt);
                let (dp, first) = (self.tmp(), self.tmp());
                self.emit(&format!("{dp} = getelementptr i8, ptr {l}, i64 32"));
                let st = self.stor(&et);
                self.emit(&format!("{first} = getelementptr {}, ptr {dp}, i64 0", st.ll));
                let f0 = self.load_mem(&first, &et);
                self.emit(&format!("store {lt} {f0}, ptr {best}"));
                self.liste_dongu(&l, &et, |me, x, _| {
                    let b = me.tmp();
                    me.emit(&format!("{b} = load {lt}, ptr {best}"));
                    let daha = if let Type::Float(_) = &et {
                        let (c, xn, bn, nb, ek, r) = (me.tmp(), me.tmp(), me.tmp(), me.tmp(), me.tmp(), me.tmp());
                        let (op, yeni, eski) = if buyuk { ("ogt", &xn, &bn) } else { ("olt", &bn, &xn) };
                        me.emit(&format!("{c} = fcmp {op} {lt} {x}, {b}"));
                        me.emit(&format!("{xn} = fcmp uno {lt} {x}, {x}"));
                        me.emit(&format!("{bn} = fcmp uno {lt} {b}, {b}"));
                        me.emit(&format!("{nb} = xor i1 {eski}, true"));
                        me.emit(&format!("{ek} = and i1 {yeni}, {nb}"));
                        me.emit(&format!("{r} = or i1 {c}, {ek}"));
                        r
                    } else {
                        me.compare(if buyuk { crate::ast::BinOp::Gt } else { crate::ast::BinOp::Lt }, x, &b, &et)?
                    };
                    let s = me.tmp();
                    me.emit(&format!("{s} = select i1 {daha}, {lt} {x}, {lt} {b}"));
                    me.emit(&format!("store {lt} {s}, ptr {best}"));
                    Ok(())
                })?;
                let r = self.tmp();
                self.emit(&format!("{r} = load {lt}, ptr {best}"));
                if self.is_rc(&et) {
                    self.retain(&r, &et);
                    self.own(r.clone(), &et);
                }
                r
            }
        };
        if let Some((f, _)) = fv {
            self.drop_temp(&f);
        }
        self.drop_temp(&l);
        Ok(result)
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
            BM::TersCevir => self.emit(&format!("call void @kolay_liste_ters(ptr {c})")),
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
            BM::Bul | BM::SBul => {
                if m == BM::Bul {
                    let p = self.spill(&a[0], &args[0].ty);
                    self.emit(&format!("{t} = call i64 @kolay_liste_bul(ptr {v}, ptr {p})"));
                } else {
                    self.emit(&format!("{t} = call i64 @kolay_yazi_bul(ptr {v}, ptr {})", a[0]));
                }
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
            BM::LAdim => {
                let tip = self.tip(ret);
                self.emit(&format!("{t} = call ptr @kolay_liste_adim(ptr {v}, i64 {}, ptr {tip}, ptr {k}, ptr {kd})", a[0]));
                self.own(t, ret)
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
            BM::SIcerir | BM::Baslar | BM::Biter => {
                let f = match m {
                    BM::SIcerir => "kolay_icerir",
                    BM::Baslar => "kolay_baslar",
                    _ => "kolay_biter",
                };
                self.emit(&format!("{t} = call zeroext i1 @{f}(ptr {v}, ptr {})", a[0]));
                t
            }
            BM::Degistir => {
                self.emit(&format!("{t} = call ptr @kolay_degistir(ptr {v}, ptr {}, ptr {}, ptr {k}, ptr {kd})", a[0], a[1]));
                self.own(t, &Type::Str)
            }
            BM::Say => {
                self.emit(&format!("{t} = call i64 @kolay_say(ptr {v}, ptr {}, ptr {k}, ptr {kd})", a[0]));
                t
            }
            BM::Tekrar => {
                self.emit(&format!("{t} = call ptr @kolay_tekrar(ptr {v}, i64 {}, ptr {k}, ptr {kd})", a[0]));
                self.own(t, &Type::Str)
            }
            BM::Hassas => {
                self.emit(&format!("{t} = call ptr @kolay_hassas(double {v}, i64 {})", a[0]));
                self.own(t, &Type::Str)
            }
            BM::Grupla => {
                self.emit(&format!("{t} = call ptr @kolay_grupla(ptr {v})"));
                self.own(t, &Type::Str)
            }
            BM::Hizala => {
                self.emit(&format!("{t} = call ptr @kolay_hizala(ptr {v}, i64 {}, ptr {}, i64 {})", a[0], a[1], a[2]));
                self.own(t, &Type::Str)
            }
            BM::Birlesik => {
                self.emit(&format!("{t} = call ptr @kolay_birlesik(ptr {v}, ptr {})", a[0]));
                self.own(t, &Type::Str)
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
            // sonuç 'veya': sayı ya da Hata.Dönüşüm
            BM::Tama | BM::Ondaliga => {
                let _ = (k, kd, t);
                let ut = self.tip(ret);
                let a = self.alloca("%veya");
                let f = if m == BM::Tama { "kolay_yazi_tama" } else { "kolay_yazi_ondaliga" };
                self.emit(&format!("call void @{f}(ptr {v}, ptr {a}, ptr {ut})"));
                let r = self.load_mem(&a, ret);
                self.own(r, ret)
            }
            BM::Sirali | BM::Tersi => {
                let f = if m == BM::Sirali { "kolay_liste_sirali" } else { "kolay_liste_tersi" };
                self.emit(&format!("{t} = call ptr @{f}(ptr {v})"));
                self.own(t, ret)
            }
            _ => unreachable!("yerleşik metot {m:?}"),
        })
    }
}

