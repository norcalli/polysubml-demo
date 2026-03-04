use mlua::prelude::*;

pub mod ast;
pub mod compiler;
pub mod core;
pub mod types;

use crate::compiler::LuaCompiler;
use crate::core::*;

#[mlua::lua_module]
fn alsub(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;

    // Core — TypeCheckerCore constructor
    exports.set(
        "Core",
        lua.create_function(|_, ()| {
            Ok(LuaCore(alsub::TypeCheckerCore::new()))
        })?,
    )?;

    // SpanManager constructor
    exports.set(
        "SpanManager",
        lua.create_function(|_, ()| {
            Ok(LuaSpanManager(std::rc::Rc::new(std::cell::RefCell::new(
                alsub::SpanManager::default(),
            ))))
        })?,
    )?;

    // Compiler constructor (high-level pipeline)
    exports.set(
        "Compiler",
        lua.create_function(|_, ()| {
            Ok(LuaCompiler(compiler_lib::State::new()))
        })?,
    )?;

    // Bindings constructor
    exports.set(
        "Bindings",
        lua.create_function(|_, ()| {
            Ok(LuaBindings(alsub::Bindings {
                vars: alsub::ast::StringIdMap::default(),
                types: alsub::ast::StringIdMap::default(),
                scopelvl: alsub::ScopeLvl(0),
            }))
        })?,
    )?;

    // TypeckState constructor
    exports.set(
        "TypeckState",
        lua.create_function(|_, ()| Ok(LuaTypeckState(alsub::TypeckState::new())))?,
    )?;

    // ScopeLvl constructor from integer
    exports.set(
        "ScopeLvl",
        lua.create_function(|_, n: u32| Ok(LuaScopeLvl(alsub::ScopeLvl(n))))?,
    )?;

    // SourceLoc constructor from span
    exports.set(
        "SourceLoc",
        lua.create_function(core::lua_new_source_loc)?,
    )?;

    // VarSpec constructor
    exports.set(
        "VarSpec",
        lua.create_function(core::lua_new_var_spec)?,
    )?;

    // PolyHeadData constructor
    exports.set(
        "PolyHeadData",
        lua.create_function(core::lua_new_poly_head_data)?,
    )?;

    Ok(exports)
}
