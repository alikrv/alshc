pub mod codegen;
pub mod control_flow;
pub mod lexer;
pub mod parser;
pub mod parser2;
// Re-export commonly used types
pub use codegen::CodeGen;
pub use control_flow::{
    CompareOp, Condition, ControlFlowParser, Environment, FunctionDef, Statement, Value,
};
//pub use parser::{parse_line, parse_pipeline, Command, Pipeline, PipelineMode};
