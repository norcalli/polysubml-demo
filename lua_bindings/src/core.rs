use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::{
    FromLua, Lua, LuaAnyUserData, LuaError, LuaMetaMethod, LuaResult, LuaTable,
    LuaUserData, LuaUserDataMethods,
};

use alsub::ast::PolyKind;
use alsub::ast::StringIdMap;
use alsub::type_errors::HoleSrc;
use alsub::{
    Bindings, PolyHeadData, ScopeLvl, SourceLoc, Span, SpanManager, SpannedError,
    TypeCheckerCore, TypeCtorInd, TypeckState, Use, Value, VarSpec,
};

// ---- Macro for FromLua on Copy wrapper types (IntoLua is auto-impl'd by mlua) ----

macro_rules! impl_from_lua {
    ($ty:ident) => {
        impl FromLua for $ty {
            fn from_lua(value: mlua::Value, _lua: &Lua) -> LuaResult<Self> {
                match value {
                    mlua::Value::UserData(ud) => {
                        let v = ud.borrow::<Self>()?;
                        Ok(*v)
                    }
                    _ => Err(LuaError::FromLuaConversionError {
                        from: value.type_name(),
                        to: stringify!($ty).to_string(),
                        message: None,
                    }),
                }
            }
        }
    };
}

macro_rules! impl_from_lua_clone {
    ($ty:ident) => {
        impl FromLua for $ty {
            fn from_lua(value: mlua::Value, _lua: &Lua) -> LuaResult<Self> {
                match value {
                    mlua::Value::UserData(ud) => {
                        let v = ud.borrow::<Self>()?;
                        Ok(v.clone())
                    }
                    _ => Err(LuaError::FromLuaConversionError {
                        from: value.type_name(),
                        to: stringify!($ty).to_string(),
                        message: None,
                    }),
                }
            }
        }
    };
}

// ---- Opaque Handle Types ----

#[derive(Clone, Copy)]
pub struct LuaValue(pub Value);
impl LuaUserData for LuaValue {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("Value({})", this.0 .0 .0))
        });
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(this.0 == other.0)
        });
    }
}
impl_from_lua!(LuaValue);

#[derive(Clone, Copy)]
pub struct LuaUse(pub Use);
impl LuaUserData for LuaUse {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("Use({})", this.0 .0 .0))
        });
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaUse| {
            Ok(this.0 == other.0)
        });
    }
}
impl_from_lua!(LuaUse);

#[derive(Clone, Copy)]
pub struct LuaScopeLvl(pub ScopeLvl);
impl LuaUserData for LuaScopeLvl {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("ScopeLvl({})", this.0 .0))
        });
    }
}
impl_from_lua!(LuaScopeLvl);

#[derive(Clone, Copy)]
pub struct LuaTypeCtorInd(pub TypeCtorInd);
impl LuaUserData for LuaTypeCtorInd {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("TypeCtorInd({})", this.0 .0))
        });
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaTypeCtorInd| {
            Ok(this.0 == other.0)
        });
    }
}
impl_from_lua!(LuaTypeCtorInd);

#[derive(Clone, Copy)]
pub struct LuaSpan(pub Span);
impl LuaUserData for LuaSpan {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("Span({:?})", this.0))
        });
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaSpan| {
            Ok(this.0 == other.0)
        });
    }
}
impl_from_lua!(LuaSpan);

#[derive(Clone, Copy)]
pub struct LuaSourceLoc(pub SourceLoc);
impl LuaUserData for LuaSourceLoc {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("SourceLoc({:?})", this.0))
        });
    }
}
impl_from_lua!(LuaSourceLoc);

// ---- Span Management ----

#[derive(Clone)]
pub struct LuaSpanManager(pub Rc<RefCell<SpanManager>>);
impl LuaUserData for LuaSpanManager {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("add_source", |_, this, source: String| {
            let mut sm = this.0.borrow_mut();
            let source_ind = sm.source_count();
            let _ = sm.add_source(source);
            drop(sm);
            Ok(LuaSpanMaker {
                sm: Rc::clone(&this.0),
                source_ind,
            })
        });

        methods.add_method("format_error", |_, this, err: LuaAnyUserData| {
            let err = err.borrow::<LuaSpannedError>()?;
            let sm = this.0.borrow();
            Ok(err.0.print(&sm))
        });
    }
}
impl_from_lua_clone!(LuaSpanManager);

pub struct LuaSpanMaker {
    sm: Rc<RefCell<SpanManager>>,
    source_ind: usize,
}
impl Clone for LuaSpanMaker {
    fn clone(&self) -> Self {
        Self {
            sm: Rc::clone(&self.sm),
            source_ind: self.source_ind,
        }
    }
}
impl LuaUserData for LuaSpanMaker {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("span", |_, this, (l, r): (usize, usize)| {
            let mut sm = this.sm.borrow_mut();
            let span = sm.new_span(this.source_ind, l, r);
            Ok(LuaSpan(span))
        });
    }
}
impl_from_lua_clone!(LuaSpanMaker);

pub struct LuaSpannedError(pub SpannedError);
impl LuaUserData for LuaSpannedError {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, _this, ()| {
            Ok("SpannedError".to_string())
        });
    }
}

// ---- LuaCore wrapping TypeCheckerCore ----

pub struct LuaCore(pub TypeCheckerCore);
impl LuaUserData for LuaCore {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Clone / snapshot
        methods.add_method("clone", |_, this, ()| Ok(LuaCore(this.0.clone())));

        // Inference variables
        methods.add_method_mut(
            "var",
            |_, this, (hole_src_str, span, scopelvl): (String, LuaSpan, LuaScopeLvl)| {
                let src = parse_hole_src(&hole_src_str, span.0)?;
                let (v, u) = this.0.var(src, scopelvl.0);
                Ok((LuaValue(v), LuaUse(u)))
            },
        );

        // Flow constraint (throws on error)
        methods.add_method_mut(
            "flow",
            |_, this, (value, use_, span, scopelvl): (LuaValue, LuaUse, LuaSpan, LuaScopeLvl)| {
                this.0
                    .flow(value.0, use_.0, span.0, scopelvl.0)
                    .map_err(|e| LuaError::runtime(format!("TypeError: {:?}", e)))?;
                Ok(())
            },
        );

        // Flow constraint (returns nil, SpannedError on error instead of throwing)
        methods.add_method_mut(
            "try_flow",
            |lua, this, (value, use_, span, scopelvl): (LuaValue, LuaUse, LuaSpan, LuaScopeLvl)| {
                match this.0.flow(value.0, use_.0, span.0, scopelvl.0) {
                    Ok(()) => Ok((true, mlua::Value::Nil)),
                    Err(e) => {
                        let ud = lua.create_any_userdata(LuaSpannedError(e))?;
                        Ok((false, mlua::Value::UserData(ud)))
                    }
                }
            },
        );

        // Bot / top_use
        methods.add_method("bot", |_, this, ()| Ok(LuaValue(this.0.bot())));
        methods.add_method("top_use", |_, this, ()| Ok(LuaUse(this.0.top_use())));

        // Abstract types
        methods.add_method_mut("add_builtin_type", |_, this, name: String| {
            let name = ustr::ustr(&name);
            Ok(LuaTypeCtorInd(this.0.add_builtin_type(name)))
        });

        methods.add_method_mut(
            "add_abstract_type",
            |_, this, (name, span, scopelvl): (String, LuaSpan, LuaScopeLvl)| {
                let name = ustr::ustr(&name);
                Ok(LuaTypeCtorInd(
                    this.0.add_abstract_type(name, span.0, scopelvl.0),
                ))
            },
        );

        methods.add_method_mut("custom", |_, this, (ty, span): (LuaTypeCtorInd, LuaSpan)| {
            let (v, u) = this.0.custom(ty.0, span.0);
            Ok((LuaValue(v), LuaUse(u)))
        });

        // Placeholders
        methods.add_method_mut("val_placeholder", |_, this, ()| {
            Ok(LuaValue(this.0.val_placeholder()))
        });
        methods.add_method_mut("use_placeholder", |_, this, ()| {
            Ok(LuaUse(this.0.use_placeholder()))
        });

        // Type description methods
        methods.add_method("describe_value", |_, this, value: LuaValue| {
            Ok(crate::describe::describe_value(&this.0, value.0))
        });
        methods.add_method("describe_use", |_, this, use_: LuaUse| {
            Ok(crate::describe::describe_use(&this.0, use_.0))
        });
        methods.add_method("describe_demanded", |_, this, value: LuaValue| {
            Ok(crate::describe::describe_demanded(&this.0, value.0))
        });

        // Type head construction methods are registered in types.rs
        crate::types::register_core_type_methods(methods);
    }
}

// ---- Bindings ----

pub struct LuaBindings(pub Bindings);
impl LuaUserData for LuaBindings {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_var", |_, this, name: String| {
            let name = ustr::ustr(&name);
            Ok(this.0.vars.get(&name).copied().map(LuaValue))
        });

        methods.add_method_mut("set_var", |_, this, (name, val): (String, LuaValue)| {
            let name = ustr::ustr(&name);
            this.0.vars.insert(name, val.0);
            Ok(())
        });

        methods.add_method("get_type", |_, this, name: String| {
            let name = ustr::ustr(&name);
            Ok(this.0.types.get(&name).copied().map(LuaTypeCtorInd))
        });

        methods.add_method_mut(
            "set_type",
            |_, this, (name, ty): (String, LuaTypeCtorInd)| {
                let name = ustr::ustr(&name);
                this.0.types.insert(name, ty.0);
                Ok(())
            },
        );

        methods.add_method("scopelvl", |_, this, ()| Ok(LuaScopeLvl(this.0.scopelvl)));

        methods.add_method_mut("set_scopelvl", |_, this, lvl: LuaScopeLvl| {
            this.0.scopelvl = lvl.0;
            Ok(())
        });

        methods.add_method("clone", |_, this, ()| Ok(LuaBindings(this.0.clone())));
    }
}

// ---- TypeckState ----

pub struct LuaTypeckState(pub TypeckState);
impl LuaUserData for LuaTypeckState {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("check_script", |_, this, script: LuaAnyUserData| {
            let script = script.borrow::<crate::ast::LuaScript>()?;
            this.0
                .check_script(&script.0)
                .map_err(|e| LuaError::runtime(format!("{:?}", e)))?;
            Ok(())
        });
    }
}

// ---- PolyHeadData / VarSpec constructors ----

#[derive(Clone)]
pub struct LuaPolyHeadData(pub Rc<PolyHeadData>);
impl LuaUserData for LuaPolyHeadData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("PolyHeadData({:?})", this.0.kind))
        });
    }
}
impl_from_lua_clone!(LuaPolyHeadData);

#[derive(Clone, Copy)]
pub struct LuaVarSpec(pub VarSpec);
impl LuaUserData for LuaVarSpec {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("VarSpec({})", this.0.name.as_str()))
        });
    }
}
impl_from_lua!(LuaVarSpec);

// ---- Constructor functions exposed to Lua ----

pub fn lua_new_source_loc(_lua: &Lua, span: LuaSpan) -> LuaResult<LuaSourceLoc> {
    Ok(LuaSourceLoc(SourceLoc::from_span(span.0)))
}

pub fn lua_new_var_spec(
    _lua: &Lua,
    (source_loc, name): (LuaSourceLoc, String),
) -> LuaResult<LuaVarSpec> {
    Ok(LuaVarSpec(VarSpec {
        loc: source_loc.0,
        name: ustr::ustr(&name),
    }))
}

pub fn lua_new_poly_head_data(
    _lua: &Lua,
    (kind_str, source_loc, params_table): (String, LuaSourceLoc, LuaTable),
) -> LuaResult<LuaPolyHeadData> {
    let kind = match kind_str.as_str() {
        "universal" | "Universal" => PolyKind::Universal,
        "existential" | "Existential" => PolyKind::Existential,
        _ => {
            return Err(LuaError::runtime(format!(
                "Unknown poly kind: '{}'. Expected: universal, existential",
                kind_str
            )))
        }
    };

    let mut params = StringIdMap::default();
    for pair in params_table.pairs::<String, LuaAnyUserData>() {
        let (name, span_ud) = pair?;
        let span = span_ud.borrow::<LuaSpan>()?;
        params.insert(ustr::ustr(&name), span.0);
    }

    Ok(LuaPolyHeadData(Rc::new(PolyHeadData {
        kind,
        loc: source_loc.0,
        params,
    })))
}

// ---- Helpers ----

fn parse_hole_src(kind: &str, span: Span) -> LuaResult<HoleSrc> {
    match kind {
        "explicit" => Ok(HoleSrc::Explicit(span)),
        "checked_expr" => Ok(HoleSrc::CheckedExpr(span)),
        "opt_ascribe" => Ok(HoleSrc::OptAscribe(span)),
        "bare_var_pattern" => Ok(HoleSrc::BareVarPattern(span)),
        _ => Err(LuaError::runtime(format!(
            "Unknown hole source kind: '{}'. Expected: explicit, checked_expr, opt_ascribe, bare_var_pattern",
            kind
        ))),
    }
}
