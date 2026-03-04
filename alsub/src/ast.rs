pub mod expr;
pub use expr::Expr;
pub use expr::InstantiateSourceKind;
pub use expr::SExpr;

use crate::spans::Span;
use crate::spans::SpanMaker;
use crate::spans::Spanned;

pub struct ParserContext<'input> {
    pub span_maker: SpanMaker<'input>,
}
pub type StringId = ustr::Ustr;

#[derive(Debug, Clone)]
pub enum Literal {
    Float,
    Int,
    Str,
}

#[derive(Debug, Clone)]
pub enum Op {
    Add,
    Sub,
    Mult,
    Div,
    Rem,

    Lt,
    Lte,
    Gt,
    Gte,

    Eq,
    Neq,
}

#[derive(Debug, Clone)]
pub enum RetType {
    Lit(Literal),
    Bool, // Variant type `t {} | `f {}
}
pub type OpType = (Option<Literal>, RetType);
pub const INT_OP: OpType = (Some(Literal::Int), RetType::Lit(Literal::Int));
pub const FLOAT_OP: OpType = (Some(Literal::Float), RetType::Lit(Literal::Float));
pub const STR_OP: OpType = (Some(Literal::Str), RetType::Lit(Literal::Str));
pub const INT_CMP: OpType = (Some(Literal::Int), RetType::Bool);
pub const FLOAT_CMP: OpType = (Some(Literal::Float), RetType::Bool);
pub const ANY_CMP: OpType = (None, RetType::Bool);

type LetDefinition = (LetPattern, Box<SExpr>);
pub type LetRecDefinition = (StringId, SExpr);

#[derive(Debug, Clone)]
pub enum LetPattern {
    Case(Spanned<StringId>, Box<LetPattern>),
    Record(Spanned<(Vec<TypeParam>, Vec<(Spanned<StringId>, Box<LetPattern>)>)>),
    Var((Option<StringId>, Span), Option<STypeExpr>),
}
impl LetPattern {
    /// `{}` — the unit pattern
    pub fn unit(span: Span) -> Self {
        LetPattern::Record(((vec![], vec![]), span))
    }

    /// Pattern for `true` → `` `t {} ``
    pub fn bool_true(span: Span) -> Self {
        LetPattern::Case((ustr::ustr("t"), span), Box::new(Self::unit(span)))
    }

    /// Pattern for `false` → `` `f {} ``
    pub fn bool_false(span: Span) -> Self {
        LetPattern::Case((ustr::ustr("f"), span), Box::new(Self::unit(span)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TypeParam {
    pub name: Spanned<StringId>,
    pub alias: Spanned<StringId>,
}
impl TypeParam {
    pub fn new(name: Spanned<StringId>, alias: Option<Spanned<StringId>>) -> Self {
        let alias = alias.unwrap_or(name);
        Self { name, alias }
    }
}

#[derive(Debug, Clone)]
pub enum FieldTypeDecl {
    Imm(STypeExpr),
    RWSame(STypeExpr),
    RWPair(STypeExpr, STypeExpr),
}
pub type KeyPairType = (Spanned<StringId>, FieldTypeDecl);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyKind {
    Universal,
    Existential,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JoinKind {
    Union,
    Intersect,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Bot,
    Case(Vec<(Spanned<StringId>, Box<STypeExpr>)>),
    Func(Box<STypeExpr>, Box<STypeExpr>),
    Hole,
    Ident(StringId),
    Poly(Vec<TypeParam>, Box<STypeExpr>, PolyKind),
    Record(Vec<KeyPairType>),
    RecursiveDef(StringId, Box<STypeExpr>),
    Top,
    VarJoin(JoinKind, Vec<STypeExpr>),
}
pub type STypeExpr = Spanned<TypeExpr>;

#[derive(Debug, Clone)]
pub enum Statement {
    Empty,
    Expr(SExpr),
    LetDef(LetDefinition),
    LetRecDef(Vec<LetRecDefinition>),
    Println(Vec<SExpr>),
}

fn enumerate_tuple_fields<T, R>(
    vals: impl IntoIterator<Item = (T, Span)>,
    mut make_field: impl FnMut(Spanned<StringId>, T) -> R,
) -> Vec<R> {
    vals.into_iter()
        .enumerate()
        .map(|(i, (val, span))| {
            let name = ustr::ustr(&format!("_{}", i));
            make_field((name, span), val)
        })
        .collect()
}

// TODO, cleanup
pub fn make_tuple_expr(mut vals: Vec<SExpr>) -> Expr {
    if vals.len() <= 1 {
        return vals.pop().unwrap().0;
    }

    // Tuple
    let fields = enumerate_tuple_fields(vals, |name, val| (name, Box::new((val, name.1)), false, None));
    expr::record(fields)
}

pub fn make_tuple_pattern(vals: Spanned<Vec<Spanned<LetPattern>>>) -> LetPattern {
    let (mut vals, full_span) = vals;
    if vals.len() <= 1 {
        return vals.pop().unwrap().0;
    }

    let fields = enumerate_tuple_fields(vals, |name, val| (name, Box::new(val)));
    LetPattern::Record(((vec![], fields), full_span))
}

pub fn make_tuple_type(mut vals: Vec<STypeExpr>) -> TypeExpr {
    if vals.len() <= 1 {
        return vals.pop().unwrap().0;
    }

    let fields = enumerate_tuple_fields(vals, |name, val| (name, FieldTypeDecl::Imm((val, name.1))));
    TypeExpr::Record(fields)
}

pub fn make_join_ast(kind: JoinKind, mut children: Vec<STypeExpr>) -> TypeExpr {
    if children.len() <= 1 {
        children.pop().unwrap().0
    } else {
        TypeExpr::VarJoin(kind, children)
    }
}
