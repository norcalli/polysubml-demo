use mlua::prelude::*;

use compiler_lib::State;

use crate::ast::LuaScript;

pub struct LuaCompiler(pub State);
impl LuaUserData for LuaCompiler {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // process_lua(source) -> code, nil OR nil, error
        methods.add_method_mut("process_lua", |_, this, source: String| {
            match this.0.process_lua(&source) {
                compiler_lib::CompilationResult::Success(code) => Ok((Some(code), None::<String>)),
                compiler_lib::CompilationResult::Error(err) => Ok((None, Some(err))),
            }
        });

        // process_js(source) -> code, nil OR nil, error
        methods.add_method_mut("process_js", |_, this, source: String| {
            match this.0.process(&source) {
                compiler_lib::CompilationResult::Success(code) => Ok((Some(code), None::<String>)),
                compiler_lib::CompilationResult::Error(err) => Ok((None, Some(err))),
            }
        });

        // parse(source) -> LuaScript, nil OR nil, error
        methods.add_method_mut("parse", |_, this, source: String| {
            match this.0.parse(&source) {
                Ok(ast) => Ok((Some(LuaScript(ast)), None::<String>)),
                Err(e) => {
                    let msg = this.0.format_error(&e);
                    Ok((None, Some(msg)))
                }
            }
        });

        // check(script) -> true, nil OR nil, error
        methods.add_method_mut("check", |_, this, script: LuaAnyUserData| {
            let script = script.borrow::<LuaScript>()?;
            match this.0.check(&script.0) {
                Ok(()) => Ok((Some(true), None::<String>)),
                Err(e) => {
                    let msg = this.0.format_error(&e);
                    Ok((None, Some(msg)))
                }
            }
        });

        // generate_lua(script) -> code
        methods.add_method_mut("generate_lua", |_, this, script: LuaAnyUserData| {
            let script = script.borrow::<LuaScript>()?;
            Ok(this.0.generate_lua(&script.0))
        });

        // generate_js(script) -> code
        methods.add_method_mut("generate_js", |_, this, script: LuaAnyUserData| {
            let script = script.borrow::<LuaScript>()?;
            Ok(this.0.generate_js(&script.0))
        });

        // reset()
        methods.add_method_mut("reset", |_, this, ()| {
            this.0.reset();
            Ok(())
        });
    }
}
