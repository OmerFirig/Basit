// Referans sayımı: geçici değerlerin sahipliği, tutma ve bırakma (kurallar için bkz. mod.rs).

use super::duzen::{is_esnek, is_ptr_type};
use super::Gen;
use crate::types::Type;

impl<'p> Gen<'p> {
    /// Bu tipteki değerler referans sayımlı bir nesneye işaret edebilir mi?
    pub(super) fn is_rc(&self, t: &Type) -> bool {
        match t {
            Type::Union(ms) => ms.iter().any(|m| self.is_rc(m)),
            _ => is_ptr_type(t) || is_esnek(t),
        }
    }

    /// Yeni üretilen (sahipli) değer
    pub(super) fn own(&mut self, v: String, t: &Type) -> String {
        if self.is_rc(t) {
            self.f.owned.insert(v.clone());
            self.f.owned_tys.insert(v.clone(), t.clone());
        }
        v
    }

    /// Kullanılıp biten değer sahipli bir geçiciyse bırakılır.
    pub(super) fn drop_temp(&mut self, v: &str) {
        if self.f.owned.remove(v) {
            let t = self.f.owned_tys.remove(v).expect("tip");
            self.release(v, &t);
        }
    }

    /// Sahipli geçiciyi listeden çıkarır (sahiplik başka bir yere geçti). Sahipliyse true.
    pub(super) fn disown(&mut self, v: &str) -> bool {
        self.f.owned_tys.remove(v);
        self.f.owned.remove(v)
    }

    /// Değer bir yere saklanacak: sahipliyse sahiplik devredilir, ödünçse tutulur.
    pub(super) fn keep_ref(&mut self, v: &str, t: &Type) {
        if !self.disown(v) {
            self.retain(v, t);
        }
    }

    /// Değeri sahipli hale getirir (ödünçse tutar).
    pub(super) fn ensure_owned(&mut self, v: &str, t: &Type) {
        if !self.f.owned.contains(v) && self.is_rc(t) {
            self.retain(v, t);
            self.own(v.to_string(), t);
        }
    }

    pub(super) fn retain(&mut self, v: &str, t: &Type) {
        self.rc_op(v, t, true);
    }

    pub(super) fn release(&mut self, v: &str, t: &Type) {
        self.rc_op(v, t, false);
    }

    fn rc_op(&mut self, v: &str, t: &Type, tut: bool) {
        if is_ptr_type(t) {
            let f = if tut { "kolay_tut" } else { "kolay_birak" };
            self.emit(&format!("call void @{f}(ptr {v})"));
        } else if is_esnek(t) {
            // küçük değerler (tek sayılar) için çağrı yapılmaz
            let (b, z) = (self.tmp(), self.tmp());
            self.emit(&format!("{b} = and i64 {v}, 1"));
            self.emit(&format!("{z} = icmp eq i64 {b}, 0"));
            let (call, done) = (self.new_label(), self.new_label());
            self.br_cold(&z, &call, &done);
            self.block(&call);
            let f = if tut { "kolay_esnek_tut" } else { "kolay_esnek_birak" };
            self.emit(&format!("call void @{f}(i64 {v})"));
            self.block(&done);
        } else if let Type::Union(_) = t {
            if !self.is_rc(t) {
                return;
            }
            let tip = self.tip(t);
            let (tag, w0) = (self.tmp(), self.tmp());
            self.emit(&format!("{tag} = extractvalue %veya {v}, 0"));
            self.emit(&format!("{w0} = extractvalue %veya {v}, 1"));
            let f = if tut { "kolay_veya_tut" } else { "kolay_veya_birak" };
            self.emit(&format!("call void @{f}(ptr {tip}, i64 {tag}, i64 {w0})"));
        }
    }

    /// Fonksiyondan çıkarken yerel değişkenleri bırakır.
    pub(super) fn release_rc_slots(&mut self) {
        for (p, t) in self.f.rc_slots.clone() {
            let ll = self.llty(&t);
            let v = self.tmp();
            self.emit(&format!("{v} = load {ll}, ptr {p}"));
            self.release(&v, &t);
        }
        if self.f.dene_modu {
            // temizlik kaydı zincirden çıkarılır (bkz. mod.rs temizlik_kaydi; değişkenler sonradan da
            // eklenebildiği için, değişkeni olmayan kopyada da kayıt tutulur)
            self.emit("store ptr %tz.u, ptr @kolay_temizlik_ust");
        }
    }

    /// Döngü boyunca yaşaması gereken gizli değişken (örneğin üzerinde dönülen liste).
    /// Önceki değeri bırakılır, yenisi saklanır; fonksiyon dönerken de bırakılır.
    pub(super) fn hidden_slot(&mut self, v: &str, t: &Type) -> String {
        let ll = self.llty(t);
        let a = self.alloca(&ll);
        let z = self.zero_value(t);
        // alloca girişte sıfırlanır (fonksiyon başında)
        self.f.allocas.push_str(&format!("  store {ll} {z}, ptr {a}\n"));
        self.f.rc_slots.push((a.clone(), t.clone()));
        self.keep_ref(v, t);
        let old = self.tmp();
        self.emit(&format!("{old} = load {ll}, ptr {a}"));
        self.emit(&format!("store {ll} {v}, ptr {a}"));
        self.release(&old, t);
        a
    }
}
