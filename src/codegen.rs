// src/codegen.rs
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue, IntValue};
use inkwell::basic_block::BasicBlock;
use inkwell::AddressSpace;
use std::collections::HashMap;

use crate::control_flow::{Expression, Statement, Condition};
// For runtime symbols
extern crate libc;

pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, PointerValue<'ctx>>,
    variable_types: HashMap<String, String>,
    #[allow(dead_code)]
    functions: HashMap<String, FunctionValue<'ctx>>,
    just_run: bool,
    has_main: bool,
    main_function_name: Option<String>,
    loop_end_block: Option<BasicBlock<'ctx>>,
    loop_start_block: Option<BasicBlock<'ctx>>,
    needs_return: bool,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn new(context: &'ctx Context, just_run: bool) -> Self {
        let module = context.create_module("alsh");
        let builder = context.create_builder();

        CodeGen {
            context,
            module,
            builder,
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            functions: HashMap::new(),
            just_run,
            has_main: false,
            main_function_name: None,
            loop_end_block: None,
            loop_start_block: None,
            needs_return: true,
        }
    }

    pub fn set_has_main(&mut self, has_main: bool) {
        self.has_main = has_main;
    }

    pub fn set_main_function_name(&mut self, name: Option<String>) {
        self.main_function_name = name;
    }

    pub fn generate(&mut self, statements: &[Statement]) -> Result<(), String> {
        // First pass: collect function definitions and separate from top-level code
        let mut top_level_statements = Vec::new();
        let mut functions_def: HashMap<String, Vec<Statement>> = HashMap::new();

        for stmt in statements {
            match stmt {
                Statement::FunctionDef { name, params: _, body } => {
                    functions_def.insert(name.clone(), body.clone());
                }
                _ => {
                    top_level_statements.push(stmt.clone());
                }
            }
        }

        // Generate function definitions
        for (name, body) in &functions_def {
            self.generate_function(name, &[], body)?;
        }

        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
        let argv_type = self.context.ptr_type(AddressSpace::default());
        // alsh_str type: { i64, i64, i8* }
        let i64_type = self.context.i64_type();
        let _alsh_str_type = self.context.struct_type(&[i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);

        if self.has_main {
            // @main is set: the marked function is already generated as "main"
            if self.main_function_name.is_none() {
                return Err("@main directive found but no function follows it".to_string());
            }
            // The main function is already generated, no need to create a wrapper
        } else if self.just_run {
            // Generate main() that executes top-level code
            let main_fn_type = i32_type.fn_type(&[i32_type.into(), argv_type.into()], false);
            let main_fn = self.module.add_function("main", main_fn_type, None);
            let entry_bb = self.context.append_basic_block(main_fn, "entry");
            self.builder.position_at_end(entry_bb);

            self.variables.clear();
            self.variable_types.clear();
            self.needs_return = true;

            for stmt in &top_level_statements {
                self.generate_statement(stmt).map_err(|e| format!("Codegen error: {}", e))?;
            }

            // Return 0 if we haven't already returned
            if self.needs_return {
                let _ = self.builder.build_return(Some(&i32_type.const_int(0, false)));
            }
        } else if !top_level_statements.is_empty() {
            return Err("Top-level code found but no @main or @justrunit directive. Add @main above a function or use @justrunit.".to_string());
        } else {
            // No code at all: create empty main
            let main_fn_type = i32_type.fn_type(&[i32_type.into(), argv_type.into()], false);
            let main_fn = self.module.add_function("main", main_fn_type, None);
            let entry_bb = self.context.append_basic_block(main_fn, "entry");
            self.builder.position_at_end(entry_bb);
            let _ = self.builder.build_return(Some(&i32_type.const_int(0, false)));
        }

        Ok(())
    }

    fn generate_function(&mut self, name: &str, _params: &[String], body: &[Statement]) -> Result<(), String> {
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let _alsh_str_type = self.context.struct_type(&[i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);
        let argv_type = self.context.ptr_type(AddressSpace::default());

        // If this function is marked with @main, name it "main"
        let actual_name = if self.has_main && self.main_function_name.as_ref().map(|n| n == name).unwrap_or(false) {
            "main"
        } else {
            name
        };

        let fn_type = if actual_name == "main" {
            i32_type.fn_type(&[i32_type.into(), argv_type.into()], false)
        } else {
            i32_type.fn_type(&[], false)
        };

        let func = self.module.add_function(actual_name, fn_type, None);

        let entry_bb = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_bb);

        // Save current variable state
        let saved_vars = self.variables.clone();
        let saved_types = self.variable_types.clone();

        self.variables.clear();
        self.variable_types.clear();
        self.needs_return = true;

        // Generate function body
        for stmt in body {
            self.generate_statement(stmt).map_err(|e| format!("Codegen error in function {}: {}", name, e))?;
        }

        // Return 0 if we haven't already returned
        if self.needs_return {
            let _ = self.builder.build_return(Some(&i32_type.const_int(0, false)));
        }

        // Restore variable state
        self.variables = saved_vars;
        self.variable_types = saved_types;

        self.functions.insert(name.to_string(), func);
        Ok(())
    }

    fn generate_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let { name, value } => {
                let val = self.generate_expression(value)?;
                let var = self.builder.build_alloca(val.get_type(), name).map_err(|e| e.to_string())?;
                self.builder.build_store(var, val).map_err(|e| e.to_string())?;

                // Track the variable type
                let var_type = match val {
                    BasicValueEnum::PointerValue(_) => "pointer",
                    BasicValueEnum::IntValue(_) => "i32",
                    BasicValueEnum::FloatValue(_) => "f64",
                    BasicValueEnum::ArrayValue(_) => "array",
                    BasicValueEnum::StructValue(_) => "struct",
                    BasicValueEnum::VectorValue(_) => "vector",
                    BasicValueEnum::ScalableVectorValue(_) => "scalable_vector",
                };

                self.variables.insert(name.clone(), var);
                self.variable_types.insert(name.clone(), var_type.to_string());
            }
            Statement::Command(cmd) => {
                // For now, just print the command
                let printf_fn = self.get_printf_fn();
                let str_val = self.context.const_string(cmd.as_bytes(), false);
                let global_str = self.module.add_global(str_val.get_type(), None, "cmd");
                global_str.set_initializer(&str_val);
                let str_ptr = self.builder.build_pointer_cast(
                    global_str.as_pointer_value(),
                    self.context.ptr_type(AddressSpace::default()),
                    "str_ptr",
                ).map_err(|e| e.to_string())?;
                self.builder.build_call(printf_fn, &[str_ptr.into()], "call").map_err(|e| e.to_string())?;
            }
            Statement::Expression(expr) => {
                let _ = self.generate_expression(expr)?;
            }
            Statement::While { condition, body } => {
                self.generate_while(condition, body)?;
            }
            Statement::If { condition, then_block, elif_blocks, else_block } => {
                self.generate_if(condition, then_block, elif_blocks, else_block)?;
            }
            Statement::For { var, items, body } => {
                self.generate_for(var, items, body)?;
            }
            Statement::Foreach { var, iterable, body } => {
                self.generate_foreach(var, iterable, body)?;
            }
            Statement::Loop { count, interval, body } => {
                self.generate_loop(count, interval, body)?;
            }
            Statement::Break { .. } => {
                if let Some(end_block) = self.loop_end_block {
                    self.builder.build_unconditional_branch(end_block).map_err(|e| e.to_string())?;
                } else {
                    return Err("break statement outside of loop".to_string());
                }
            }
            Statement::Continue => {
                if let Some(start_block) = self.loop_start_block {
                    self.builder.build_unconditional_branch(start_block).map_err(|e| e.to_string())?;
                } else {
                    return Err("continue statement outside of loop".to_string());
                }
            }
            Statement::Return { value } => {
                if let Some(ret_expr) = value {
                    let ret_val = self.generate_expression(ret_expr)?;
                    let return_value: BasicValueEnum<'ctx> = match ret_val {
                        BasicValueEnum::IntValue(i) => i.into(),
                        BasicValueEnum::FloatValue(f) => self.builder.build_float_to_signed_int(f, self.context.i32_type(), "ret_cast").map_err(|e| e.to_string())?.into(),
                        BasicValueEnum::PointerValue(p) => self.builder.build_ptr_to_int(p, self.context.i32_type(), "ret_ptr_cast").map_err(|e| e.to_string())?.into(),
                        _ => self.context.i32_type().const_int(0, false).into(),
                    };
                    self.builder.build_return(Some(&return_value)).map_err(|e| e.to_string())?;
                } else {
                    let zero = self.context.i32_type().const_int(0, false);
                    self.builder.build_return(Some(&zero)).map_err(|e| e.to_string())?;
                }
                self.needs_return = false;
            }
            _ => {
                // Other statement types (Try, FunctionDef, etc.) will be handled later
            }
        }
        Ok(())
    }

    fn generate_while(&mut self, condition: &Condition, body: &[Statement]) -> Result<(), String> {
        // Get the current function and append basic blocks
        let current_fn = self.builder.get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No function to insert into")?;

        let cond_block = self.context.append_basic_block(current_fn, "while_cond");
        let body_block = self.context.append_basic_block(current_fn, "while_body");
        let end_block = self.context.append_basic_block(current_fn, "while_end");

        // Branch to condition check
        self.builder.build_unconditional_branch(cond_block).map_err(|e| e.to_string())?;

        // Generate condition block
        self.builder.position_at_end(cond_block);
        let cond_val = self.generate_condition(condition)?;
        self.builder.build_conditional_branch(cond_val, body_block, end_block).map_err(|e| e.to_string())?;

        // Generate body block
        self.builder.position_at_end(body_block);
        for stmt in body {
            self.generate_statement(stmt)?;
        }
        self.builder.build_unconditional_branch(cond_block).map_err(|e| e.to_string())?;

        // Set position to end block for next statements
        self.builder.position_at_end(end_block);

        Ok(())
    }

    fn generate_if(&mut self, condition: &Condition, then_block: &[Statement],
                   elif_blocks: &[(Condition, Vec<Statement>)], else_block: &Option<Vec<Statement>>) -> Result<(), String> {
        let current_fn = self.builder.get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No function to insert into")?;

        let then_bb = self.context.append_basic_block(current_fn, "if_then");
        let end_bb = self.context.append_basic_block(current_fn, "if_end");

        // Generate condition and branch
        let cond_val = self.generate_condition(condition)?;

        // Determine else branch
        let else_bb = if !elif_blocks.is_empty() {
            self.context.append_basic_block(current_fn, "elif_block")
        } else if else_block.is_some() {
            self.context.append_basic_block(current_fn, "else_block")
        } else {
            end_bb
        };

        self.builder.build_conditional_branch(cond_val, then_bb, else_bb).map_err(|e| e.to_string())?;

        // Generate then block
        self.builder.position_at_end(then_bb);
        for stmt in then_block {
            self.generate_statement(stmt)?;
        }
        self.builder.build_unconditional_branch(end_bb).map_err(|e| e.to_string())?;

        // Generate elif/else blocks
        if !elif_blocks.is_empty() {
            self.builder.position_at_end(else_bb);
            self.generate_elif_chain(current_fn, end_bb, elif_blocks, else_block)?;
        } else if let Some(else_body) = else_block {
            self.builder.position_at_end(else_bb);
            for stmt in else_body {
                self.generate_statement(stmt)?;
            }
            self.builder.build_unconditional_branch(end_bb).map_err(|e| e.to_string())?;
        }

        self.builder.position_at_end(end_bb);
        Ok(())
    }

    fn generate_elif_chain(&mut self, current_fn: FunctionValue<'ctx>, end_bb: BasicBlock<'ctx>,
                          elif_blocks: &[(Condition, Vec<Statement>)], else_block: &Option<Vec<Statement>>) -> Result<(), String> {
        if elif_blocks.is_empty() {
            if let Some(else_body) = else_block {
                for stmt in else_body {
                    self.generate_statement(stmt)?;
                }
                self.builder.build_unconditional_branch(end_bb).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

        let (elif_cond, elif_body) = &elif_blocks[0];
        let remaining_elifs = &elif_blocks[1..];

        let elif_body_bb = self.context.append_basic_block(current_fn, "elif_body");
        let next_block = if !remaining_elifs.is_empty() {
            self.context.append_basic_block(current_fn, "next_elif")
        } else if else_block.is_some() {
            self.context.append_basic_block(current_fn, "else_block")
        } else {
            end_bb
        };

        // Generate condition check
        let elif_cond_val = self.generate_condition(elif_cond)?;
        self.builder.build_conditional_branch(elif_cond_val, elif_body_bb, next_block).map_err(|e| e.to_string())?;

        // Generate elif body
        self.builder.position_at_end(elif_body_bb);
        for stmt in elif_body {
            self.generate_statement(stmt)?;
        }
        self.builder.build_unconditional_branch(end_bb).map_err(|e| e.to_string())?;

        // Recursively handle remaining elif/else blocks
        self.builder.position_at_end(next_block);
        self.generate_elif_chain(current_fn, end_bb, remaining_elifs, else_block)?;

        Ok(())
    }

    fn generate_for(&mut self, var: &str, items: &[Expression], body: &[Statement]) -> Result<(), String> {
        let int_type = self.context.i32_type();
        let current_fn = self.builder.get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No function to insert into")?;

        let loop_block = self.context.append_basic_block(current_fn, "for_loop");
        let end_block = self.context.append_basic_block(current_fn, "for_end");

        // For simple numeric range for now
        if items.len() == 1 {
            if let Expression::Literal(crate::control_flow::Value::Number(n)) = items[0] {
                // Create loop counter
                let counter = self.builder.build_alloca(int_type, var).map_err(|e| e.to_string())?;
                self.builder.build_store(counter, int_type.const_int(0, false)).map_err(|e| e.to_string())?;
                self.variables.insert(var.to_string(), counter);
                self.variable_types.insert(var.to_string(), "i32".to_string());

                self.builder.build_unconditional_branch(loop_block).map_err(|e| e.to_string())?;

                self.builder.position_at_end(loop_block);
                let counter_val = self.builder.build_load(int_type, counter, "counter").map_err(|e| e.to_string())?;
                let limit = int_type.const_int(n as u64, false);
                let cond = self.builder.build_int_compare(
                    inkwell::IntPredicate::SLT,
                    counter_val.into_int_value(),
                    limit,
                    "for_cond",
                ).map_err(|e| e.to_string())?;

                let body_bb = self.context.append_basic_block(current_fn, "for_body");
                self.builder.build_conditional_branch(cond, body_bb, end_block).map_err(|e| e.to_string())?;

                self.builder.position_at_end(body_bb);
                for stmt in body {
                    self.generate_statement(stmt)?;
                }

                // Increment counter
                let counter_val = self.builder.build_load(int_type, counter, "counter").map_err(|e| e.to_string())?;
                let incremented = self.builder.build_int_add(
                    counter_val.into_int_value(),
                    int_type.const_int(1, false),
                    "inc",
                ).map_err(|e| e.to_string())?;
                self.builder.build_store(counter, incremented).map_err(|e| e.to_string())?;
                self.builder.build_unconditional_branch(loop_block).map_err(|e| e.to_string())?;
            }
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    fn generate_foreach(&mut self, _var: &str, _iterable: &Expression, _body: &[Statement]) -> Result<(), String> {
        // For now, defer foreach implementation as it requires array handling
        // TODO: Implement foreach with proper array/list iteration
        Ok(())
    }

    fn generate_loop(&mut self, count: &Option<u64>, interval: &Option<u64>, body: &[Statement]) -> Result<(), String> {
        let int_type = self.context.i32_type();
        let current_fn = self.builder.get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No function to insert into")?;

        let loop_block = self.context.append_basic_block(current_fn, "loop");
        let end_block = self.context.append_basic_block(current_fn, "loop_end");

        // Store previous loop blocks for nested loops
        let prev_end = self.loop_end_block;
        let prev_start = self.loop_start_block;

        self.loop_end_block = Some(end_block.into());
        self.loop_start_block = Some(loop_block.into());

        if let Some(loop_count) = count {
            // Loop with count
            let counter = self.builder.build_alloca(int_type, "_loop_counter").map_err(|e| e.to_string())?;
            self.builder.build_store(counter, int_type.const_int(0, false)).map_err(|e| e.to_string())?;

            self.builder.build_unconditional_branch(loop_block).map_err(|e| e.to_string())?;

            self.builder.position_at_end(loop_block);
            let counter_val = self.builder.build_load(int_type, counter, "counter").map_err(|e| e.to_string())?;
            let limit = int_type.const_int(*loop_count as u64, false);
            let cond = self.builder.build_int_compare(
                inkwell::IntPredicate::SLT,
                counter_val.into_int_value(),
                limit,
                "loop_cond",
            ).map_err(|e| e.to_string())?;

            let body_bb = self.context.append_basic_block(current_fn, "loop_body");
            self.builder.build_conditional_branch(cond, body_bb, end_block).map_err(|e| e.to_string())?;

            self.builder.position_at_end(body_bb);
            for stmt in body {
                self.generate_statement(stmt)?;
            }

            // Increment counter
            let counter_val = self.builder.build_load(int_type, counter, "counter").map_err(|e| e.to_string())?;
            let incremented = self.builder.build_int_add(
                counter_val.into_int_value(),
                int_type.const_int(1, false),
                "inc",
            ).map_err(|e| e.to_string())?;
            self.builder.build_store(counter, incremented).map_err(|e| e.to_string())?;
            self.builder.build_unconditional_branch(loop_block).map_err(|e| e.to_string())?;
        } else if interval.is_some() {
            // Loop with interval - similar to count but with sleep
            // For now, just do a simple loop (sleep not yet implemented)
            self.builder.build_unconditional_branch(loop_block).map_err(|e| e.to_string())?;
            self.builder.position_at_end(loop_block);

            for stmt in body {
                self.generate_statement(stmt)?;
            }

            self.builder.build_unconditional_branch(loop_block).map_err(|e| e.to_string())?;
        } else {
            // Infinite loop
            self.builder.build_unconditional_branch(loop_block).map_err(|e| e.to_string())?;
            self.builder.position_at_end(loop_block);

            for stmt in body {
                self.generate_statement(stmt)?;
            }

            self.builder.build_unconditional_branch(loop_block).map_err(|e| e.to_string())?;
        }

        self.builder.position_at_end(end_block);

        // Restore previous loop blocks
        self.loop_end_block = prev_end;
        self.loop_start_block = prev_start;

        Ok(())
    }

    fn generate_condition(&mut self, condition: &Condition) -> Result<IntValue<'ctx>, String> {
        use crate::control_flow::Condition;
        use inkwell::IntPredicate;

        match condition {
            Condition::Compare(left, op, right) => {
                let left_val = self.generate_expression(left)?.into_int_value();
                let right_val = self.generate_expression(right)?.into_int_value();

                let predicate = match op {
                    crate::control_flow::CompareOp::Lt => IntPredicate::SLT,
                    crate::control_flow::CompareOp::Gt => IntPredicate::SGT,
                    crate::control_flow::CompareOp::Le => IntPredicate::SLE,
                    crate::control_flow::CompareOp::Ge => IntPredicate::SGE,
                    crate::control_flow::CompareOp::Eq => IntPredicate::EQ,
                    crate::control_flow::CompareOp::Ne => IntPredicate::NE,
                };

                self.builder.build_int_compare(predicate, left_val, right_val, "cmp").map_err(|e| e.to_string())
            }
            Condition::Command(expr) => {
                let val = self.generate_expression(expr)?.into_int_value();
                Ok(val)
            }
            _ => Err("Unsupported condition type".to_string()),
        }
    }

    fn generate_expression(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            Expression::Literal(val) => {
                use crate::control_flow::Value;
                match val {
                    Value::Number(n) => Ok(self.context.i32_type().const_int(*n as u64, false).into()),
                    Value::String(s) => {
                        // Create a global C string and then create a global alsh_str pointing at it
                        let bytes = s.as_bytes();
                        let str_val = self.context.const_string(bytes, true);
                        let global_chars = self.module.add_global(str_val.get_type(), None, "str_chars");
                        global_chars.set_initializer(&str_val);
                        // alsh_str instance as a global struct
                        let i64_type = self.context.i64_type();
                        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
                        let alsh_str_ty = self.context.struct_type(&[i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);
                        let alsh_str_global = self.module.add_global(alsh_str_ty, None, "str_obj");
                        // Build initializer: { len, cap, ptr }
                        let len = i64_type.const_int(bytes.len() as u64, false);
                        let cap = i64_type.const_int(bytes.len() as u64, false);
                        let ptr = global_chars.as_pointer_value();
                        let init = alsh_str_ty.const_named_struct(&[len.into(), cap.into(), ptr.into()]);
                        alsh_str_global.set_initializer(&init);
                        Ok(alsh_str_global.as_pointer_value().into())
                    }
                    _ => Err("Unsupported literal type".to_string()),
                }
            }
            Expression::Variable(name) => {
                if let Some(var) = self.variables.get(name) {
                    // Load based on the variable's type
                    if let Some(var_type) = self.variable_types.get(name) {
                        match var_type.as_str() {
                            "pointer" => {
                                let load = self.builder.build_load(self.context.ptr_type(AddressSpace::default()), *var, name).map_err(|e| e.to_string())?;
                                Ok(load)
                            }
                            "i32" => {
                                let load = self.builder.build_load(self.context.i32_type(), *var, name).map_err(|e| e.to_string())?;
                                Ok(load)
                            }
                            _ => {
                                let load = self.builder.build_load(self.context.i64_type(), *var, name).map_err(|e| e.to_string())?;
                                Ok(load)
                            }
                        }
                    } else {
                        Err(format!("Unknown variable type for: {}", name))
                    }
                } else {
                    Err(format!("Undefined variable: {}", name))
                }
            }
            Expression::FunctionCall(name, args) => {
                self.generate_function_call(name, args)
            }
            Expression::CCall(name, args) => {
                self.generate_c_call(name, args)
            }
            Expression::BinaryOp(left, op, right) => {
                self.generate_binary_op(left, op, right)
            }
            _ => Err("Unsupported expression".to_string()),
        }
    }

    fn generate_binary_op(&mut self, left: &Expression, op: &crate::control_flow::BinOp, right: &Expression) -> Result<BasicValueEnum<'ctx>, String> {
        let left_val = self.generate_expression(left)?;
        let right_val = self.generate_expression(right)?;

        // Extract i64 values
        let left_int = match left_val {
            BasicValueEnum::IntValue(iv) => iv,
            _ => return Err("Binary operations require integer operands".to_string()),
        };

        let right_int = match right_val {
            BasicValueEnum::IntValue(iv) => iv,
            _ => return Err("Binary operations require integer operands".to_string()),
        };

        let result = match op {
            crate::control_flow::BinOp::Add => {
                self.builder.build_int_add(left_int, right_int, "add").map_err(|e| e.to_string())?
            }
            crate::control_flow::BinOp::Sub => {
                self.builder.build_int_sub(left_int, right_int, "sub").map_err(|e| e.to_string())?
            }
            crate::control_flow::BinOp::Mul => {
                self.builder.build_int_mul(left_int, right_int, "mul").map_err(|e| e.to_string())?
            }
            crate::control_flow::BinOp::Div => {
                self.builder.build_int_signed_div(left_int, right_int, "div").map_err(|e| e.to_string())?
            }
            _ => return Err(format!("Unsupported binary operation: {:?}", op)),
        };

        Ok(result.into())
    }

    fn generate_function_call(&mut self, name: &str, args: &[Expression]) -> Result<BasicValueEnum<'ctx>, String> {
        if name == "std::println" {
            // Implement std::println as printf
            let printf_fn = self.get_printf_fn();
            if args.len() == 1 {
                let arg_val = self.generate_expression(&args[0])?;

                match arg_val {
                    BasicValueEnum::PointerValue(ptr_val) => {
                        // Pointer may be a pointer to alsh_str; extract .data field
                        let i64_type = self.context.i64_type();
                        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
                        let alsh_str_ty = self.context.struct_type(&[i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);
                        let data_ptr = self.builder.build_struct_gep(alsh_str_ty, ptr_val, 2, "data_ptr").map_err(|e| e.to_string())?;
                        let data = self.builder.build_load(i8_ptr_type, data_ptr, "data_load").map_err(|e| e.to_string())?;
                        let format_str = self.context.const_string(b"%s\n", true);
                        let global_format = self.module.add_global(format_str.get_type(), None, "format");
                        global_format.set_initializer(&format_str);
                        let format_ptr = self.builder.build_pointer_cast(
                            global_format.as_pointer_value(),
                            self.context.ptr_type(AddressSpace::default()),
                            "format_ptr",
                        ).map_err(|e| e.to_string())?;
                        let _ = self.builder.build_call(printf_fn, &[format_ptr.into(), data.into()], "println").map_err(|e| e.to_string())?;
                    }
                    BasicValueEnum::IntValue(int_val) => {
                        // Integer argument
                        let format_str = self.context.const_string(b"%d\n", true);
                        let global_format = self.module.add_global(format_str.get_type(), None, "format");
                        global_format.set_initializer(&format_str);
                        let format_ptr = self.builder.build_pointer_cast(
                            global_format.as_pointer_value(),
                            self.context.ptr_type(AddressSpace::default()),
                            "format_ptr",
                        ).map_err(|e| e.to_string())?;
                        let _ = self.builder.build_call(printf_fn, &[format_ptr.into(), int_val.into()], "println").map_err(|e| e.to_string())?;
                    }
                    _ => return Err("std::println expects string or integer argument".to_string()),
                }

                // Return a dummy value since we don't care about the return value of printf here
                Ok(self.context.i32_type().const_int(0, false).into())
            } else {
                Err("std::println expects 1 argument".to_string())
            }
        } else {
            Err(format!("Unknown function: {}", name))
        }
    }

    fn generate_c_call(&mut self, name: &str, args: &[Expression]) -> Result<BasicValueEnum<'ctx>, String> {
        let c_fn = self.get_c_function(name)?;
        let mut arg_vals = Vec::new();
        for arg in args {
            arg_vals.push(self.generate_c_call_argument(arg)?.into());
        }
        let _ = self.builder.build_call(c_fn, &arg_vals, &format!("c_{}", name)).map_err(|e| e.to_string())?;
        // Return a dummy value - the actual return depends on the C function
        Ok(self.context.i32_type().const_int(0, false).into())
    }

    fn generate_c_call_argument(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            Expression::Literal(crate::control_flow::Value::String(s)) => {
                let str_val = self.context.const_string(s.as_bytes(), true);
                let global_chars = self.module.add_global(str_val.get_type(), None, "cstr");
                global_chars.set_initializer(&str_val);
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let cstr_ptr = self.builder.build_pointer_cast(
                    global_chars.as_pointer_value(),
                    ptr_type,
                    "cstr_ptr",
                ).map_err(|e| e.to_string())?;
                Ok(cstr_ptr.into())
            }
            Expression::Variable(name) => {
                if let Some(var) = self.variables.get(name) {
                    if let Some(var_type) = self.variable_types.get(name) {
                        if var_type == "pointer" {
                            let loaded = self.builder.build_load(self.context.ptr_type(AddressSpace::default()), *var, name).map_err(|e| e.to_string())?;
                            if let BasicValueEnum::PointerValue(ptr_val) = loaded {
                                let i64_type = self.context.i64_type();
                                let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
                                let alsh_str_ty = self.context.struct_type(&[i64_type.into(), i64_type.into(), i8_ptr_type.into()], false);
                                let data_ptr = self.builder.build_struct_gep(alsh_str_ty, ptr_val, 2, "data_ptr").map_err(|e| e.to_string())?;
                                let data = self.builder.build_load(i8_ptr_type, data_ptr, "data_load").map_err(|e| e.to_string())?;
                                Ok(data)
                            } else {
                                Ok(loaded)
                            }
                        } else {
                            self.generate_expression(expr)
                        }
                    } else {
                        self.generate_expression(expr)
                    }
                } else {
                    self.generate_expression(expr)
                }
            }
            _ => self.generate_expression(expr),
        }
    }

    fn get_printf_fn(&self) -> FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("printf") {
            func
        } else {
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let printf_type = self.context.i32_type().fn_type(&[ptr_type.into()], true);
            self.module.add_function("printf", printf_type, None)
        }
    }

    fn get_c_function(&self, name: &str) -> Result<FunctionValue<'ctx>, String> {
        if let Some(func) = self.module.get_function(name) {
            Ok(func)
        } else {
            // Declare common C library functions on demand
            let fn_type = match name {
                "printf" => {
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    // printf is variadic: int printf(const char *format, ...)
                    self.context.i32_type().fn_type(&[ptr_type.into()], true)
                }
                "puts" => {
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    // puts: int puts(const char *s)
                    self.context.i32_type().fn_type(&[ptr_type.into()], false)
                }
                "strlen" => {
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    // strlen: size_t strlen(const char *s)
                    self.context.i64_type().fn_type(&[ptr_type.into()], false)
                }
                "malloc" => {
                    // malloc: void *malloc(size_t size)
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    ptr_type.fn_type(&[self.context.i64_type().into()], false)
                }
                "free" => {
                    // free: void free(void *ptr)
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    self.context.void_type().fn_type(&[ptr_type.into()], false)
                }
                _ => {
                    // For unknown functions, assume they take no args and return i32
                    // This is a fallback - ideally we'd have better type info
                    return Err(format!("Unknown C function: {}", name));
                }
            };
            Ok(self.module.add_function(name, fn_type, None))
        }
    }

    pub fn print_ir(&self) {
        println!("{}", self.module.print_to_string().to_string());
    }

    pub fn get_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    pub fn get_assembly(&self) -> Result<String, String> {
        use inkwell::targets::{InitializationConfig, Target};

        let config = InitializationConfig::default();
        Target::initialize_all(&config);

        let triple = inkwell::targets::TargetTriple::create("x86_64-unknown-linux-gnu");
        let target = Target::from_triple(&triple)
            .map_err(|_| "Failed to get target".to_string())?;

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                inkwell::OptimizationLevel::Default,
                inkwell::targets::RelocMode::Default,
                inkwell::targets::CodeModel::Default,
            )
            .ok_or_else(|| "Failed to create target machine".to_string())?;

        target_machine
            .write_to_memory_buffer(&self.module, inkwell::targets::FileType::Assembly)
            .map(|buf| String::from_utf8_lossy(buf.as_slice()).to_string())
            .map_err(|_| "Failed to emit assembly".to_string())
    }

    pub fn get_object(&self) -> Result<Vec<u8>, String> {
        use inkwell::targets::{InitializationConfig, Target};

        let config = InitializationConfig::default();
        Target::initialize_all(&config);

        let triple = inkwell::targets::TargetTriple::create("x86_64-unknown-linux-gnu");
        let target = Target::from_triple(&triple)
            .map_err(|_| "Failed to get target".to_string())?;

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                inkwell::OptimizationLevel::Default,
                inkwell::targets::RelocMode::Default,
                inkwell::targets::CodeModel::Default,
            )
            .ok_or_else(|| "Failed to create target machine".to_string())?;

        target_machine
            .write_to_memory_buffer(&self.module, inkwell::targets::FileType::Object)
            .map(|buf| buf.as_slice().to_vec())
            .map_err(|_| "Failed to emit object file".to_string())
    }
}

