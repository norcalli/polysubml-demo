use std::mem::swap;

use alsub::ast;
use alsub::ast::{StringId, StringIdMap};
use alsub::Spanned;

use crate::lua;

pub struct ModuleBuilder {
    scope_var_name: String,
    scope_counter: u64,
    param_counter: u64,
    var_counter: u64,
    bindings: StringIdMap<lua::Expr>,
}
impl ModuleBuilder {
    pub fn new() -> Self {
        Self {
            scope_var_name: "_S".to_string(),
            scope_counter: 0,
            param_counter: 0,
            var_counter: 0,
            bindings: StringIdMap::default(),
        }
    }

    fn set_binding(&mut self, k: StringId, v: lua::Expr) {
        self.bindings.insert(k, v);
    }

    fn new_var_name(&mut self) -> String {
        let name = format!("v{}", self.var_counter);
        self.var_counter += 1;
        name
    }

    fn new_temp_var_assign(&mut self, rhs: lua::Expr, out: &mut Vec<lua::Expr>) -> lua::Expr {
        if rhs.should_inline() {
            return rhs;
        }

        let name = format!("t{}", self.var_counter);
        self.var_counter += 1;

        let expr = lua::scope_field(&self.scope_var_name, &name);
        out.push(lua::assign(expr.clone(), rhs, false));
        expr
    }

    fn new_var(&mut self, ml_name: StringId) -> lua::Expr {
        let name = self.new_var_name();
        let expr = lua::scope_field(&self.scope_var_name, &name);
        self.set_binding(ml_name, expr.clone());
        expr
    }

    fn new_var_assign(&mut self, ml_name: StringId, rhs: lua::Expr, out: &mut Vec<lua::Expr>) -> lua::Expr {
        if rhs.should_inline() {
            self.set_binding(ml_name, rhs.clone());
            return rhs;
        }

        let expr = self.new_var(ml_name);
        out.push(lua::assign(expr.clone(), rhs, false));
        expr
    }

    fn new_scope_name(&mut self) -> String {
        let name = format!("s{}", self.scope_counter);
        self.scope_counter += 1;
        name
    }

    fn new_param_name(&mut self) -> String {
        let name = format!("p{}", self.param_counter);
        self.param_counter += 1;
        name
    }
}
pub struct Context<'a>(pub &'a mut ModuleBuilder);
impl<'a> Context<'a> {
    fn ml_scope<T>(&mut self, cb: impl FnOnce(&mut Self) -> T) -> T {
        let saved = self.bindings.clone();
        let res = cb(self);
        self.bindings = saved;
        res
    }

    fn fn_scope<T>(&mut self, cb: impl FnOnce(&mut Self) -> T) -> T {
        let old_var_counter = self.var_counter;
        let old_param_counter = self.param_counter;
        let old_scope_counter = self.scope_counter;
        self.var_counter = 0;

        let res = self.ml_scope(cb);

        self.var_counter = old_var_counter;
        self.param_counter = old_param_counter;
        self.scope_counter = old_scope_counter;
        res
    }

    fn get(&self, id: StringId) -> &'static str {
        id.as_str()
    }

    /// Get a field name with @ prefix for Lua record fields
    fn field_name(&self, id: StringId) -> String {
        format!("@{}", id.as_str())
    }
}
impl<'a> core::ops::Deref for Context<'a> {
    type Target = ModuleBuilder;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a> core::ops::DerefMut for Context<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn is_bool_case(e: &ast::expr::CaseExpr) -> bool {
    (e.tag.0.as_str() == "t" || e.tag.0.as_str() == "f")
        && matches!(&e.expr.0, ast::Expr::Record(r) if r.fields.is_empty())
}

fn is_bool_match(cases: &[(Spanned<ast::LetPattern>, Box<ast::SExpr>)]) -> bool {
    let mut has_case = false;
    for ((pattern, _), _) in cases {
        match pattern {
            ast::LetPattern::Case((tag, _), _) => {
                let s = tag.as_str();
                if s != "t" && s != "f" {
                    return false;
                }
                has_case = true;
            }
            _ => {}
        }
    }
    has_case
}

fn compile(ctx: &mut Context<'_>, expr: &ast::SExpr) -> lua::Expr {
    match &expr.0 {
        ast::Expr::BinOp(e) => {
            let lhs = compile(ctx, &e.lhs);
            let rhs = compile(ctx, &e.rhs);
            match e.op {
                ast::Op::Div => lua::floor_div(lhs, rhs),
                _ => {
                    let op = match e.op {
                        ast::Op::Add => lua::Op::Add,
                        ast::Op::Sub => lua::Op::Sub,
                        ast::Op::Mult => lua::Op::Mult,
                        ast::Op::Div => unreachable!(),
                        ast::Op::Rem => lua::Op::Rem,

                        ast::Op::Lt => lua::Op::Lt,
                        ast::Op::Lte => lua::Op::Lte,
                        ast::Op::Gt => lua::Op::Gt,
                        ast::Op::Gte => lua::Op::Gte,

                        ast::Op::Eq => lua::Op::Eq,
                        ast::Op::Neq => lua::Op::Neq,
                    };
                    lua::binop(lhs, rhs, op)
                }
            }
        }
        ast::Expr::Block(e) => {
            ctx.ml_scope(|ctx| {
                let mut exprs = Vec::new();

                for stmt in &e.statements {
                    compile_statement(ctx, &mut exprs, stmt);
                }

                exprs.push(compile(ctx, &e.expr));
                lua::comma_list(exprs)
            })
        }
        ast::Expr::Call(e) => {
            if e.eval_arg_first {
                let mut exprs = Vec::new();
                let arg = compile(ctx, &e.arg);
                let arg = ctx.new_temp_var_assign(arg, &mut exprs);
                let func = compile(ctx, &e.func);
                exprs.push(lua::call(func, arg));
                lua::comma_list(exprs)
            } else {
                let func = compile(ctx, &e.func);
                let arg = compile(ctx, &e.arg);
                lua::call(func, arg)
            }
        }
        ast::Expr::Case(e) => {
            if is_bool_case(e) {
                lua::lit(if e.tag.0.as_str() == "t" { "true" } else { "false" })
            } else {
                let tag = lua::lit(format!("\"{}\"", ctx.get(e.tag.0)));
                let expr = compile(ctx, &e.expr);
                lua::obj(vec![("$tag".to_string(), tag), ("$val".to_string(), expr)])
            }
        }
        ast::Expr::FieldAccess(e) => {
            let lhs = compile(ctx, &e.expr);
            lua::field(lhs, ctx.field_name(e.field.0))
        }
        ast::Expr::FieldSet(e) => {
            let mut exprs = Vec::new();

            let lhs_compiled = compile(ctx, &e.expr);
            let lhs_temp_var = ctx.new_temp_var_assign(lhs_compiled, &mut exprs);
            let lhs = lua::field(lhs_temp_var, ctx.field_name(e.field.0));

            let res_temp_var = ctx.new_temp_var_assign(lhs.clone(), &mut exprs);
            exprs.push(lua::assign(lhs.clone(), compile(ctx, &e.value), false));
            exprs.push(res_temp_var);

            lua::comma_list(exprs)
        }
        ast::Expr::FuncDef(e) => {
            ctx.fn_scope(|ctx| {
                let mut new_scope_name = ctx.new_scope_name();
                swap(&mut new_scope_name, &mut ctx.scope_var_name);

                let lua_pattern = compile_let_pattern(ctx, &e.param.0).unwrap_or_else(|| lua::var("_"));
                let body = compile(ctx, &e.body);

                swap(&mut new_scope_name, &mut ctx.scope_var_name);
                lua::func(lua_pattern, new_scope_name, body)
            })
        }
        ast::Expr::InstantiateExist(e) => compile(ctx, &e.expr),
        ast::Expr::InstantiateUni(e) => compile(ctx, &e.expr),
        ast::Expr::Literal(e) => {
            let code = e.value.0.clone();
            // No "n" suffix for integers in Lua — doubles are used
            if code.starts_with("-") {
                lua::unary_minus(lua::lit(code[1..].to_string()))
            } else {
                lua::lit(code)
            }
        }
        ast::Expr::Loop(e) => {
            let lhs = lua::var("_loop");
            let rhs = compile(ctx, &e.body);
            let rhs = lua::func(lua::var("_"), "_2".to_string(), rhs);
            lua::call(lhs, rhs)
        }
        ast::Expr::Match(e) => {
            let is_bool = is_bool_match(&e.cases);

            let mut exprs = Vec::new();
            let match_compiled = compile(ctx, &e.expr.0);
            let temp_var = ctx.new_temp_var_assign(match_compiled, &mut exprs);

            let tag_expr = if is_bool {
                temp_var.clone()
            } else {
                lua::field(temp_var.clone(), "$tag")
            };
            let val_expr = if is_bool {
                lua::var("_UNIT")
            } else {
                lua::field(temp_var.clone(), "$val")
            };

            let mut branches = Vec::new();
            let mut wildcard = None;
            for ((pattern, _), rhs_expr) in &e.cases {
                use ast::LetPattern::*;
                match pattern {
                    Case((tag, _), sub_pattern) => {
                        ctx.ml_scope(|ctx| {
                            let mut exprs = Vec::new();
                            compile_let_pattern_flat(ctx, &mut exprs, sub_pattern, val_expr.clone());
                            exprs.push(compile(ctx, rhs_expr));
                            branches.push((ctx.get(*tag), lua::comma_list(exprs)));
                        });
                    }
                    _ => {
                        wildcard = Some(ctx.ml_scope(|ctx| {
                            let mut exprs = Vec::new();
                            compile_let_pattern_flat(ctx, &mut exprs, pattern, temp_var.clone());
                            exprs.push(compile(ctx, rhs_expr));
                            lua::comma_list(exprs)
                        }));
                    }
                }
            }

            let mut res = wildcard.unwrap_or_else(|| branches.pop().unwrap().1);
            while let Some((tag, rhs_expr)) = branches.pop() {
                assert!(tag.len() > 0);
                let cond = if is_bool {
                    lua::eqop(tag_expr.clone(), lua::lit(if tag == "t" { "true" } else { "false" }))
                } else {
                    lua::eqop(tag_expr.clone(), lua::lit(format!("\"{}\"", tag)))
                };
                res = lua::ternary(cond, rhs_expr, res);
            }

            exprs.push(res);
            lua::comma_list(exprs)
        }
        ast::Expr::Record(e) => {
            if e.fields.is_empty() {
                lua::var("_UNIT")
            } else {
                lua::obj(
                    e.fields
                        .iter()
                        .map(|((name, _), expr, _, _)| (ctx.field_name(*name), compile(ctx, expr)))
                        .collect(),
                )
            }
        }
        ast::Expr::Typed(e) => compile(ctx, &e.expr),
        ast::Expr::Variable(e) => ctx.bindings.get(&e.name).unwrap().clone(),
    }
}

fn compile_let_pattern_flat(ctx: &mut Context<'_>, out: &mut Vec<lua::Expr>, pat: &ast::LetPattern, rhs: lua::Expr) {
    use ast::LetPattern::*;
    match pat {
        Case(_, val_pat) => {
            let rhs = lua::field(rhs, "$val".to_string());
            compile_let_pattern_flat(ctx, out, val_pat, rhs);
        }
        Record(((_, pairs), _)) => {
            let lhs = ctx.new_temp_var_assign(rhs, out);

            for ((name, _), pat) in pairs.iter() {
                compile_let_pattern_flat(ctx, out, pat, lua::field(lhs.clone(), ctx.field_name(*name)));
            }
        }

        Var((ml_name, _), _) => {
            if let Some(ml_name) = ml_name {
                ctx.new_var_assign(*ml_name, rhs, out);
            } else {
                out.push(rhs);
            }
        }
    }
}

fn compile_let_pattern(ctx: &mut Context<'_>, pat: &ast::LetPattern) -> Option<lua::Expr> {
    use ast::LetPattern::*;
    Some(match pat {
        Case(_, val_pat) => lua::obj(vec![("$val".to_string(), compile_let_pattern(ctx, &*val_pat)?)]),
        Record(((_, pairs), _)) => lua::obj(
            pairs
                .iter()
                .filter_map(|((name, _), pat)| Some((ctx.field_name(*name), compile_let_pattern(ctx, &*pat)?)))
                .collect(),
        ),

        Var((ml_name, _), _) => {
            let arg = lua::var(ctx.new_param_name());
            let ml_name = ml_name.as_ref()?;
            ctx.set_binding(*ml_name, arg.clone());
            arg
        }
    })
}

fn compile_statement(ctx: &mut Context<'_>, exprs: &mut Vec<lua::Expr>, stmt: &ast::Statement) {
    use ast::Statement::*;
    match stmt {
        Empty => {}
        Expr(expr) => exprs.push(compile(ctx, expr)),
        LetDef((pat, var_expr)) => {
            let rhs = compile(ctx, var_expr);
            compile_let_pattern_flat(ctx, exprs, pat, rhs);
        }
        LetRecDef(defs) => {
            let mut vars = Vec::new();
            let mut rhs_exprs = Vec::new();
            for (name, _) in defs {
                vars.push(ctx.new_var(*name))
            }
            for (_, expr) in defs {
                rhs_exprs.push(compile(ctx, expr))
            }

            let dont_optimize = vars.len() > 1;
            for (lhs, rhs) in vars.into_iter().zip(rhs_exprs) {
                exprs.push(lua::assign(lhs, rhs, dont_optimize));
            }
        }
        Println(args) => {
            let args = args.iter().map(|expr| compile(ctx, expr)).collect();
            exprs.push(lua::println(args));
        }
    }
}

pub fn compile_script(ctx: &mut Context<'_>, parsed: &[ast::Statement]) -> lua::Expr {
    let mut exprs = Vec::new();

    for item in parsed {
        compile_statement(ctx, &mut exprs, item);
    }
    if !matches!(parsed.last(), Some(ast::Statement::Expr(_))) {
        exprs.push(lua::nil());
    }

    let mut res = lua::comma_list(exprs);
    lua::optimize(&mut res, ctx.scope_var_name.to_owned(), &ctx.bindings);
    res
}
