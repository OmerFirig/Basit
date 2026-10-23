// Parser'ın ürettiği sözdizimi ağacı.

use crate::diag::Span;
use num_bigint::BigInt;

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "ve",
            BinOp::Or => "veya",
        }
    }
    pub fn is_arith(self) -> bool {
        matches!(self, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem)
    }
}

#[derive(Debug, Clone)]
pub enum FPartE {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct Arg {
    pub name: Option<(String, Span)>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(BigInt),
    Float(f64),
    Str(String),
    FStr(Vec<FPartE>),
    Bool(bool),
    Hic,
    Bu,
    Ident(String),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Range { start: Option<Box<Expr>>, end: Option<Box<Expr>>, inclusive: bool },
    Call { callee: Box<Expr>, args: Vec<Arg> },
    Field(Box<Expr>, String, Span),
    Index(Box<Expr>, Box<Expr>),
    Olarak(Box<Expr>, TypeExpr),
    Match(Box<Match>),
}

#[derive(Debug, Clone)]
pub enum TypeArg {
    Num(u32),
    Plus,
    Minus,
}

#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Path {
        path: Vec<String>,
        generics: Vec<TypeExpr>,
        args: Option<Vec<TypeArg>>,
        dims: Vec<Option<usize>>,
    },
    Union(Vec<TypeExpr>),
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Int(BigInt, Span),
    Float(f64, Span),
    Str(String, Span),
    Bool(bool, Span),
    Hic(Span),
    Wild,
    Range(BigInt, BigInt, bool, Span),
    Path { path: Vec<String>, binds: Option<Vec<Option<(String, Span)>>>, span: Span },
}

#[derive(Debug, Clone)]
pub enum ArmBody {
    Expr(Expr),
    Block(Block),
    Stmt(Box<Stmt>),
}

#[derive(Debug, Clone)]
pub struct Arm {
    /// None: `değilse` dalı
    pub pats: Option<Vec<Pattern>>,
    pub guard: Option<Expr>,
    pub body: ArmBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Match {
    pub scrutinee: Expr,
    pub arms: Vec<Arm>,
    pub span: Span,
}

pub type Block = Vec<Stmt>;

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub sabit: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Yeniden {
    Yok,
    Genel,
    Ebeveyn(String, Span),
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub span: Span,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: Block,
    pub yeniden: Yeniden,
}

/// Dış (C) fonksiyon imzası. `symbol`: C'deki adı (= "sqrt"), yoksa Kolay'daki adıyla aynı.
#[derive(Debug, Clone)]
pub struct DisFn {
    pub name: String,
    pub span: Span,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub name: String,
    pub span: Span,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct TurDecl {
    pub name: String,
    pub span: Span,
    pub parents: Vec<(Vec<String>, Span)>,
    pub fields: Vec<Param>,
    pub methods: Vec<FnDecl>,
}

#[derive(Debug, Clone)]
pub struct ArayuzDecl {
    pub name: String,
    pub span: Span,
    pub sigs: Vec<FnSig>,
}

#[derive(Debug, Clone)]
pub struct VariantDecl {
    pub name: String,
    pub span: Span,
    pub fields: Vec<Param>,
}

#[derive(Debug, Clone)]
pub struct SecenekDecl {
    pub name: String,
    pub span: Span,
    pub interfaces: Vec<(Vec<String>, Span)>,
    pub variants: Vec<VariantDecl>,
    pub methods: Vec<FnDecl>,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Ata {
        sabit: bool,
        explicit_ata: bool,
        name: String,
        name_span: Span,
        ty: Option<TypeExpr>,
        value: Option<Expr>,
    },
    Assign { target: Expr, op: Option<BinOp>, value: Expr },
    Expr(Expr),
    Eger { branches: Vec<(Expr, Block)>, else_: Option<Block> },
    Kere { count: Expr, body: Block },
    Her { iter: Expr, pat: Vec<(String, Span)>, body: Block },
    Surece { cond: Expr, body: Block },
    CFor { init: Box<Stmt>, cond: Expr, step: Box<Stmt>, body: Block },
    Dur,
    Devam,
    Dondur(Option<Expr>),
    Match(Match),
    Fonksiyon(FnDecl),
    Tur(TurDecl),
    Arayuz(ArayuzDecl),
    Secenek(SecenekDecl),
    Dahil { path: String, alias: Option<(String, Span)> },
    /// C kütüphanesinden fonksiyonlar (#Güvensiz ile)
    Dis { alias: Option<(String, Span)>, lib: String, lib_span: Span, funcs: Vec<DisFn> },
}
