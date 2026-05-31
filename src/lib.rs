pub mod parser;
pub mod control_flow;
pub mod codegen;

// Re-export commonly used types
pub use parser::{Command, Pipeline, PipelineMode, parse_line, parse_pipeline};
pub use control_flow::{
    Value, Statement, Condition, CompareOp, ControlFlowParser, FunctionDef, Environment,
};
pub use codegen::CodeGen;
