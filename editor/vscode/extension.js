// Kolay VS Code eklentisi: tamamlama, hata gösterme (kolay denetle) ve çalıştırma komutu.
const vscode = require("vscode");
const cp = require("child_process");
const path = require("path");

const KEYWORDS = [
  "ata", "sabit", "fonksiyon", "döndür", "eğer", "değilse", "yap", "içindeki", "her", "için",
  "sürece", "kere", "dur", "devam", "doğru", "yanlış", "tür", "yeniden", "bu", "arayüz",
  "seçenek", "eşleştir", "veya", "ve", "değil", "dahil et", "olarak", "Hiç", "adıyla",
  "dış kütüphane", "kütüphanesindeki", "dene", "yakala", "fırlat",
];
const TYPES = ["Tam", "Ondalık", "Yazı", "Mantık", "Liste", "Sözlük", "Fonksiyon", "Hata"];
const FUNCS = [
  ["yazdır", "yazdır(${1})", "Ekrana yazar"],
  ["oku", "oku(\"${1:Soru: }\")", "Klavyeden bir satır okur (Yazı)"],
];
const METHODS = {
  liste: [
    ["ekle", "değer", "Sona eleman ekler"],
    ["araya_ekle", "konum, değer", "Verilen konuma eleman ekler"],
    ["sil", "konum", "Konumdaki elemanı siler"],
    ["çıkar", "", "Son elemanı çıkarıp döndürür"],
    ["uzunluk", "", "Eleman sayısı"],
    ["içerir", "değer", "Eleman listede var mı?"],
    ["bul", "değer", "Elemanın konumu, yoksa Hiç"],
    ["sırala", "", "Küçükten büyüğe sıralar (listeyi değiştirir)"],
    ["sıralı", "", "Sıralanmış yeni liste"],
    ["ters_çevir", "", "Sırayı ters çevirir (listeyi değiştirir)"],
    ["tersi", "", "Ters sıralı yeni liste"],
    ["temizle", "", "Tüm elemanları siler"],
    ["adım", "n", "Her n. eleman (yeni liste); eksi n sondan başlar"],
    ["dönüşüm", "her x için x", "Her elemana fonksiyonu uygular (yeni liste)"],
    ["filtre", "her x için x > 0", "Koşula uyan elemanlar (yeni liste)"],
    ["hepsi", "her x için x > 0", "Bütün elemanlar koşula uyuyor mu?"],
    ["herhangi", "her x için x > 0", "Koşula uyan bir eleman var mı?"],
    ["toplam", "", "Elemanların toplamı"],
    ["en_büyük", "", "En büyük eleman"],
    ["en_küçük", "", "En küçük eleman"],
    ["yazıya", "", "Yazıya çevirir"],
  ],
  sözlük: [
    ["uzunluk", "", "Anahtar sayısı"],
    ["içerir", "anahtar", "Anahtar var mı?"],
    ["sil", "anahtar", "Anahtarı siler"],
    ["anahtarlar", "", "Anahtarların listesi"],
    ["değerler", "", "Değerlerin listesi"],
    ["temizle", "", "Tümünü siler"],
    ["yazıya", "", "Yazıya çevirir"],
  ],
  yazı: [
    ["uzunluk", "", "Karakter sayısı"],
    ["büyük_harf", "", "BÜYÜK HARFE çevirir (Türkçe kurallarla)"],
    ["küçük_harf", "", "küçük harfe çevirir (Türkçe kurallarla)"],
    ["karakter", "konum", "Konumdaki karakter"],
    ["içerir", "parça", "Parçayı içeriyor mu?"],
    ["böl", "ayraç", "Ayraçtan bölüp liste yapar"],
    ["kırp", "", "Baştaki ve sondaki boşlukları siler"],
    ["tama", "", "Sayıya çevirir (Tam)"],
    ["ondalığa", "", "Sayıya çevirir (Ondalık)"],
    ["yazıya", "", "Yazıya çevirir"],
  ],
  ondalık: [
    ["tama", "", "Tam sayıya çevirir"],
    ["yazıya", "", "Yazıya çevirir"],
  ],
  tam: [["yazıya", "", "Yazıya çevirir"]],
};

// us klavye modu: Türkçe harfler İngilizce karşılıklarına
const US = { ç: "c", ğ: "g", ı: "i", ö: "o", ş: "s", ü: "u", Ç: "C", Ğ: "G", İ: "I", Ö: "O", Ş: "S", Ü: "U" };
const toUs = (s) => s.replace(/[çğıöşüÇĞİÖŞÜ]/g, (c) => US[c]);
const isUs = (text) => /^\s*klavye\s*\(\s*"us"\s*\)/m.test(text);

const L = "\\p{L}\\p{N}_";
const ID = `[\\p{L}_][${L}]*`;

/** Belgedeki tanımları bulur: değişkenler, fonksiyonlar, türler, seçenekler. */
function scan(text) {
  const vars = new Map(); // ad -> tip ipucu
  const funcs = new Map(); // ad -> parametreler
  const types = new Map(); // ad -> {parents, fields, methods}
  const enums = new Map(); // ad -> varyantlar
  const ifaces = new Set();
  const modules = new Set();
  const tparams = new Set(); // generic tip parametreleri: <T, Eleman>
  const clean = text.replace(/\/<[\s\S]*?>\//g, "").replace(/\/\/.*$/gm, "");

  for (const m of clean.matchAll(new RegExp(`ata\\s+(?:sabit\\s+)?(${ID})\\s*(?::\\s*([^=\\n]+))?(?:=\\s*(.*))?`, "gu"))) {
    vars.set(m[1], (m[2] || m[3] || "").trim());
  }
  for (const m of clean.matchAll(new RegExp(`her\\s+(${ID})(?:\\s*,\\s*(${ID}))?\\s+(?:için|icin)`, "gu"))) {
    vars.set(m[1], vars.get(m[1]) || "");
    if (m[2]) vars.set(m[2], vars.get(m[2]) || "");
  }
  for (const m of clean.matchAll(new RegExp(`(${ID})\\s+(?:adıyla|adiyla)`, "gu"))) modules.add(m[1]);

  // tür blokları: tür Ad<T>[Ebeveyn](alanlar) { ... }
  for (const m of clean.matchAll(new RegExp(`(?:tür|tur)\\s+(${ID})`, "gu"))) {
    let i = m.index + m[0].length;
    const skip = () => { while (/\s/.test(clean[i] || "")) i++; };
    skip();
    let parents = [], params = "", generics = "";
    if (clean[i] === "<") {
      const e = clean.indexOf(">", i);
      generics = clean.slice(i + 1, e);
      for (const g of splitTop(generics)) if (g.trim()) tparams.add(g.trim());
      i = e + 1;
      skip();
    }
    if (clean[i] === "[") {
      const e = clean.indexOf("]", i);
      parents = splitTop(clean.slice(i + 1, e));
      i = e + 1;
      skip();
    }
    if (clean[i] === "(") {
      params = paren(clean, i);
      i += params.length + 2;
      skip();
    }
    // gövdesiz tür: tür Nokta(x: Tam)
    const body = clean[i] === "{" ? block(clean, i + 1) : "";
    const fieldTypes = new Map();
    for (const f of splitTop(params)) {
      const [n, ...t] = f.replace(/^\s*(sabit|ata)\s+/, "").split(":");
      if (n.trim()) fieldTypes.set(n.trim(), t.join(":").split("=")[0].trim());
    }
    const fields = [...fieldTypes.keys()];
    const methods = [];
    for (const x of body.matchAll(new RegExp(`(?:fonksiyon|yeniden)\\s+(${ID})\\s*\\(`, "gu"))) {
      methods.push([x[1], paren(body, x.index + x[0].length - 1)]);
    }
    types.set(m[1], { parents, fields, fieldTypes, methods, params, generics });
  }
  for (const m of clean.matchAll(new RegExp(`(?:seçenek|secenek)\\s+(${ID})\\s*\\{`, "gu"))) {
    const body = block(clean, m.index + m[0].length);
    enums.set(m[1], [...body.matchAll(new RegExp(`^\\s*(${ID})`, "gmu"))].map((x) => x[1]));
  }
  for (const m of clean.matchAll(new RegExp(`(?:arayüz|arayuz)\\s+(${ID})`, "gu"))) ifaces.add(m[1]);
  // tür ve arayüz gövdelerindekiler hariç, üst düzey ve dış kütüphane fonksiyonları
  for (const m of clean.matchAll(new RegExp(`fonksiyon\\s+(${ID})\\s*(?:<([^>]*)>)?\\s*\\(`, "gu"))) {
    if (m[2]) for (const g of splitTop(m[2])) if (g.trim()) tparams.add(g.trim());
    const ps = paren(clean, m.index + m[0].length - 1);
    if (!funcs.has(m[1]) && !enclosingType(clean, m.index, true)) funcs.set(m[1], ps);
    for (const p of splitTop(ps)) {
      const [n, t] = p.split(":");
      if (n && n.trim() && new RegExp(`^${ID}$`, "u").test(n.trim())) vars.set(n.trim(), (t || "").trim());
    }
  }
  return { vars, funcs, types, enums, ifaces, modules, tparams };
}

/** İmlecin içinde bulunduğu türün adı */
function enclosingType(text, offset, iface = false) {
  let found = null;
  const kw = iface ? "tür|tur|arayüz|arayuz" : "tür|tur";
  for (const m of text.slice(0, offset).matchAll(new RegExp(`(?:${kw})\\s+(${ID})[^{]*\\{`, "gu"))) {
    const body = block(text, m.index + m[0].length);
    if (m.index + m[0].length + body.length >= offset) found = m[1];
  }
  return found;
}

/** `(` konumundan eşleşen `)` öncesine kadar */
function paren(text, open) {
  let d = 0;
  for (let i = open; i < text.length; i++) {
    if (text[i] === "(") d++;
    else if (text[i] === ")" && --d === 0) return text.slice(open + 1, i);
  }
  return text.slice(open + 1);
}

/** Üst düzey virgüllerden böler: "a: Tam(8, +), b" -> ["a: Tam(8, +)", "b"] */
function splitTop(s) {
  const out = [];
  let d = 0, cur = "";
  for (const c of s) {
    if ("([<{".includes(c)) d++;
    else if (")]>}".includes(c)) d--;
    if (c === "," && d === 0) (out.push(cur.trim()), (cur = ""));
    else cur += c;
  }
  if (cur.trim()) out.push(cur.trim());
  return out;
}

/** `{` sonrasından eşleşen `}` öncesine kadar */
function block(text, start) {
  let d = 1;
  for (let i = start; i < text.length; i++) {
    if (text[i] === "{") d++;
    else if (text[i] === "}" && --d === 0) return text.slice(start, i);
  }
  return text.slice(start);
}

/** Türün (ya da ebeveynlerinin) alanının tipi */
function fieldType(info, name, field, seen = new Set()) {
  const t = info.types.get(name);
  if (!t || seen.has(name)) return null;
  seen.add(name);
  if (t.fieldTypes.has(field)) return t.fieldTypes.get(field);
  for (const p of t.parents) {
    const r = fieldType(info, p, field, seen);
    if (r) return r;
  }
  return null;
}

/** Tür ve ebeveynlerinin alan ve metotları */
function members(info, name, seen = new Set()) {
  const t = info.types.get(name);
  if (!t || seen.has(name)) return { fields: [], methods: [] };
  seen.add(name);
  const out = { fields: [...t.fields], methods: [...t.methods] };
  for (const p of t.parents) {
    const pm = members(info, p, seen);
    out.fields.push(...pm.fields);
    for (const pmm of pm.methods) if (!out.methods.some((x) => x[0] === pmm[0])) out.methods.push(pmm);
  }
  return out;
}

/** Tip ipucundan (tip yazısı ya da ilk değer) yerleşik tip sınıfı ya da tür adı */
function kindOf(hint, info) {
  if (!hint) return null;
  const h = hint.trim();
  if (/^Liste\s*</.test(h) || /^\[/.test(h) || /^-?\d[\d_]*\s*\.\.=?/.test(h)) return "liste";
  if (/^Sözlük\s*</.test(h) || /^\{/.test(h)) return "sözlük";
  if (/^(Yazı|f?")/.test(h) || /^oku\s*\(/.test(h)) return "yazı";
  if (/^Ondalık/.test(h) || /^-?\d[\d_]*\.\d/.test(h)) return "ondalık";
  if (/^Tam/.test(h) || /^-?(0x)?\d/.test(h)) return "tam";
  const m = h.match(new RegExp(`^(${ID})`, "u"));
  if (m && (info.types.has(m[1]) || info.enums.has(m[1]))) return m[1];
  return null;
}

function item(label, kind, insert, detail, us) {
  const shown = us ? toUs(label) : label;
  const it = new vscode.CompletionItem(shown, kind);
  if (insert) it.insertText = new vscode.SnippetString(us ? toUs(insert) : insert);
  if (detail) it.detail = detail;
  return it;
}

function paramSnippet(name, params) {
  const names = splitTop(params).map((p) => p.split(":")[0].replace(/^\s*sabit\s+/, "").trim()).filter(Boolean);
  if (!names.length) return `${name}()`;
  return `${name}(${names.map((n, i) => `\${${i + 1}:${n}}`).join(", ")})`;
}

const K = vscode.CompletionItemKind;

function complete(doc, pos) {
  const text = doc.getText();
  const us = isUs(text);
  const info = scan(text);
  const before = doc.lineAt(pos.line).text.slice(0, pos.character);

  // nokta sonrası: metotlar, alanlar, seçenek varyantları, modül üyeleri
  const dot = before.match(new RegExp(`(${ID}(?:\\s*\\.\\s*${ID})*|\\)|\\]|")\\s*\\.\\s*[${L}]*$`, "u"));
  if (dot) {
    const chain = dot[1].split(".").map((x) => x.trim());
    const recv = chain[0];
    const out = [];
    const addBuiltin = (k) => {
      for (const [n, p, d] of METHODS[k] || []) {
        const args = p ? p.split(",").map((a, i) => `\${${i + 1}:${a.trim()}}`).join(", ") : "";
        out.push(item(n, K.Method, `${n}(${args})`, d, us));
      }
    };
    if (recv === "Hata") {
      for (const h of ["SıfıraBölme", "İndeks", "Anahtar", "Taşma", "Dönüşüm"]) out.push(item(h, K.Class, null, "hazır hata türü", us));
      return out;
    }
    if (info.enums.has(recv)) {
      for (const v of info.enums.get(recv)) out.push(item(v, K.EnumMember, null, recv, false));
      return out;
    }
    let k = recv === '"' ? "yazı" : recv === "]" ? "liste" : kindOf(info.vars.get(recv), info);
    if (info.types.has(recv)) k = recv; // Ebeveyn.metot()
    if (recv === "bu") k = enclosingType(text, doc.offsetAt(pos));
    else if (!info.vars.has(recv) && !info.types.has(recv)) {
      // tür içinde alan adı: ad.
      const cur = enclosingType(text, doc.offsetAt(pos));
      if (cur) k = kindOf(fieldType(info, cur, recv), info);
    }
    // zincir: ali.notlar.
    for (const f of chain.slice(1)) k = k && info.types.has(k) ? kindOf(fieldType(info, k, f), info) : null;
    if (k && info.types.has(k)) {
      const m = members(info, k);
      for (const f of m.fields) out.push(item(f, K.Field, null, k, false));
      for (const [n, p] of m.methods) out.push(item(n, K.Method, paramSnippet(n, p), `${k} metodu`, false));
      addBuiltin("tam");
      return out;
    }
    if (k) {
      addBuiltin(k);
      return out;
    }
    // tip bilinmiyor: hepsini öner
    const seen = new Set();
    for (const key of Object.keys(METHODS)) {
      for (const [n, p, d] of METHODS[key]) {
        if (seen.has(n)) continue;
        seen.add(n);
        const args = p ? p.split(",").map((a, i) => `\${${i + 1}:${a.trim()}}`).join(", ") : "";
        out.push(item(n, K.Method, `${n}(${args})`, d, us));
      }
    }
    for (const t of info.types.keys()) {
      const m = members(info, t);
      for (const f of m.fields) if (!seen.has(f)) (seen.add(f), out.push(item(f, K.Field, null, t, false)));
      for (const [n, p] of m.methods) if (!seen.has(n)) (seen.add(n), out.push(item(n, K.Method, paramSnippet(n, p), `${t} metodu`, false)));
    }
    return out;
  }

  const out = [];
  for (const kw of KEYWORDS) out.push(item(kw, K.Keyword, null, "anahtar kelime", us));
  for (const t of TYPES) out.push(item(t, K.Class, null, "hazır tip", us));
  for (const [n, s, d] of FUNCS) out.push(item(n, K.Function, s, d, us));
  const cur = enclosingType(text, doc.offsetAt(pos));
  if (cur && info.types.has(cur)) {
    const m = members(info, cur);
    for (const f of m.fields) out.push(item(f, K.Field, null, `${cur} alanı`, false));
    for (const [n, p] of m.methods) out.push(item(n, K.Method, paramSnippet(n, p), `${cur} metodu`, false));
  }
  for (const [v, hint] of info.vars) out.push(item(v, K.Variable, null, hint || "değişken", false));
  for (const [f, p] of info.funcs) out.push(item(f, K.Function, paramSnippet(f, p), `fonksiyon(${p})`, false));
  for (const [t, ti] of info.types) {
    const it = item(t, K.Class, ti.generics ? `${t}<\${1:Tam}>` : null, `tür${ti.generics ? `<${ti.generics}>` : ""} (${ti.params})`, false);
    out.push(it);
    // tür oluşturma: Ad(alan: , alan: )
    const fields = members(info, t).fields;
    const ctor = new vscode.CompletionItem(`${t}(...)`, K.Constructor);
    ctor.insertText = new vscode.SnippetString(`${t}(${fields.map((f, i) => `${f}: \${${i + 1}}`).join(", ")})`);
    ctor.detail = "yeni nesne";
    ctor.filterText = t;
    out.push(ctor);
  }
  for (const g of info.tparams) out.push(item(g, K.TypeParameter, null, "tip parametresi", false));
  for (const e of info.enums.keys()) out.push(item(e, K.Enum, null, "seçenek", false));
  for (const a of info.ifaces) out.push(item(a, K.Interface, null, "arayüz", false));
  for (const m of info.modules) out.push(item(m, K.Module, null, "dahil edilen dosya", false));
  return out;
}

// ---- hata gösterme: kaydedince ve açınca `kolay denetle` ----

function kolayPath() {
  return vscode.workspace.getConfiguration("kolay").get("yol") || "kolay";
}

function check(doc, diags) {
  if (doc.languageId !== "kolay" || doc.uri.scheme !== "file") return;
  const file = doc.uri.fsPath;
  cp.execFile(kolayPath(), ["denetle", file], { cwd: path.dirname(file), timeout: 15000 }, (err, stdout, stderr) => {
    if (err && err.code === "ENOENT") {
      diags.set(doc.uri, []);
      return;
    }
    diags.set(doc.uri, parse(stdout + stderr, doc));
  });
}

function parse(out, doc) {
  const lines = out.split("\n");
  const list = [];
  const base = path.basename(doc.uri.fsPath);
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(/^HATA \((.+?)(?:, satır (\d+))?\): (.*)$/);
    if (!m) continue;
    let msg = m[3];
    // mesaj birkaç satır sürebilir: kaynak satırına ("  N | ") kadar ekle
    let j = i + 1;
    while (j < lines.length && lines[j].trim() && !/^\s*\d+ \|/.test(lines[j]) && !/^HATA /.test(lines[j])) msg += "\n" + lines[j++];
    let line = m[2] ? Math.max(0, parseInt(m[2]) - 1) : 0;
    let start = 0, end = 0;
    if (m[1] !== base) {
      msg = `${m[1]}${m[2] ? `, satır ${m[2]}` : ""}: ${msg}`;
      line = 0;
    } else if (line < doc.lineCount) {
      end = doc.lineAt(line).text.length;
      start = doc.lineAt(line).firstNonWhitespaceCharacterIndex;
      // işaret satırı: "    |     ^^^"
      const caret = lines[j + 1] || "";
      const bar = caret.indexOf("|");
      const hat = caret.indexOf("^");
      if (/^\s*\d+ \|/.test(lines[j] || "") && bar >= 0 && hat > bar) {
        start = hat - bar - 2;
        end = start + (caret.match(/\^+/)[0].length);
      }
    }
    line = Math.min(line, doc.lineCount - 1);
    const d = new vscode.Diagnostic(new vscode.Range(line, start, line, Math.max(end, start + 1)), msg, vscode.DiagnosticSeverity.Error);
    d.source = "kolay";
    list.push(d);
  }
  return list;
}

function activate(ctx) {
  const sel = { language: "kolay" };
  ctx.subscriptions.push(
    vscode.languages.registerCompletionItemProvider(sel, { provideCompletionItems: complete }, ".")
  );

  const diags = vscode.languages.createDiagnosticCollection("kolay");
  ctx.subscriptions.push(diags);
  ctx.subscriptions.push(vscode.workspace.onDidSaveTextDocument((d) => check(d, diags)));
  ctx.subscriptions.push(vscode.workspace.onDidOpenTextDocument((d) => check(d, diags)));
  ctx.subscriptions.push(vscode.workspace.onDidCloseTextDocument((d) => diags.delete(d.uri)));
  vscode.workspace.textDocuments.forEach((d) => check(d, diags));

  const run = (cmd) => async () => {
    const ed = vscode.window.activeTextEditor;
    if (!ed || ed.document.languageId !== "kolay") return;
    await ed.document.save();
    const term = vscode.window.terminals.find((t) => t.name === "Kolay") || vscode.window.createTerminal("Kolay");
    term.show();
    const f = ed.document.uri.fsPath.replace(/'/g, "'\\''");
    term.sendText(cmd === "derle" ? `${kolayPath()} derle '${f}'` : `${kolayPath()} '${f}'`);
  };
  ctx.subscriptions.push(vscode.commands.registerCommand("kolay.calistir", run("çalıştır")));
  ctx.subscriptions.push(vscode.commands.registerCommand("kolay.derle", run("derle")));
}

function deactivate() {}

module.exports = { activate, deactivate };
