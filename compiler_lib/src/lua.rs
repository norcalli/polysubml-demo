#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
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

/////////////////////////////////////////////////////////////////////////////////////////////
pub fn assign(lhs: Expr, rhs: Expr, keep_if_unused: bool) -> Expr {
    Expr(Expr2::Assignment(lhs.0.into(), rhs.0.into(), keep_if_unused))
}
pub fn binop(lhs: Expr, rhs: Expr, op: Op) -> Expr {
    Expr(Expr2::BinOp(lhs.0.into(), rhs.0.into(), op))
}
pub fn floor_div(lhs: Expr, rhs: Expr) -> Expr {
    Expr(Expr2::FloorDiv(lhs.0.into(), rhs.0.into()))
}
pub fn call(lhs: Expr, rhs: Expr) -> Expr {
    Expr(Expr2::Call(lhs.0.into(), rhs.0.into()))
}
pub fn unary_minus(rhs: Expr) -> Expr {
    Expr(Expr2::Minus(rhs.0.into()))
}
pub fn nil() -> Expr {
    Expr(Expr2::Nil)
}
pub fn eqop(lhs: Expr, rhs: Expr) -> Expr {
    Expr(Expr2::BinOp(lhs.0.into(), rhs.0.into(), Op::Eq))
}
pub fn field(lhs: Expr, rhs: impl Into<String>) -> Expr {
    Expr(Expr2::Field(lhs.0.into(), rhs.into()))
}
pub fn scope_field(scope_var: &str, name: &str) -> Expr {
    Expr(Expr2::ScopeField(scope_var.to_string(), name.to_string()))
}
pub fn lit(code: impl Into<String>) -> Expr {
    Expr(Expr2::Literal(code.into()))
}
pub fn ternary(cond: Expr, e1: Expr, e2: Expr) -> Expr {
    Expr(Expr2::IfElse(cond.0.into(), e1.0.into(), e2.0.into()))
}
pub fn var(s: impl Into<String>) -> Expr {
    Expr(Expr2::Var(s.into()))
}

pub fn comma_list(exprs: Vec<Expr>) -> Expr {
    let mut flattened = Vec::new();
    for expr in exprs {
        match expr.0 {
            Expr2::Do(nested) => {
                flattened.extend(nested);
            }
            _ => {
                flattened.push(expr.0);
            }
        }
    }

    Expr(do_block_sub(flattened))
}
fn do_block_sub(flattened: Vec<Expr2>) -> Expr2 {
    match flattened.len() {
        0 => Expr2::Nil,
        1 => flattened.into_iter().next().unwrap(),
        _ => Expr2::Do(flattened),
    }
}

pub fn println(exprs: Vec<Expr>) -> Expr {
    Expr(Expr2::Print(exprs.into_iter().map(|e| e.0).collect()))
}

pub fn func(arg: Expr, scope: String, body: Expr) -> Expr {
    Expr(Expr2::Func(Box::new(arg.0), scope, Box::new(body.0)))
}

pub fn obj(fields: Vec<(String, Expr)>) -> Expr {
    Expr(Expr2::Table(
        fields
            .into_iter()
            .map(|(name, v)| (name, Box::new(v.0)))
            .collect(),
    ))
}

#[derive(Clone, Debug)]
pub struct Expr(Expr2);
impl Expr {
    pub fn to_source(mut self) -> String {
        self.0.add_parens();

        let mut s = String::new();
        self.0.write(&mut s);
        s
    }

    pub fn should_inline(&self) -> bool {
        self.0.should_inline()
    }
}

// Lua 5.1 precedence (lowest to highest)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    PRIMARY = 0,
    MEMBER,
    CALL,
    UNARY,
    MULTIPLICATIVE,
    ADDITIVE,
    RELATIONAL,
    EQUALITY,
    AND,
    OR,
    ASSIGN,
    EXPR,
}

#[derive(Clone, Debug)]
enum Expr2 {
    Paren(Box<Expr2>),
    Literal(String),
    Table(Vec<(String, Box<Expr2>)>),

    Var(String),

    // lhs["field"] — bracket indexing for user fields
    Field(Box<Expr2>, String),
    // scope.name — dot indexing for generated scope vars
    ScopeField(String, String),

    Call(Box<Expr2>, Box<Expr2>),

    Minus(Box<Expr2>),
    Nil,

    BinOp(Box<Expr2>, Box<Expr2>, Op),
    // math.floor(lhs / rhs)
    FloorDiv(Box<Expr2>, Box<Expr2>),

    // (function() if c then return a else return b end end)()
    IfElse(Box<Expr2>, Box<Expr2>, Box<Expr2>),

    // lhs = rhs; keep_if_unused flag for letrec
    Assignment(Box<Expr2>, Box<Expr2>, bool),
    // function(arg) local scope = {} return body end
    Func(Box<Expr2>, String, Box<Expr2>),

    // (function() stmt1; stmt2; return last end)()
    Do(Vec<Expr2>),

    // _println(args)
    Print(Vec<Expr2>),
}
impl Expr2 {
    fn precedence(&self) -> Precedence {
        use Expr2::*;
        use Op::*;
        use Precedence::*;
        match self {
            Paren(..) => PRIMARY,
            Literal(..) => PRIMARY,
            Table(..) => PRIMARY,
            Var(..) => PRIMARY,
            Nil => PRIMARY,
            Field(..) => MEMBER,
            ScopeField(..) => MEMBER,
            Call(..) | FloorDiv(..) => CALL,
            Minus(..) => UNARY,
            BinOp(_, _, op) => match op {
                Mult | Div | Rem => MULTIPLICATIVE,
                Add | Sub => ADDITIVE,
                Lt | Lte | Gt | Gte => RELATIONAL,
                Eq | Neq => EQUALITY,
            },
            IfElse(..) => CALL, // it's an IIFE call
            Assignment(..) => ASSIGN,
            Func(..) => PRIMARY,
            Do(..) => CALL, // it's an IIFE call
            Print(..) => CALL,
        }
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Paren(e) => {
                *out += "(";
                e.write(out);
                *out += ")";
            }
            Self::Literal(code) => {
                *out += code;
            }
            Self::Table(fields) => {
                *out += "{";
                let mut cw = CommaListWrite::new(out);
                for (name, val) in fields {
                    cw.write(|out| {
                        *out += "[\"";
                        *out += name;
                        *out += "\"] = ";
                        val.write(out);
                    });
                }
                *out += "}";
            }
            Self::Var(name) => {
                *out += name;
            }
            Self::Field(lhs, rhs) => {
                lhs.write(out);
                *out += "[\"";
                *out += rhs;
                *out += "\"]";
            }
            Self::ScopeField(s1, s2) => {
                *out += s1;
                *out += ".";
                *out += s2;
            }
            Self::Call(lhs, rhs) => {
                lhs.write(out);
                *out += "(";
                rhs.write(out);
                *out += ")";
            }
            Self::Minus(e) => {
                *out += "-";
                e.write(out);
            }
            Self::Nil => {
                *out += "nil";
            }
            Self::BinOp(lhs, rhs, op) => {
                use Op::*;
                let opstr = match op {
                    Add => " + ",
                    Sub => " - ",
                    Mult => " * ",
                    Div => " / ",
                    Rem => " % ",

                    Lt => " < ",
                    Lte => " <= ",
                    Gt => " > ",
                    Gte => " >= ",

                    Eq => " == ",
                    Neq => " ~= ",
                };

                lhs.write(out);
                *out += opstr;
                rhs.write(out);
            }
            Self::FloorDiv(lhs, rhs) => {
                *out += "math.floor(";
                lhs.write(out);
                *out += " / ";
                rhs.write(out);
                *out += ")";
            }
            Self::IfElse(cond, e1, e2) => {
                *out += "(function() if ";
                cond.write(out);
                *out += " then return ";
                e1.write(out);
                *out += " else return ";
                e2.write(out);
                *out += " end end)()";
            }
            Self::Assignment(lhs, rhs, _) => {
                lhs.write(out);
                *out += " = ";
                rhs.write(out);
            }
            Self::Func(arg, scope_name, body) => {
                *out += "function(";
                arg.write(out);
                *out += ") local ";
                *out += scope_name;
                *out += " = {} return ";
                body.write(out);
                *out += " end";
            }
            Self::Do(exprs) => {
                *out += "(function() ";
                for (i, ex) in exprs.iter().enumerate() {
                    if i == exprs.len() - 1 {
                        *out += "return ";
                        ex.write(out);
                    } else {
                        ex.write(out);
                        *out += "; ";
                    }
                }
                *out += " end)()";
            }
            Self::Print(exprs) => {
                *out += "_println(";
                let mut cw = CommaListWrite::new(out);
                for ex in exprs {
                    cw.write(|out| ex.write(out));
                }
                *out += ")";
            }
        }
    }

    fn wrap_in_parens(&mut self) {
        let dummy = Expr2::Literal(String::new());
        let temp = std::mem::replace(self, dummy);
        *self = Expr2::Paren(Box::new(temp));
    }

    fn ensure(&mut self, required: Precedence) {
        if self.precedence() > required {
            self.wrap_in_parens();
        }
    }

    fn add_parens(&mut self) {
        use Precedence::*;
        match self {
            Self::Paren(e) => {
                e.add_parens();
            }
            Self::Literal(_) => {}
            Self::Table(fields) => {
                for (_, val) in fields {
                    val.add_parens();
                    val.ensure(ASSIGN);
                }
            }
            Self::Var(_) => {}
            Self::Field(lhs, _) => {
                lhs.add_parens();
                lhs.ensure(MEMBER);
            }
            Self::ScopeField(..) => {}
            Self::Call(lhs, rhs) => {
                lhs.add_parens();
                lhs.ensure(MEMBER);
                rhs.add_parens();
                rhs.ensure(ASSIGN);
            }
            Self::Minus(e) => {
                e.add_parens();
                e.ensure(UNARY);
            }
            Self::Nil => {}
            Self::BinOp(lhs, rhs, op) => {
                use Op::*;
                let req = match op {
                    Mult | Div | Rem => (MULTIPLICATIVE, UNARY),
                    Add | Sub => (ADDITIVE, MULTIPLICATIVE),
                    Lt | Lte | Gt | Gte => (RELATIONAL, ADDITIVE),
                    Eq | Neq => (EQUALITY, RELATIONAL),
                };

                lhs.add_parens();
                lhs.ensure(req.0);
                rhs.add_parens();
                rhs.ensure(req.1);
            }
            Self::FloorDiv(lhs, rhs) => {
                lhs.add_parens();
                rhs.add_parens();
                // Inside math.floor(...) so no precedence constraints needed
            }
            Self::IfElse(cond, e1, e2) => {
                cond.add_parens();
                e1.add_parens();
                e2.add_parens();
                // All inside IIFE, no precedence constraints needed
            }
            Self::Assignment(lhs, rhs, _) => {
                lhs.add_parens();
                rhs.add_parens();
            }
            Self::Func(arg, _, body) => {
                arg.add_parens();
                body.add_parens();
            }
            Self::Do(exprs) => {
                for expr in exprs.iter_mut() {
                    expr.add_parens();
                }
            }
            Self::Print(exprs) => {
                for ex in exprs {
                    ex.add_parens();
                    ex.ensure(PRIMARY);
                }
            }
        }
    }

    fn should_inline(&self) -> bool {
        use Expr2::*;
        match &self {
            Literal(s) => s.len() <= 10,
            Minus(e) => e.should_inline(),
            ScopeField(..) => true,
            Var(..) => true,
            _ => false,
        }
    }
}

/// Helper for writing comma-separated lists
struct CommaListWrite<'a> {
    out: &'a mut String,
    first: bool,
}
impl<'a> CommaListWrite<'a> {
    fn new(out: &'a mut String) -> Self {
        Self { out, first: true }
    }

    fn write(&mut self, f: impl FnOnce(&mut String)) {
        if self.first {
            self.first = false;
        } else {
            *self.out += ", ";
        }
        f(self.out);
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////
// Dead code elimination
struct DeadCodeRemover {
    used: HashMap<String, HashSet<String>>,
}
impl DeadCodeRemover {
    fn new() -> Self {
        Self { used: HashMap::new() }
    }

    fn add_var(&mut self, scope: &str, name: String) {
        self.used.get_mut(scope).unwrap().insert(name);
    }

    fn remove_var_assign_if_unused(&self, expr: &mut Expr2) {
        use Expr2::*;
        if let Assignment(lhs, rhs, keep_if_unused) = expr {
            if !*keep_if_unused {
                if let ScopeField(ref s1, ref s2) = **lhs {
                    if !self.used.get(s1).unwrap().contains(s2) {
                        let rhs: Expr2 = std::mem::replace(&mut *rhs, Nil);
                        *expr = rhs;
                    }
                }
            }
        }
    }

    fn process_used_expr(&mut self, expr: &mut Expr2) {
        self.remove_var_assign_if_unused(expr);

        use Expr2::*;
        match expr {
            Paren(e) => {
                self.process_used_expr(e);
            }
            Literal(_) => {}
            Table(fields) => {
                for (_, val) in fields.iter_mut().rev() {
                    self.process_used_expr(val);
                }
            }
            Var(_) => {}
            Field(lhs, _) => {
                self.process_used_expr(lhs);
            }
            ScopeField(s1, s2) => {
                self.add_var(s1, s2.clone());
            }
            Call(lhs, rhs) => {
                self.process_used_expr(rhs);
                self.process_used_expr(lhs);
            }
            Minus(e) => {
                self.process_used_expr(e);
            }
            Nil => {}
            BinOp(lhs, rhs, _) => {
                self.process_used_expr(rhs);
                self.process_used_expr(lhs);
            }
            FloorDiv(lhs, rhs) => {
                self.process_used_expr(rhs);
                self.process_used_expr(lhs);
            }
            IfElse(cond, e1, e2) => {
                self.process_used_expr(e2);
                self.process_used_expr(e1);
                self.process_used_expr(cond);
            }
            Assignment(lhs, rhs, _) => {
                self.process_used_expr(rhs);
                self.process_used_expr(lhs);
            }
            Func(_, scope, body) => {
                self.used.insert(scope.clone(), HashSet::new());
                self.process_used_expr(body);
                self.used.remove(scope);
            }
            Do(exprs) => {
                let mut last = exprs.pop().unwrap();
                self.process_used_expr(&mut last);
                let mut out = vec![last];

                while let Some(ex) = exprs.pop() {
                    self.process_unused_expr(ex, &mut out);
                }

                out.reverse();
                *expr = do_block_sub(out);
            }
            Print(exprs) => {
                for ex in exprs.iter_mut().rev() {
                    self.process_used_expr(ex);
                }
            }
        }
    }

    fn process_unused_expr(&mut self, mut expr: Expr2, out: &mut Vec<Expr2>) {
        self.remove_var_assign_if_unused(&mut expr);

        use Expr2::*;
        match expr {
            Paren(e) => {
                self.process_unused_expr(*e, out);
            }
            Literal(_) => {}
            Table(fields) => {
                for (_, val) in fields.into_iter().rev() {
                    self.process_unused_expr(*val, out);
                }
            }
            Var(_) => {}
            Field(lhs, _) => {
                self.process_unused_expr(*lhs, out);
            }
            ScopeField(..) => {}
            Minus(e) => {
                self.process_unused_expr(*e, out);
            }
            Nil => {}
            BinOp(lhs, rhs, _) => {
                self.process_unused_expr(*rhs, out);
                self.process_unused_expr(*lhs, out);
            }
            FloorDiv(lhs, rhs) => {
                self.process_unused_expr(*rhs, out);
                self.process_unused_expr(*lhs, out);
            }
            Func(..) => {}
            Do(exprs) => {
                for ex in exprs.into_iter().rev() {
                    self.process_unused_expr(ex, out);
                }
            }
            Call(..) | IfElse(..) | Assignment(..) | Print(..) => {
                self.process_used_expr(&mut expr);
                out.push(expr);
            }
        }
    }
}

pub fn optimize(expr: &mut Expr, main_scope_name: String, bindings: &crate::ast::StringIdMap<Expr>) {
    let mut optimizer = DeadCodeRemover::new();
    optimizer.used.insert(main_scope_name, HashSet::new());
    for expr in bindings.values() {
        if let Expr2::ScopeField(s1, s2) = &expr.0 {
            optimizer.add_var(&s1, s2.clone());
        }
    }
    optimizer.process_used_expr(&mut expr.0);
}
