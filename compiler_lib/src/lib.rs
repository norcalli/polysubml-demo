#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod codegen;
mod grammar;
mod js;
pub mod lua;
mod lua_codegen;

// Re-export alsub modules so grammar.lalr's `use super::ast` and `use super::spans` resolve
pub use alsub::ast;
pub use alsub::spans;

use lalrpop_util::ParseError;

use self::codegen::ModuleBuilder;
use self::grammar::ScriptParser;
use self::lua_codegen::ModuleBuilder as LuaModuleBuilder;
use alsub::spans::{SpanMaker, SpanManager, SpannedError};
use alsub::typeck::TypeckState;

fn convert_parse_error<T: std::fmt::Display>(
    mut sm: SpanMaker,
    e: ParseError<usize, T, (&'static str, alsub::spans::Span)>,
) -> SpannedError {
    match e {
        ParseError::InvalidToken { location } => {
            SpannedError::new1("SyntaxError: Invalid token", sm.span(location, location))
        }
        ParseError::UnrecognizedEof { location, expected } => SpannedError::new1(
            format!(
                "SyntaxError: Unexpected end of input.\nNote: expected tokens: [{}]\nParse error occurred here:",
                expected.join(", ")
            ),
            sm.span(location, location),
        ),
        ParseError::UnrecognizedToken { token, expected } => SpannedError::new1(
            format!(
                "SyntaxError: Unexpected token {}\nNote: expected tokens: [{}]\nParse error occurred here:",
                token.1,
                expected.join(", ")
            ),
            sm.span(token.0, token.2),
        ),
        ParseError::ExtraToken { token } => {
            SpannedError::new1("SyntaxError: Unexpected extra token", sm.span(token.0, token.2))
        }
        ParseError::User { error: (msg, span) } => SpannedError::new1(msg, span),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilationResult {
    Success(String), // Contains compiled JS code
    Error(String),   // Contains error message
}
impl std::fmt::Display for CompilationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilationResult::Success(js_code) => write!(f, "SUCCESS\n{}", js_code),
            CompilationResult::Error(error_msg) => write!(f, "ERROR\n{}", error_msg),
        }
    }
}

pub struct State {
    parser: ScriptParser,
    spans: SpanManager,

    checker: TypeckState,
    compiler: ModuleBuilder,
    lua_compiler: LuaModuleBuilder,
}
impl State {
    pub fn new() -> Self {
        let checker = TypeckState::new();

        State {
            parser: ScriptParser::new(),
            spans: SpanManager::default(),

            checker,
            compiler: ModuleBuilder::new(),
            lua_compiler: LuaModuleBuilder::new(),
        }
    }

    fn process_sub(&mut self, source: &str) -> Result<String, SpannedError> {
        let span_maker = self.spans.add_source(source.to_owned());
        let mut ctx = ast::ParserContext { span_maker };

        let ast = self
            .parser
            .parse(&mut ctx, source)
            .map_err(|e| convert_parse_error(ctx.span_maker, e))?;
        let _t = self.checker.check_script(&ast)?;

        let mut ctx = codegen::Context(&mut self.compiler);
        let js_ast = codegen::compile_script(&mut ctx, &ast);
        Ok(js_ast.to_source())
    }

    pub fn process(&mut self, source: &str) -> CompilationResult {
        let res = self.process_sub(source);
        match res {
            Ok(s) => CompilationResult::Success(s),
            Err(e) => CompilationResult::Error(e.print(&self.spans)),
        }
    }

    fn process_sub_lua(&mut self, source: &str) -> Result<String, SpannedError> {
        let span_maker = self.spans.add_source(source.to_owned());
        let mut ctx = ast::ParserContext { span_maker };

        let ast = self
            .parser
            .parse(&mut ctx, source)
            .map_err(|e| convert_parse_error(ctx.span_maker, e))?;
        let _t = self.checker.check_script(&ast)?;

        let mut ctx = lua_codegen::Context(&mut self.lua_compiler);
        let lua_ast = lua_codegen::compile_script(&mut ctx, &ast);
        Ok(lua_ast.to_source())
    }

    pub fn process_lua(&mut self, source: &str) -> CompilationResult {
        let res = self.process_sub_lua(source);
        match res {
            Ok(s) => CompilationResult::Success(s),
            Err(e) => CompilationResult::Error(e.print(&self.spans)),
        }
    }

    pub fn reset(&mut self) {
        self.checker = TypeckState::new();
        self.compiler = ModuleBuilder::new();
        self.lua_compiler = LuaModuleBuilder::new();
    }

    // ---- Split pipeline methods ----

    pub fn parse(&mut self, source: &str) -> Result<Vec<ast::Statement>, SpannedError> {
        let span_maker = self.spans.add_source(source.to_owned());
        let mut ctx = ast::ParserContext { span_maker };
        self.parser
            .parse(&mut ctx, source)
            .map_err(|e| convert_parse_error(ctx.span_maker, e))
    }

    pub fn check(&mut self, ast: &[ast::Statement]) -> Result<(), SpannedError> {
        self.checker.check_script(ast)
    }

    pub fn generate_lua(&mut self, ast: &[ast::Statement]) -> String {
        let mut ctx = lua_codegen::Context(&mut self.lua_compiler);
        let lua_ast = lua_codegen::compile_script(&mut ctx, ast);
        lua_ast.to_source()
    }

    pub fn generate_js(&mut self, ast: &[ast::Statement]) -> String {
        let mut ctx = codegen::Context(&mut self.compiler);
        let js_ast = codegen::compile_script(&mut ctx, ast);
        js_ast.to_source()
    }

    pub fn format_error(&self, e: &SpannedError) -> String {
        e.print(&self.spans)
    }
}
