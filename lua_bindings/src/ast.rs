use mlua::prelude::{LuaError, LuaUserData, LuaUserDataMethods};

use alsub::ast::{self, Expr, LetPattern, SExpr, Statement};

use crate::core::LuaSpan;

// ---- LuaScript ----

pub struct LuaScript(pub Vec<Statement>);
impl LuaUserData for LuaScript {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| Ok(this.0.len()));

        // 1-indexed
        methods.add_method("get", |_, this, i: usize| {
            if i == 0 || i > this.0.len() {
                return Err(LuaError::runtime(format!(
                    "Index {} out of bounds (1..{})",
                    i,
                    this.0.len()
                )));
            }
            Ok(LuaStatement(this.0[i - 1].clone()))
        });
    }
}

// ---- LuaStatement ----

pub struct LuaStatement(pub Statement);
impl LuaUserData for LuaStatement {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("kind", |_, this, ()| {
            Ok(match &this.0 {
                Statement::Empty => "Empty",
                Statement::Expr(_) => "Expr",
                Statement::LetDef(_) => "LetDef",
                Statement::LetRecDef(_) => "LetRecDef",
                Statement::Println(_) => "Println",
            })
        });

        // Expr variant accessor
        methods.add_method("expr", |_, this, ()| match &this.0 {
            Statement::Expr(e) => Ok(LuaExpr(e.clone())),
            _ => Err(LuaError::runtime("Statement is not Expr")),
        });

        // LetDef accessors
        methods.add_method("let_pattern", |_, this, ()| match &this.0 {
            Statement::LetDef((pat, _)) => Ok(LuaLetPattern(pat.clone())),
            _ => Err(LuaError::runtime("Statement is not LetDef")),
        });

        methods.add_method("let_expr", |_, this, ()| match &this.0 {
            Statement::LetDef((_, expr)) => Ok(LuaExpr(*expr.clone())),
            _ => Err(LuaError::runtime("Statement is not LetDef")),
        });

        // LetRecDef accessor: returns array of {name=string, expr=LuaExpr}
        methods.add_method("let_rec_defs", |lua, this, ()| match &this.0 {
            Statement::LetRecDef(defs) => {
                let tbl = lua.create_table()?;
                for (i, (name, sexpr)) in defs.iter().enumerate() {
                    let entry = lua.create_table()?;
                    entry.set("name", name.as_str())?;
                    entry.set("expr", LuaExpr(sexpr.clone()))?;
                    tbl.set(i + 1, entry)?;
                }
                Ok(tbl)
            }
            _ => Err(LuaError::runtime("Statement is not LetRecDef")),
        });

        // Println accessor: returns array of LuaExpr
        methods.add_method("println_args", |lua, this, ()| match &this.0 {
            Statement::Println(exprs) => {
                let tbl = lua.create_table()?;
                for (i, e) in exprs.iter().enumerate() {
                    tbl.set(i + 1, LuaExpr(e.clone()))?;
                }
                Ok(tbl)
            }
            _ => Err(LuaError::runtime("Statement is not Println")),
        });
    }
}

// ---- LuaExpr ----

pub struct LuaExpr(pub SExpr);
impl LuaUserData for LuaExpr {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("kind", |_, this, ()| {
            Ok(match &this.0 .0 {
                Expr::BinOp(_) => "BinOp",
                Expr::Block(_) => "Block",
                Expr::Call(_) => "Call",
                Expr::Case(_) => "Case",
                Expr::FieldAccess(_) => "FieldAccess",
                Expr::FieldSet(_) => "FieldSet",
                Expr::FuncDef(_) => "FuncDef",
                Expr::InstantiateExist(_) => "InstantiateExist",
                Expr::InstantiateUni(_) => "InstantiateUni",
                Expr::Literal(_) => "Literal",
                Expr::Loop(_) => "Loop",
                Expr::Match(_) => "Match",
                Expr::Record(_) => "Record",
                Expr::Typed(_) => "Typed",
                Expr::Variable(_) => "Variable",
            })
        });

        methods.add_method("span", |_, this, ()| Ok(LuaSpan(this.0 .1)));

        // ---- BinOp ----
        methods.add_method("lhs", |_, this, ()| match &this.0 .0 {
            Expr::BinOp(e) => Ok(LuaExpr(*e.lhs.clone())),
            _ => Err(LuaError::runtime("Not a BinOp")),
        });
        methods.add_method("rhs", |_, this, ()| match &this.0 .0 {
            Expr::BinOp(e) => Ok(LuaExpr(*e.rhs.clone())),
            _ => Err(LuaError::runtime("Not a BinOp")),
        });
        methods.add_method("op", |_, this, ()| match &this.0 .0 {
            Expr::BinOp(e) => Ok(format!("{:?}", e.op)),
            _ => Err(LuaError::runtime("Not a BinOp")),
        });

        // ---- Block ----
        methods.add_method("statements", |lua, this, ()| match &this.0 .0 {
            Expr::Block(e) => {
                let tbl = lua.create_table()?;
                for (i, stmt) in e.statements.iter().enumerate() {
                    tbl.set(i + 1, LuaStatement(stmt.clone()))?;
                }
                Ok(tbl)
            }
            _ => Err(LuaError::runtime("Not a Block")),
        });
        methods.add_method("block_expr", |_, this, ()| match &this.0 .0 {
            Expr::Block(e) => Ok(LuaExpr(*e.expr.clone())),
            _ => Err(LuaError::runtime("Not a Block")),
        });

        // ---- Call ----
        methods.add_method("func", |_, this, ()| match &this.0 .0 {
            Expr::Call(e) => Ok(LuaExpr(*e.func.clone())),
            _ => Err(LuaError::runtime("Not a Call")),
        });
        methods.add_method("arg", |_, this, ()| match &this.0 .0 {
            Expr::Call(e) => Ok(LuaExpr(*e.arg.clone())),
            _ => Err(LuaError::runtime("Not a Call")),
        });

        // ---- Case ----
        methods.add_method("tag", |_, this, ()| match &this.0 .0 {
            Expr::Case(e) => Ok(e.tag.0.as_str().to_owned()),
            _ => Err(LuaError::runtime("Not a Case")),
        });
        methods.add_method("case_expr", |_, this, ()| match &this.0 .0 {
            Expr::Case(e) => Ok(LuaExpr(*e.expr.clone())),
            _ => Err(LuaError::runtime("Not a Case")),
        });

        // ---- FieldAccess / FieldSet ----
        methods.add_method("field_expr", |_, this, ()| match &this.0 .0 {
            Expr::FieldAccess(e) => Ok(LuaExpr(*e.expr.clone())),
            Expr::FieldSet(e) => Ok(LuaExpr(*e.expr.clone())),
            _ => Err(LuaError::runtime("Not a FieldAccess or FieldSet")),
        });
        methods.add_method("field_name", |_, this, ()| match &this.0 .0 {
            Expr::FieldAccess(e) => Ok(e.field.0.as_str().to_owned()),
            Expr::FieldSet(e) => Ok(e.field.0.as_str().to_owned()),
            _ => Err(LuaError::runtime("Not a FieldAccess or FieldSet")),
        });
        methods.add_method("field_value", |_, this, ()| match &this.0 .0 {
            Expr::FieldSet(e) => Ok(LuaExpr(*e.value.clone())),
            _ => Err(LuaError::runtime("Not a FieldSet")),
        });

        // ---- FuncDef ----
        methods.add_method("param", |_, this, ()| match &this.0 .0 {
            Expr::FuncDef(e) => Ok(LuaLetPattern(e.param.0.clone())),
            _ => Err(LuaError::runtime("Not a FuncDef")),
        });
        methods.add_method("body", |_, this, ()| match &this.0 .0 {
            Expr::FuncDef(e) => Ok(LuaExpr(*e.body.clone())),
            _ => Err(LuaError::runtime("Not a FuncDef")),
        });
        // return_type is an optional STypeExpr; return nil if absent
        methods.add_method("return_type", |_, this, ()| match &this.0 .0 {
            Expr::FuncDef(e) => Ok(e.return_type.is_some()),
            _ => Err(LuaError::runtime("Not a FuncDef")),
        });

        // ---- Literal ----
        methods.add_method("lit_type", |_, this, ()| match &this.0 .0 {
            Expr::Literal(e) => Ok(match e.lit_type {
                ast::Literal::Float => "Float",
                ast::Literal::Int => "Int",
                ast::Literal::Str => "Str",
            }),
            _ => Err(LuaError::runtime("Not a Literal")),
        });
        methods.add_method("lit_value", |_, this, ()| match &this.0 .0 {
            Expr::Literal(e) => Ok(e.value.0.clone()),
            _ => Err(LuaError::runtime("Not a Literal")),
        });

        // ---- Loop ----
        methods.add_method("loop_body", |_, this, ()| match &this.0 .0 {
            Expr::Loop(e) => Ok(LuaExpr(*e.body.clone())),
            _ => Err(LuaError::runtime("Not a Loop")),
        });

        // ---- Match ----
        methods.add_method("match_expr", |_, this, ()| match &this.0 .0 {
            Expr::Match(e) => Ok(LuaExpr(*e.expr.0.clone())),
            _ => Err(LuaError::runtime("Not a Match")),
        });
        // Returns array of { pattern = LuaLetPattern, expr = LuaExpr }
        methods.add_method("match_cases", |lua, this, ()| match &this.0 .0 {
            Expr::Match(e) => {
                let tbl = lua.create_table()?;
                for (i, ((pat, _span), expr)) in e.cases.iter().enumerate() {
                    let entry = lua.create_table()?;
                    entry.set("pattern", LuaLetPattern(pat.clone()))?;
                    entry.set("expr", LuaExpr(*expr.clone()))?;
                    tbl.set(i + 1, entry)?;
                }
                Ok(tbl)
            }
            _ => Err(LuaError::runtime("Not a Match")),
        });

        // ---- Record ----
        // Returns array of { name = string, expr = LuaExpr, mutable = bool }
        methods.add_method("record_fields", |lua, this, ()| match &this.0 .0 {
            Expr::Record(e) => {
                let tbl = lua.create_table()?;
                for (i, ((name, _), expr, mutable, _type_annot)) in e.fields.iter().enumerate() {
                    let entry = lua.create_table()?;
                    entry.set("name", name.as_str())?;
                    entry.set("expr", LuaExpr(*expr.clone()))?;
                    entry.set("mutable", *mutable)?;
                    tbl.set(i + 1, entry)?;
                }
                Ok(tbl)
            }
            _ => Err(LuaError::runtime("Not a Record")),
        });

        // ---- Typed ----
        methods.add_method("typed_expr", |_, this, ()| match &this.0 .0 {
            Expr::Typed(e) => Ok(LuaExpr(*e.expr.clone())),
            _ => Err(LuaError::runtime("Not a Typed")),
        });

        // ---- Variable ----
        methods.add_method("var_name", |_, this, ()| match &this.0 .0 {
            Expr::Variable(e) => Ok(e.name.as_str().to_owned()),
            _ => Err(LuaError::runtime("Not a Variable")),
        });
    }
}

// ---- LuaLetPattern ----

pub struct LuaLetPattern(pub LetPattern);
impl LuaUserData for LuaLetPattern {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("kind", |_, this, ()| {
            Ok(match &this.0 {
                LetPattern::Case(..) => "Case",
                LetPattern::Record(..) => "Record",
                LetPattern::Var(..) => "Var",
            })
        });

        // Case variant
        methods.add_method("tag", |_, this, ()| match &this.0 {
            LetPattern::Case((tag, _span), _) => Ok(tag.as_str().to_owned()),
            _ => Err(LuaError::runtime("Not a Case pattern")),
        });
        methods.add_method("sub_pattern", |_, this, ()| match &this.0 {
            LetPattern::Case(_, sub) => Ok(LuaLetPattern(*sub.clone())),
            _ => Err(LuaError::runtime("Not a Case pattern")),
        });

        // Record variant: returns array of { name = string, pattern = LuaLetPattern }
        methods.add_method("record_fields", |lua, this, ()| match &this.0 {
            LetPattern::Record(((_, fields), _)) => {
                let tbl = lua.create_table()?;
                for (i, ((name, _), pat)) in fields.iter().enumerate() {
                    let entry = lua.create_table()?;
                    entry.set("name", name.as_str())?;
                    entry.set("pattern", LuaLetPattern(*pat.clone()))?;
                    tbl.set(i + 1, entry)?;
                }
                Ok(tbl)
            }
            _ => Err(LuaError::runtime("Not a Record pattern")),
        });

        // Var variant
        methods.add_method("var_name", |_, this, ()| match &this.0 {
            LetPattern::Var((name, _), _) => Ok(name.map(|n| n.as_str().to_owned())),
            _ => Err(LuaError::runtime("Not a Var pattern")),
        });
        methods.add_method("type_annotation", |_, this, ()| match &this.0 {
            LetPattern::Var(_, annot) => Ok(annot.is_some()),
            _ => Err(LuaError::runtime("Not a Var pattern")),
        });
    }
}
