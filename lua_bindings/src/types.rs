use mlua::prelude::{
    FromLua, Lua, LuaAnyUserData, LuaError, LuaResult, LuaTable, LuaUserData,
    LuaUserDataMethods,
};

use alsub::ast::StringIdMap;
use alsub::{PolyDeps, UTypeHead, VTypeHead};

use crate::core::{LuaCore, LuaPolyHeadData, LuaSpan, LuaTypeCtorInd, LuaUse, LuaValue};

/// Register VTypeHead/UTypeHead construction methods on LuaCore.
pub fn register_core_type_methods<M: LuaUserDataMethods<LuaCore>>(methods: &mut M) {
    // ---- Constructing Value types ----

    // core:val_func(arg_use, ret_value, span) -> LuaValue
    methods.add_method_mut(
        "val_func",
        |_, this, (arg, ret, span): (LuaUse, LuaValue, LuaSpan)| {
            let v = this.0.new_val(
                VTypeHead::VFunc {
                    arg: arg.0,
                    ret: ret.0,
                },
                span.0,
                None,
            );
            Ok(LuaValue(v))
        },
    );

    // core:val_obj(fields_table, span) -> LuaValue
    // fields_table: { name = { read = LuaValue, write = LuaUse|nil, span = LuaSpan } }
    methods.add_method_mut("val_obj", |_, this, (tbl, span): (LuaTable, LuaSpan)| {
        let fields = parse_val_obj_fields(&tbl)?;
        let v = this.0.new_val(VTypeHead::VObj { fields }, span.0, None);
        Ok(LuaValue(v))
    });

    // core:val_case(tag, value, span) -> LuaValue
    methods.add_method_mut(
        "val_case",
        |_, this, (tag, value, span): (String, LuaValue, LuaSpan)| {
            let tag = ustr::ustr(&tag);
            let v = this.0.new_val(
                VTypeHead::VCase {
                    case: (tag, value.0),
                },
                span.0,
                None,
            );
            Ok(LuaValue(v))
        },
    );

    // core:val_union(values_array, span) -> LuaValue
    methods.add_method_mut(
        "val_union",
        |_, this, (tbl, span): (LuaTable, LuaSpan)| {
            let values: Vec<alsub::Value> = tbl
                .sequence_values::<LuaAnyUserData>()
                .map(|ud| {
                    let ud = ud?;
                    let v = ud.borrow::<LuaValue>()?;
                    Ok(v.0)
                })
                .collect::<LuaResult<Vec<_>>>()?;
            let v = this
                .0
                .new_val(VTypeHead::VUnion(values.into()), span.0, None);
            Ok(LuaValue(v))
        },
    );

    // core:val_top(span) -> LuaValue
    methods.add_method_mut("val_top", |_, this, span: LuaSpan| {
        let v = this.0.new_val(VTypeHead::VTop, span.0, None);
        Ok(LuaValue(v))
    });

    // core:val_abstract(type_ctor_ind, span) -> LuaValue
    methods.add_method_mut(
        "val_abstract",
        |_, this, (ty, span): (LuaTypeCtorInd, LuaSpan)| {
            let v = this.0.simple_val(ty.0, span.0);
            Ok(LuaValue(v))
        },
    );

    // core:val_poly(poly_data, value, poison, span) -> LuaValue
    methods.add_method_mut(
        "val_poly",
        |_, this, (poly, value, poison, span): (LuaPolyHeadData, LuaValue, bool, LuaSpan)| {
            let deps = PolyDeps::single(poly.0.loc);
            let v = this.0.new_val(
                VTypeHead::VPolyHead(poly.0.clone(), value.0, poison),
                span.0,
                Some(deps),
            );
            Ok(LuaValue(v))
        },
    );

    // core:set_val(ph, head, span) — fill a value placeholder
    methods.add_method_mut(
        "set_val",
        |_, this, (ph, head, span): (LuaValue, LuaVTypeHeadWrapper, LuaSpan)| {
            this.0.set_val(ph.0, head.0.clone(), span.0, None);
            Ok(())
        },
    );

    // ---- Constructing Use types ----

    // core:use_func(arg_value, ret_use, span) -> LuaUse
    methods.add_method_mut(
        "use_func",
        |_, this, (arg, ret, span): (LuaValue, LuaUse, LuaSpan)| {
            let u = this.0.new_use(
                UTypeHead::UFunc {
                    arg: arg.0,
                    ret: ret.0,
                },
                span.0,
                None,
            );
            Ok(LuaUse(u))
        },
    );

    // core:use_obj(fields_table, span) -> LuaUse
    // fields_table: { name = { read = LuaUse, write = LuaValue|nil, span = LuaSpan } }
    methods.add_method_mut("use_obj", |_, this, (tbl, span): (LuaTable, LuaSpan)| {
        let fields = parse_use_obj_fields(&tbl)?;
        let u = this.0.new_use(UTypeHead::UObj { fields }, span.0, None);
        Ok(LuaUse(u))
    });

    // core:use_case(cases_table, wildcard_or_nil, span) -> LuaUse
    // cases_table: { tag = LuaUse }
    methods.add_method_mut(
        "use_case",
        |_, this, (tbl, wildcard, span): (LuaTable, Option<LuaUse>, LuaSpan)| {
            let mut cases = StringIdMap::default();
            for pair in tbl.pairs::<String, LuaAnyUserData>() {
                let (tag, use_ud) = pair?;
                let u = use_ud.borrow::<LuaUse>()?;
                cases.insert(ustr::ustr(&tag), u.0);
            }
            let u = this.0.new_use(
                UTypeHead::UCase {
                    cases,
                    wildcard: wildcard.map(|w| w.0),
                },
                span.0,
                None,
            );
            Ok(LuaUse(u))
        },
    );

    // core:use_intersection(uses_array, span) -> LuaUse
    methods.add_method_mut(
        "use_intersection",
        |_, this, (tbl, span): (LuaTable, LuaSpan)| {
            let uses: Vec<alsub::Use> = tbl
                .sequence_values::<LuaAnyUserData>()
                .map(|ud| {
                    let ud = ud?;
                    let u = ud.borrow::<LuaUse>()?;
                    Ok(u.0)
                })
                .collect::<LuaResult<Vec<_>>>()?;
            let u = this
                .0
                .new_use(UTypeHead::UIntersection(uses.into()), span.0, None);
            Ok(LuaUse(u))
        },
    );

    // core:use_bot(span) -> LuaUse
    methods.add_method_mut("use_bot", |_, this, span: LuaSpan| {
        let u = this.0.new_use(UTypeHead::UBot, span.0, None);
        Ok(LuaUse(u))
    });

    // core:use_abstract(type_ctor_ind, span) -> LuaUse
    methods.add_method_mut(
        "use_abstract",
        |_, this, (ty, span): (LuaTypeCtorInd, LuaSpan)| {
            let u = this.0.simple_use(ty.0, span.0);
            Ok(LuaUse(u))
        },
    );

    // core:use_poly(poly_data, use_, poison, span) -> LuaUse
    methods.add_method_mut(
        "use_poly",
        |_, this, (poly, use_, poison, span): (LuaPolyHeadData, LuaUse, bool, LuaSpan)| {
            let deps = PolyDeps::single(poly.0.loc);
            let u = this.0.new_use(
                UTypeHead::UPolyHead(poly.0.clone(), use_.0, poison),
                span.0,
                Some(deps),
            );
            Ok(LuaUse(u))
        },
    );

    // core:set_use(ph, head, span) — fill a use placeholder
    methods.add_method_mut(
        "set_use",
        |_, this, (ph, head, span): (LuaUse, LuaUTypeHeadWrapper, LuaSpan)| {
            this.0.set_use(ph.0, head.0.clone(), span.0, None);
            Ok(())
        },
    );
}

// ---- Helper: wrap type heads as UserData so they can be passed to set_val/set_use ----

#[derive(Clone)]
pub struct LuaVTypeHeadWrapper(pub VTypeHead);
impl LuaUserData for LuaVTypeHeadWrapper {}
impl FromLua for LuaVTypeHeadWrapper {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> LuaResult<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaVTypeHeadWrapper".to_string(),
                message: None,
            }),
        }
    }
}

#[derive(Clone)]
pub struct LuaUTypeHeadWrapper(pub UTypeHead);
impl LuaUserData for LuaUTypeHeadWrapper {}
impl FromLua for LuaUTypeHeadWrapper {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> LuaResult<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaUTypeHeadWrapper".to_string(),
                message: None,
            }),
        }
    }
}

// ---- Parse table to field maps ----

fn parse_val_obj_fields(
    tbl: &LuaTable,
) -> LuaResult<StringIdMap<(alsub::Value, Option<alsub::Use>, alsub::Span)>> {
    let mut fields = StringIdMap::default();
    for pair in tbl.pairs::<String, LuaTable>() {
        let (name, field_tbl) = pair?;
        let read_ud: LuaAnyUserData = field_tbl.get("read")?;
        let read = read_ud.borrow::<LuaValue>()?;
        let write: Option<alsub::Use> = match field_tbl.get::<Option<LuaAnyUserData>>("write")? {
            Some(ud) => Some(ud.borrow::<LuaUse>()?.0),
            None => None,
        };
        let span_ud: LuaAnyUserData = field_tbl.get("span")?;
        let span = span_ud.borrow::<LuaSpan>()?;
        fields.insert(ustr::ustr(&name), (read.0, write, span.0));
    }
    Ok(fields)
}

fn parse_use_obj_fields(
    tbl: &LuaTable,
) -> LuaResult<StringIdMap<(alsub::Use, Option<alsub::Value>, alsub::Span)>> {
    let mut fields = StringIdMap::default();
    for pair in tbl.pairs::<String, LuaTable>() {
        let (name, field_tbl) = pair?;
        let read_ud: LuaAnyUserData = field_tbl.get("read")?;
        let read = read_ud.borrow::<LuaUse>()?;
        let write: Option<alsub::Value> = match field_tbl.get::<Option<LuaAnyUserData>>("write")? {
            Some(ud) => Some(ud.borrow::<LuaValue>()?.0),
            None => None,
        };
        let span_ud: LuaAnyUserData = field_tbl.get("span")?;
        let span = span_ud.borrow::<LuaSpan>()?;
        fields.insert(ustr::ustr(&name), (read.0, write, span.0));
    }
    Ok(fields)
}
