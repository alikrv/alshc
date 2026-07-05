// src/codegen.rs
use crate::control_flow::StringPart;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
#[allow(deprecated)]
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, BasicType};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::OptimizationLevel;
use inkwell::AddressSpace;
use std::collections::HashMap;

use crate::control_flow::{Condition, Expression, Statement};
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
    function_sigs: HashMap<String, Vec<crate::control_flow::Parameter>>,
    just_run: bool,
    has_main: bool,
    main_function_name: Option<String>,
    loop_end_block: Option<BasicBlock<'ctx>>,
    loop_start_block: Option<BasicBlock<'ctx>>,
    needs_return: bool,
    opt_level: OptimizationLevel,
    global_counter: u64,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn new(context: &'ctx Context, just_run: bool, opt_level: OptimizationLevel) -> Self {
        let module = context.create_module("alsh");
        let builder = context.create_builder();

        CodeGen {
            context,
            module,
            builder,
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            functions: HashMap::new(),
            function_sigs: HashMap::new(),
            just_run,
            has_main: false,
            main_function_name: None,
            loop_end_block: None,
            loop_start_block: None,
            needs_return: true,
            opt_level,
            global_counter: 0,
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
        let mut functions_def: HashMap<String, (Vec<crate::control_flow::Parameter>, String, Vec<Statement>)> = HashMap::new();

        for stmt in statements {
            match stmt {
                Statement::FunctionDef {
                    name,
                    params,
                    return_type,
                    body,
                } => {
                    functions_def.insert(name.clone(), (params.clone(), return_type.clone(), body.clone()));
                }
                _ => {
                    top_level_statements.push(stmt.clone());
                }
            }
        }

        // Pre-pass: declare all functions so they can reference each other
        for (name, (params, return_type, _body)) in &functions_def {
            self.declare_function(name, params, return_type)?;
        }

        // Generate function bodies
        for (name, (params, _return_type, body)) in &functions_def {
            self.generate_function_body(name, params, body)?;
        }

        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
        let argv_type = self.context.ptr_type(AddressSpace::default());
        // alsh_str type: { i64, i64, i8* }
        let i64_type = self.context.i64_type();
        let _alsh_str_type = self.context.struct_type(
            &[i64_type.into(), i64_type.into(), i8_ptr_type.into()],
            false,
        );

        if self.has_main {
            // @main was indicated by preprocessing. If we don't know which function
            // was marked (no `main_function_name`), attempt to fall back to a
            // function literally named "main". If that also doesn't exist, ignore
            // the has_main marker and proceed (no wrapper will be generated).
            if self.main_function_name.is_none() {
                if functions_def.contains_key("main") {
                    self.main_function_name = Some("main".to_string());
                } else {
                    // silently ignore missing main target from preprocessor
                    self.has_main = false;
                }
            }
            // If we still have a main_function_name, the named function is already
            // generated and no wrapper is needed. Otherwise we continue.
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
                self.generate_statement(stmt)
                    .map_err(|e| format!("Codegen error: {}", e))?;
            }

            // Return 0 if we haven't already returned
            if self.needs_return {
                let _ = self
                    .builder
                    .build_return(Some(&i32_type.const_int(0, false)));
            }
        } else if !top_level_statements.is_empty() {
            return Err("Top-level code found but no @main or @justrunit directive. Add @main above a function or use @justrunit.".to_string());
        } else {
            // No code at all: create empty main
            let main_fn_type = i32_type.fn_type(&[i32_type.into(), argv_type.into()], false);
            let main_fn = self.module.add_function("main", main_fn_type, None);
            let entry_bb = self.context.append_basic_block(main_fn, "entry");
            self.builder.position_at_end(entry_bb);
            let _ = self
                .builder
                .build_return(Some(&i32_type.const_int(0, false)));
        }

        Ok(())
    }

    #[allow(deprecated)]
    fn declare_function(
        &mut self,
        name: &str,
        params: &[crate::control_flow::Parameter],
        _return_type: &str,
    ) -> Result<(), String> {
        let i32_type = self.context.i32_type();
        let argv_type = self.context.ptr_type(AddressSpace::default());

        // If this function is marked with @main, name it "main"
        let actual_name = if self.has_main
            && self
                .main_function_name
                .as_ref()
                .map(|n| n == name)
                .unwrap_or(false)
        {
            "main"
        } else {
            name
        };

        // Build function type based on parameters and their types
        let fn_type = if actual_name == "main" {
            i32_type.fn_type(&[i32_type.into(), argv_type.into()], false)
        } else {
            // Convert parameter types to LLVM types. If a parameter is variadic,
            // lower it to two explicit parameters: a pointer to the element array
            // and an i64 length.
            let mut param_types_meta: Vec<BasicMetadataTypeEnum> = Vec::new();
            for p in params.iter() {
                    if p.is_variadic {
                    // element pointer type and length
                    let elem_ptr_ty = match p.type_name.as_str() {
                        "i32" | "int" => self.context.i32_type().ptr_type(AddressSpace::default()).into(),
                        "i64" | "long" => self.context.i64_type().ptr_type(AddressSpace::default()).into(),
                        "f64" | "double" | "float" => self.context.f64_type().ptr_type(AddressSpace::default()).into(),
                        "bool" => i32_type.ptr_type(AddressSpace::default()).into(),
                        "str" => self.context.ptr_type(AddressSpace::default()).ptr_type(AddressSpace::default()).into(),
                        _ => i32_type.ptr_type(AddressSpace::default()).into(),
                    };
                    param_types_meta.push(elem_ptr_ty);
                    param_types_meta.push(self.context.i64_type().into());
                } else {
                    let ty = match p.type_name.as_str() {
                        "i32" | "int" => i32_type.into(),
                        "i64" | "long" => self.context.i64_type().into(),
                        "f64" | "double" | "float" => self.context.f64_type().into(),
                        "bool" => i32_type.into(), // bool is i1 in LLVM, but we use i32 for simplicity
                        "str" => self.context.ptr_type(AddressSpace::default()).into(),
                        _ => i32_type.into(), // default to i32
                    };
                    param_types_meta.push(ty);
                }
            }

            // For now, all functions return i32. Type system can be extended later.
            i32_type.fn_type(&param_types_meta, false)
        };

        let func = self.module.add_function(actual_name, fn_type, None);
        self.functions.insert(name.to_string(), func);
        // Record the parameter signature for later use at call sites
        self.function_sigs.insert(name.to_string(), params.to_vec());
        Ok(())
    }

    fn generate_function_body(
        &mut self,
        name: &str,
        params: &[crate::control_flow::Parameter],
        body: &[Statement],
    ) -> Result<(), String> {
        let i32_type = self.context.i32_type();

        // Get the previously declared function
        let func = self.functions.get(name).cloned()
            .ok_or(format!("Function {} not found", name))?;

        let entry_bb = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_bb);

        // Save current variable state
        let saved_vars = self.variables.clone();
        let saved_types = self.variable_types.clone();

        self.variables.clear();
        self.variable_types.clear();
        self.needs_return = true;

        // Set up local variables for parameters. Variadic params are lowered
        // to (elem_ptr, len) so we handle those specially.
        let mut param_index: u32 = 0;
        for param in params.iter() {
            if param.is_variadic {
                // elem_ptr is at param_index, len is at param_index + 1
                if let Some(elem_ptr_val) = func.get_nth_param(param_index) {
                    let elem_ptr_alloca = self.create_entry_alloca(&param.name, elem_ptr_val.get_type())?;
                    self.builder
                        .build_store(elem_ptr_alloca, elem_ptr_val)
                        .map_err(|e| e.to_string())?;
                    self.variables.insert(param.name.clone(), elem_ptr_alloca);
                    self.variable_types.insert(param.name.clone(), format!("{}[]", param.type_name));
                }
                if let Some(len_val) = func.get_nth_param(param_index + 1) {
                    let len_alloca = self.create_entry_alloca(&format!("{}_len", param.name), len_val.get_type())?;
                    self.builder
                        .build_store(len_alloca, len_val)
                        .map_err(|e| e.to_string())?;
                    self.variables.insert(format!("{}_len", param.name), len_alloca);
                    self.variable_types.insert(format!("{}_len", param.name), "i64".to_string());
                }
                param_index += 2;
            } else {
                if let Some(param_val) = func.get_nth_param(param_index) {
                    let param_type = match param.type_name.as_str() {
                        "i32" | "int" => i32_type.into(),
                        "i64" | "long" => self.context.i64_type().into(),
                        "f64" | "double" | "float" => self.context.f64_type().into(),
                        "bool" => i32_type.into(),
                        "str" => self.context.ptr_type(AddressSpace::default()).into(),
                        _ => i32_type.into(),
                    };
                    let param_alloca = self.create_entry_alloca(&param.name, param_type)?;
                    self.builder
                        .build_store(param_alloca, param_val)
                        .map_err(|e| e.to_string())?;
                    self.variables.insert(param.name.clone(), param_alloca);
                    self.variable_types.insert(param.name.clone(), param.type_name.clone());
                }
                param_index += 1;
            }
        }

        // Generate function body
        for stmt in body {
            self.generate_statement(stmt)
                .map_err(|e| format!("Codegen error in function {}: {}", name, e))?;
        }

        // Return 0 if we haven't already returned
        if self.needs_return {
            let _ = self
                .builder
                .build_return(Some(&i32_type.const_int(0, false)));
        }

        // Restore variable state
        self.variables = saved_vars;
        self.variable_types = saved_types;

        Ok(())
    }

    fn generate_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let { name, value } => {
                let val = self.generate_expression(value)?;

                // If variable already exists, store into the existing alloca instead
                if let Some(existing) = self.variables.get(name) {
                    self.builder
                        .build_store(*existing, val)
                        .map_err(|e| e.to_string())?;

                    // Update type tracking
                    let var_type = match val {
                        BasicValueEnum::PointerValue(_) => "pointer",
                        BasicValueEnum::IntValue(_) => "i32",
                        BasicValueEnum::FloatValue(_) => "f64",
                        BasicValueEnum::ArrayValue(_) => "array",
                        BasicValueEnum::StructValue(_) => "struct",
                        BasicValueEnum::VectorValue(_) => "vector",
                        BasicValueEnum::ScalableVectorValue(_) => "scalable_vector",
                    };
                    self.variable_types
                        .insert(name.clone(), var_type.to_string());
                } else {
                    // Allocate the variable in the function entry block for better IR
                    let var = self.create_entry_alloca(name, val.get_type())?;
                    self.builder
                        .build_store(var, val)
                        .map_err(|e| e.to_string())?;

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
                    self.variable_types
                        .insert(name.clone(), var_type.to_string());
                }
            }
            Statement::Command(cmd) => {
                // For now, just print the command
                let printf_fn = self.get_printf_fn();
                let str_val = self.context.const_string(cmd.as_bytes(), false);
                let global_str = self.module.add_global(str_val.get_type(), None, "cmd");
                global_str.set_initializer(&str_val);
                let str_ptr = self
                    .builder
                    .build_pointer_cast(
                        global_str.as_pointer_value(),
                        self.context.ptr_type(AddressSpace::default()),
                        "str_ptr",
                    )
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_call(printf_fn, &[str_ptr.into()], "call")
                    .map_err(|e| e.to_string())?;
            }
            Statement::Expression(expr) => {
                let _ = self.generate_expression(expr)?;
            }
            Statement::While { condition, body } => {
                self.generate_while(condition, body)?;
            }
            Statement::If {
                condition,
                then_block,
                elif_blocks,
                else_block,
            } => {
                self.generate_if(condition, then_block, elif_blocks, else_block)?;
            }
            Statement::For { var, items, body } => {
                self.generate_for(var, items, body)?;
            }
            Statement::Foreach {
                var,
                iterable,
                body,
            } => {
                self.generate_foreach(var, iterable, body)?;
            }
            Statement::Loop {
                count,
                interval,
                body,
            } => {
                self.generate_loop(count, interval, body)?;
            }
            Statement::Break { .. } => {
                if let Some(end_block) = self.loop_end_block {
                    self.builder
                        .build_unconditional_branch(end_block)
                        .map_err(|e| e.to_string())?;
                } else {
                    return Err("break statement outside of loop".to_string());
                }
            }
            Statement::Continue => {
                if let Some(start_block) = self.loop_start_block {
                    self.builder
                        .build_unconditional_branch(start_block)
                        .map_err(|e| e.to_string())?;
                } else {
                    return Err("continue statement outside of loop".to_string());
                }
            }
            Statement::Return { value } => {
                if let Some(ret_expr) = value {
                    let ret_val = self.generate_expression(ret_expr)?;
                    let return_value: BasicValueEnum<'ctx> = match ret_val {
                        BasicValueEnum::IntValue(i) => i.into(),
                        BasicValueEnum::FloatValue(f) => self
                            .builder
                            .build_float_to_signed_int(f, self.context.i32_type(), "ret_cast")
                            .map_err(|e| e.to_string())?
                            .into(),
                        BasicValueEnum::PointerValue(p) => self
                            .builder
                            .build_ptr_to_int(p, self.context.i32_type(), "ret_ptr_cast")
                            .map_err(|e| e.to_string())?
                            .into(),
                        _ => self.context.i32_type().const_int(0, false).into(),
                    };
                    self.builder
                        .build_return(Some(&return_value))
                        .map_err(|e| e.to_string())?;
                } else {
                    let zero = self.context.i32_type().const_int(0, false);
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| e.to_string())?;
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
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No function to insert into")?;

        let cond_block = self.context.append_basic_block(current_fn, "while_cond");
        let body_block = self.context.append_basic_block(current_fn, "while_body");
        let end_block = self.context.append_basic_block(current_fn, "while_end");

        // Branch to condition check
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| e.to_string())?;

        // Generate condition block
        self.builder.position_at_end(cond_block);
        let cond_val = self.generate_condition(condition)?;
        self.builder
            .build_conditional_branch(cond_val, body_block, end_block)
            .map_err(|e| e.to_string())?;

        // Generate body block
        self.builder.position_at_end(body_block);
        for stmt in body {
            self.generate_statement(stmt)?;
        }
        self.builder
            .build_unconditional_branch(cond_block)
            .map_err(|e| e.to_string())?;

        // Set position to end block for next statements
        self.builder.position_at_end(end_block);

        Ok(())
    }

    fn generate_if(
        &mut self,
        condition: &Condition,
        then_block: &[Statement],
        elif_blocks: &[(Condition, Vec<Statement>)],
        else_block: &Option<Vec<Statement>>,
    ) -> Result<(), String> {
        let current_fn = self
            .builder
            .get_insert_block()
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

        self.builder
            .build_conditional_branch(cond_val, then_bb, else_bb)
            .map_err(|e| e.to_string())?;

        // Generate then block
        self.builder.position_at_end(then_bb);
        for stmt in then_block {
            self.generate_statement(stmt)?;
        }
        self.builder
            .build_unconditional_branch(end_bb)
            .map_err(|e| e.to_string())?;

        // Generate elif/else blocks
        if !elif_blocks.is_empty() {
            self.builder.position_at_end(else_bb);
            self.generate_elif_chain(current_fn, end_bb, elif_blocks, else_block)?;
        } else if let Some(else_body) = else_block {
            self.builder.position_at_end(else_bb);
            for stmt in else_body {
                self.generate_statement(stmt)?;
            }
            self.builder
                .build_unconditional_branch(end_bb)
                .map_err(|e| e.to_string())?;
        }

        self.builder.position_at_end(end_bb);
        Ok(())
    }

    fn generate_elif_chain(
        &mut self,
        current_fn: FunctionValue<'ctx>,
        end_bb: BasicBlock<'ctx>,
        elif_blocks: &[(Condition, Vec<Statement>)],
        else_block: &Option<Vec<Statement>>,
    ) -> Result<(), String> {
        if elif_blocks.is_empty() {
            if let Some(else_body) = else_block {
                for stmt in else_body {
                    self.generate_statement(stmt)?;
                }
                self.builder
                    .build_unconditional_branch(end_bb)
                    .map_err(|e| e.to_string())?;
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
        self.builder
            .build_conditional_branch(elif_cond_val, elif_body_bb, next_block)
            .map_err(|e| e.to_string())?;

        // Generate elif body
        self.builder.position_at_end(elif_body_bb);
        for stmt in elif_body {
            self.generate_statement(stmt)?;
        }
        self.builder
            .build_unconditional_branch(end_bb)
            .map_err(|e| e.to_string())?;

        // Recursively handle remaining elif/else blocks
        self.builder.position_at_end(next_block);
        self.generate_elif_chain(current_fn, end_bb, remaining_elifs, else_block)?;

        Ok(())
    }

    fn generate_for(
        &mut self,
        var: &str,
        items: &[Expression],
        body: &[Statement],
    ) -> Result<(), String> {
        let int_type = self.context.i32_type();
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No function to insert into")?;

        let loop_block = self.context.append_basic_block(current_fn, "for_loop");
        let end_block = self.context.append_basic_block(current_fn, "for_end");

        // For simple numeric range for now
        if items.len() == 1 {
            if let Expression::Literal(crate::control_flow::Value::Number(n)) = items[0] {
                // Create loop counter
                let counter = self
                    .builder
                    .build_alloca(int_type, var)
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(counter, int_type.const_int(0, false))
                    .map_err(|e| e.to_string())?;
                self.variables.insert(var.to_string(), counter);
                self.variable_types
                    .insert(var.to_string(), "i32".to_string());

                self.builder
                    .build_unconditional_branch(loop_block)
                    .map_err(|e| e.to_string())?;

                self.builder.position_at_end(loop_block);
                let counter_val = self
                    .builder
                    .build_load(int_type, counter, "counter")
                    .map_err(|e| e.to_string())?;
                let limit = int_type.const_int(n as u64, false);
                let cond = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::SLT,
                        counter_val.into_int_value(),
                        limit,
                        "for_cond",
                    )
                    .map_err(|e| e.to_string())?;

                let body_bb = self.context.append_basic_block(current_fn, "for_body");
                self.builder
                    .build_conditional_branch(cond, body_bb, end_block)
                    .map_err(|e| e.to_string())?;

                self.builder.position_at_end(body_bb);
                for stmt in body {
                    self.generate_statement(stmt)?;
                }

                // Increment counter
                let counter_val = self
                    .builder
                    .build_load(int_type, counter, "counter")
                    .map_err(|e| e.to_string())?;
                let incremented = self
                    .builder
                    .build_int_add(
                        counter_val.into_int_value(),
                        int_type.const_int(1, false),
                        "inc",
                    )
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(counter, incremented)
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_unconditional_branch(loop_block)
                    .map_err(|e| e.to_string())?;
            }
        }

        self.builder.position_at_end(end_block);
        Ok(())
    }

    fn generate_foreach(
        &mut self,
        _var: &str,
        _iterable: &Expression,
        _body: &[Statement],
    ) -> Result<(), String> {
        // For now, defer foreach implementation as it requires array handling
        // TODO: Implement foreach with proper array/list iteration
        Ok(())
    }

    fn create_entry_alloca(&self, name: &str, ty: inkwell::types::BasicTypeEnum<'ctx>) -> Result<PointerValue<'ctx>, String> {
        // Find current function
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No function to insert into")?;

        // Get entry basic block
        let entry_bb = current_fn
            .get_first_basic_block()
            .ok_or("Function has no entry block")?;

        // Create a temporary builder to insert at the start of entry
        let entry_builder = self.context.create_builder();
        if let Some(first_instr) = entry_bb.get_first_instruction() {
            entry_builder.position_before(&first_instr);
        } else {
            entry_builder.position_at_end(entry_bb);
        }

        let alloca = entry_builder
            .build_alloca(ty, name)
            .map_err(|e| e.to_string())?;
        Ok(alloca)
    }

    fn generate_loop(
        &mut self,
        count: &Option<u64>,
        interval: &Option<u64>,
        body: &[Statement],
    ) -> Result<(), String> {
        let int_type = self.context.i32_type();
        let current_fn = self
            .builder
            .get_insert_block()
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
            let counter = self
                .builder
                .build_alloca(int_type, "_loop_counter")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(counter, int_type.const_int(0, false))
                .map_err(|e| e.to_string())?;

            self.builder
                .build_unconditional_branch(loop_block)
                .map_err(|e| e.to_string())?;

            self.builder.position_at_end(loop_block);
            let counter_val = self
                .builder
                .build_load(int_type, counter, "counter")
                .map_err(|e| e.to_string())?;
            let limit = int_type.const_int(*loop_count as u64, false);
            let cond = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::SLT,
                    counter_val.into_int_value(),
                    limit,
                    "loop_cond",
                )
                .map_err(|e| e.to_string())?;

            let body_bb = self.context.append_basic_block(current_fn, "loop_body");
            self.builder
                .build_conditional_branch(cond, body_bb, end_block)
                .map_err(|e| e.to_string())?;

            self.builder.position_at_end(body_bb);
            for stmt in body {
                self.generate_statement(stmt)?;
            }

            // Increment counter
            let counter_val = self
                .builder
                .build_load(int_type, counter, "counter")
                .map_err(|e| e.to_string())?;
            let incremented = self
                .builder
                .build_int_add(
                    counter_val.into_int_value(),
                    int_type.const_int(1, false),
                    "inc",
                )
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(counter, incremented)
                .map_err(|e| e.to_string())?;
            self.builder
                .build_unconditional_branch(loop_block)
                .map_err(|e| e.to_string())?;
        } else if interval.is_some() {
            // Loop with interval - similar to count but with sleep
            // For now, just do a simple loop (sleep not yet implemented)
            self.builder
                .build_unconditional_branch(loop_block)
                .map_err(|e| e.to_string())?;
            self.builder.position_at_end(loop_block);

            for stmt in body {
                self.generate_statement(stmt)?;
            }

            self.builder
                .build_unconditional_branch(loop_block)
                .map_err(|e| e.to_string())?;
        } else {
            // Infinite loop
            self.builder
                .build_unconditional_branch(loop_block)
                .map_err(|e| e.to_string())?;
            self.builder.position_at_end(loop_block);

            for stmt in body {
                self.generate_statement(stmt)?;
            }

            self.builder
                .build_unconditional_branch(loop_block)
                .map_err(|e| e.to_string())?;
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
                let mut left_val = self.generate_expression(left)?.into_int_value();
                let mut right_val = self.generate_expression(right)?.into_int_value();

                let left_bits = left_val.get_type().get_bit_width();
                let right_bits = right_val.get_type().get_bit_width();
                if left_bits != right_bits {
                    if left_bits < right_bits {
                        left_val = self
                            .builder
                            .build_int_s_extend(left_val, right_val.get_type(), "ext_left")
                            .map_err(|e| e.to_string())?;
                    } else {
                        right_val = self
                            .builder
                            .build_int_s_extend(right_val, left_val.get_type(), "ext_right")
                            .map_err(|e| e.to_string())?;
                    }
                }

                let predicate = match op {
                    crate::control_flow::CompareOp::Lt => IntPredicate::SLT,
                    crate::control_flow::CompareOp::Gt => IntPredicate::SGT,
                    crate::control_flow::CompareOp::Le => IntPredicate::SLE,
                    crate::control_flow::CompareOp::Ge => IntPredicate::SGE,
                    crate::control_flow::CompareOp::Eq => IntPredicate::EQ,
                    crate::control_flow::CompareOp::Ne => IntPredicate::NE,
                };

                self.builder
                    .build_int_compare(predicate, left_val, right_val, "cmp")
                    .map_err(|e| e.to_string())
            }
            Condition::Command(expr) => {
                let val = self.generate_expression(expr)?.into_int_value();
                let zero = val.get_type().const_int(0, false);
                self.builder
                    .build_int_compare(IntPredicate::NE, val, zero, "cond")
                    .map_err(|e| e.to_string())
            }
            _ => Err("Unsupported condition type".to_string()),
        }
    }

    fn generate_expression(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            Expression::Literal(val) => {
                use crate::control_flow::Value;
                match val {
                    Value::Number(n) => {
                        Ok(self.context.i32_type().const_int(*n as u64, false).into())
                    }
                    Value::Bool(b) => {
                        let val = if *b { 1u64 } else { 0u64 };
                        Ok(self.context.i32_type().const_int(val, false).into())
                    }
                    Value::String(s) => {
                        // Create a global C string and then create a global alsh_str pointing at it
                        let bytes = s.as_bytes();
                        let str_val = self.context.const_string(bytes, true);
                        let idx = self.global_counter;
                        self.global_counter += 1;
                        let global_chars_name = format!("str_chars_{}", idx);
                        let alsh_str_name = format!("str_obj_{}", idx);
                        let global_chars = self.module.add_global(str_val.get_type(), None, &global_chars_name);
                        global_chars.set_initializer(&str_val);
                        // alsh_str instance as a global struct
                        let i64_type = self.context.i64_type();
                        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
                        let alsh_str_ty = self.context.struct_type(
                            &[i64_type.into(), i64_type.into(), i8_ptr_type.into()],
                            false,
                        );
                        let alsh_str_global = self.module.add_global(alsh_str_ty, None, &alsh_str_name);
                        // Build initializer: { len, cap, ptr }
                        let len = i64_type.const_int(bytes.len() as u64, false);
                        let cap = i64_type.const_int(bytes.len() as u64, false);
                        let ptr = global_chars.as_pointer_value();
                        let init =
                            alsh_str_ty.const_named_struct(&[len.into(), cap.into(), ptr.into()]);
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
                            "pointer" | "str" => {
                                let load = self
                                    .builder
                                    .build_load(
                                        self.context.ptr_type(AddressSpace::default()),
                                        *var,
                                        name,
                                    )
                                    .map_err(|e| e.to_string())?;
                                Ok(load)
                            }
                            "i32" | "int" | "bool" => {
                                let load = self
                                    .builder
                                    .build_load(self.context.i32_type(), *var, name)
                                    .map_err(|e| e.to_string())?;
                                Ok(load)
                            }
                            "i64" | "long" => {
                                let load = self
                                    .builder
                                    .build_load(self.context.i64_type(), *var, name)
                                    .map_err(|e| e.to_string())?;
                                Ok(load)
                            }
                            "f64" | "float" | "double" => {
                                let load = self
                                    .builder
                                    .build_load(self.context.f64_type(), *var, name)
                                    .map_err(|e| e.to_string())?;
                                Ok(load)
                            }
                            _ => {
                                let load = self
                                    .builder
                                    .build_load(self.context.i64_type(), *var, name)
                                    .map_err(|e| e.to_string())?;
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
            Expression::FunctionCall(name, args) => self.generate_function_call(name, args),
            Expression::CCall(name, args) => self.generate_c_call(name, args),
            Expression::StringInterpolation(parts) => self.generate_string_interpolation(parts),
            Expression::BinaryOp(left, op, right) => self.generate_binary_op(left, op, right),
            _ => Err("Unsupported expression".to_string()),
        }
    }

    fn generate_binary_op(
        &mut self,
        left: &Expression,
        op: &crate::control_flow::BinOp,
        right: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let left_val = self.generate_expression(left)?;
        let right_val = self.generate_expression(right)?;

        // Extract integer values
        let mut left_int = match left_val {
            BasicValueEnum::IntValue(iv) => iv,
            _ => return Err("Binary operations require integer operands".to_string()),
        };

        let mut right_int = match right_val {
            BasicValueEnum::IntValue(iv) => iv,
            _ => return Err("Binary operations require integer operands".to_string()),
        };

        // Handle type promotion for mismatched types
        let left_bits = left_int.get_type().get_bit_width();
        let right_bits = right_int.get_type().get_bit_width();

        if left_bits != right_bits {
            // Promote to larger type
            if left_bits < right_bits {
                left_int = self.builder
                    .build_int_s_extend(left_int, right_int.get_type(), "ext_left")
                    .map_err(|e| e.to_string())?;
            } else {
                right_int = self.builder
                    .build_int_s_extend(right_int, left_int.get_type(), "ext_right")
                    .map_err(|e| e.to_string())?;
            }
        }

        let result = match op {
            crate::control_flow::BinOp::Add => self
                .builder
                .build_int_add(left_int, right_int, "add")
                .map_err(|e| e.to_string())?,
            crate::control_flow::BinOp::Sub => self
                .builder
                .build_int_sub(left_int, right_int, "sub")
                .map_err(|e| e.to_string())?,
            crate::control_flow::BinOp::Mul => self
                .builder
                .build_int_mul(left_int, right_int, "mul")
                .map_err(|e| e.to_string())?,
            crate::control_flow::BinOp::Div => self
                .builder
                .build_int_signed_div(left_int, right_int, "div")
                .map_err(|e| e.to_string())?,
            _ => return Err(format!("Unsupported binary operation: {:?}", op)),
        };

        Ok(result.into())
    }

    fn generate_function_call(
        &mut self,
        name: &str,
        args: &[Expression],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Ensure runtime varargs helpers are declared in the module so they
        // can be called from ALSH code (e.g. alsh_varargs_get_i32).
        if !self.functions.contains_key(name) {
            if name.starts_with("alsh_make_varargs_array_") {
                let fn_ty = self.context.ptr_type(AddressSpace::default()).fn_type(&[self.context.i64_type().into()], false);
                let f = self.module.add_function(name, fn_ty, None);
                self.functions.insert(name.to_string(), f);
            } else if name.starts_with("alsh_varargs_store_") {
                // store functions: void store(ptr, i64, <value>)
                let suffix = &name[19..]; // after 'alsh_varargs_store_'
                let val_ty: BasicTypeEnum = match suffix {
                    "i32" => self.context.i32_type().as_basic_type_enum(),
                    "i64" => self.context.i64_type().as_basic_type_enum(),
                    "f64" => self.context.f64_type().as_basic_type_enum(),
                    "ptr" => self.context.ptr_type(AddressSpace::default()).as_basic_type_enum(),
                    _ => self.context.i32_type().as_basic_type_enum(),
                };
                let fn_ty = self.context.void_type().fn_type(&[self.context.ptr_type(AddressSpace::default()).into(), self.context.i64_type().into(), val_ty.into()], false);
                let f = self.module.add_function(name, fn_ty, None);
                self.functions.insert(name.to_string(), f);
            } else if name.starts_with("alsh_varargs_get_") {
                // get functions: <value> get(ptr, i64)
                let suffix = &name[16..]; // after 'alsh_varargs_get_'
                let ret_ty: BasicTypeEnum = match suffix {
                    "i32" => self.context.i32_type().as_basic_type_enum(),
                    "i64" => self.context.i64_type().as_basic_type_enum(),
                    "f64" => self.context.f64_type().as_basic_type_enum(),
                    "ptr" => self.context.ptr_type(AddressSpace::default()).as_basic_type_enum(),
                    _ => self.context.i32_type().as_basic_type_enum(),
                };
                let fn_ty = ret_ty.fn_type(&[self.context.ptr_type(AddressSpace::default()).into(), self.context.i64_type().into()], false);
                let f = self.module.add_function(name, fn_ty, None);
                self.functions.insert(name.to_string(), f);
            }
        }

        if let Some(func) = self.functions.get(name).cloned() {
            // Call user-defined function
            let mut arg_vals: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();

            if let Some(sig) = self.function_sigs.get(name).cloned() {
                // If last parameter is variadic, pack remaining args into an array
                if let Some(last_param) = sig.last() {
                    if last_param.is_variadic {
                        let fixed_count = sig.len() - 1;
                        // first, handle fixed args
                        for i in 0..fixed_count {
                            if i < args.len() {
                                let v = self.generate_expression(&args[i])?;
                                arg_vals.push(v.into());
                            } else {
                                // missing arg: push default 0
                                arg_vals.push(self.context.i32_type().const_int(0, false).into());
                            }
                        }

                        // pack variadic args
                        let var_args = if args.len() > fixed_count { &args[fixed_count..] } else { &[] };
                        let var_count = var_args.len();

                        // element type selection (kept for potential casting needs)

                        // Allocate varargs array via runtime helper and store elements via runtime helper.
                        let make_name = match last_param.type_name.as_str() {
                            "i32" | "int" => "alsh_make_varargs_array_i32",
                            "i64" | "long" => "alsh_make_varargs_array_i64",
                            "f64" | "double" | "float" => "alsh_make_varargs_array_f64",
                            "bool" => "alsh_make_varargs_array_i32",
                            "str" => "alsh_make_varargs_array_ptr",
                            _ => "alsh_make_varargs_array_i32",
                        };

                        let store_name = match last_param.type_name.as_str() {
                            "i32" | "int" => "alsh_varargs_store_i32",
                            "i64" | "long" => "alsh_varargs_store_i64",
                            "f64" | "double" | "float" => "alsh_varargs_store_f64",
                            "bool" => "alsh_varargs_store_i32",
                            "str" => "alsh_varargs_store_ptr",
                            _ => "alsh_varargs_store_i32",
                        };

                        // declare/ensure make function exists: void* make(size_t)
                        let i64_type = self.context.i64_type();
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let make_fn_ty = ptr_type.fn_type(&[i64_type.into()], false);
                        let make_fn = match self.module.get_function(make_name) {
                            Some(f) => f,
                            None => self.module.add_function(make_name, make_fn_ty, None),
                        };

                        // call make_fn with count
                        let count_arg = i64_type.const_int(var_count as u64, false);
                        let make_call = self.builder.build_call(make_fn, &[count_arg.into()], "make_varargs").map_err(|e| e.to_string())?;
                        let array_ptr = match make_call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => v,
                            _ => return Err("Failed to create varargs array".to_string()),
                        };

                        // store each element using store_fn
                        // value type depends on element type
                        let value_type = match last_param.type_name.as_str() {
                            "i32" | "int" => self.context.i32_type().into(),
                            "i64" | "long" => self.context.i64_type().into(),
                            "f64" | "double" | "float" => self.context.f64_type().into(),
                            "bool" => self.context.i32_type().into(),
                            "str" => ptr_type.into(),
                            _ => self.context.i32_type().into(),
                        };

                        let store_fn_ty = self.context.void_type().fn_type(&[ptr_type.into(), i64_type.into(), value_type], false);
                        let store_fn = match self.module.get_function(store_name) {
                            Some(f) => f,
                            None => self.module.add_function(store_name, store_fn_ty, None),
                        };

                        for (j, expr) in var_args.iter().enumerate() {
                            let v = self.generate_expression(expr)?;
                            // adjust integer widths if needed
                            let final_val: BasicValueEnum = match v {
                                BasicValueEnum::IntValue(iv) => {
                                    // store as i64 for uniformity when store expects i64, otherwise cast
                                    if last_param.type_name == "i64" || last_param.type_name == "long" {
                                        BasicValueEnum::IntValue(iv)
                                    } else {
                                        BasicValueEnum::IntValue(iv)
                                    }
                                }
                                other => other,
                            };

                            // idx as i64
                            let idx_val = i64_type.const_int(j as u64, false);
                            // Prepare store args: (array_ptr, idx, value)
                            let mut store_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                            store_args.push(array_ptr.into());
                            store_args.push(idx_val.into());
                            // For value, ensure type matches value_type
                            match (final_val, last_param.type_name.as_str()) {
                                (BasicValueEnum::IntValue(iv), "i32") | (BasicValueEnum::IntValue(iv), "int") | (BasicValueEnum::IntValue(iv), "bool") => {
                                    store_args.push(iv.into());
                                }
                                (BasicValueEnum::IntValue(iv), "i64") | (BasicValueEnum::IntValue(iv), "long") => {
                                    store_args.push(iv.into());
                                }
                                (BasicValueEnum::FloatValue(fv), "f64") | (BasicValueEnum::FloatValue(fv), "double") | (BasicValueEnum::FloatValue(fv), "float") => {
                                    store_args.push(fv.into());
                                }
                                (BasicValueEnum::PointerValue(pv), "str") => {
                                    store_args.push(pv.into());
                                }
                                (other, _) => {
                                    store_args.push(other.into());
                                }
                            }

                            let _ = self.builder.build_call(store_fn, &store_args, "store_va").map_err(|e| e.to_string())?;
                        }

                        // push pointer and length
                        arg_vals.push(array_ptr.into());
                        arg_vals.push(i64_type.const_int(var_count as u64, false).into());

                        // build call
                        let call_result = self
                            .builder
                            .build_call(func, &arg_vals, "call")
                            .map_err(|e| e.to_string())?;

                        return match call_result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => Ok(v),
                            inkwell::values::ValueKind::Instruction(_) => {
                                Err(format!("Function {} returned void", name))
                            }
                        };
                    } else {
                        // no variadic: normal path
                        for arg in args {
                            let val = self.generate_expression(arg)?;
                            arg_vals.push(val.into());
                        }
                        let call_result = self
                            .builder
                            .build_call(func, &arg_vals, "call")
                            .map_err(|e| e.to_string())?;
                        return match call_result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => Ok(v),
                            inkwell::values::ValueKind::Instruction(_) => {
                                Err(format!("Function {} returned void", name))
                            }
                        };
                    }
                } else {
                    return Err(format!("Empty signature for function {}", name));
                }
            } else {
                // no signature recorded: fallback to naïve call
                for arg in args {
                    let val = self.generate_expression(arg)?;
                    arg_vals.push(val.into());
                }
                let call_result = self
                    .builder
                    .build_call(func, &arg_vals, "call")
                    .map_err(|e| e.to_string())?;
                return match call_result.try_as_basic_value() {
                    inkwell::values::ValueKind::Basic(v) => Ok(v),
                    inkwell::values::ValueKind::Instruction(_) => {
                        Err(format!("Function {} returned void", name))
                    }
                };
            }
            //return Err(format!("Internal error generating function call for {}", name));
        } else if name.starts_with("std::") {
            // Standard library functions - call the C FFI wrappers from alsh-std
            let c_func_name = format!("alsh_std_{}", &name[5..]); // remove "std::" prefix
            self.generate_stdlib_call(&c_func_name, args)
        } else if name.starts_with("alsh_") {
            self.generate_runtime_call(name, args)
        } else {
            Err(format!("Unknown function: {}", name))
        }
    }

    fn generate_runtime_call(
        &mut self,
        name: &str,
        args: &[Expression],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let func = self.get_runtime_fn(name);
        let mut arg_vals: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
        for arg in args {
            let val = self.generate_expression(arg)?;
            arg_vals.push(val.into());
        }
        let call_result = self
            .builder
            .build_call(func, &arg_vals, "runtime_call")
            .map_err(|e| e.to_string())?;
        match call_result.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => Ok(v),
            inkwell::values::ValueKind::Instruction(_) => {
                Err(format!("Function {} returned void", name))
            }
        }
    }

    fn generate_stdlib_call(
        &mut self,
        c_func_name: &str,
        args: &[Expression],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();

        // Define the C function signature based on name
        let fn_type = match c_func_name {
            "alsh_std_print" | "alsh_std_println" | "alsh_std_eprint" => {
                // void alsh_std_print(const char *value)
                self.context.void_type().fn_type(&[ptr_type.into()], false)
            }
            "alsh_std_input" => {
                // char* alsh_std_input(const char *prompt)
                ptr_type.fn_type(&[ptr_type.into()], false)
            }
            "alsh_std_exit" => {
                // void alsh_std_exit(int code)
                self.context.void_type().fn_type(&[i32_type.into()], false)
            }
            // File I/O functions
            "alsh_std_env" => {
                // char* alsh_std_env(const char *key)
                ptr_type.fn_type(&[ptr_type.into()], false)
            }
            "alsh_std_readfile" => {
                // char* alsh_std_readfile(const char *path)
                ptr_type.fn_type(&[ptr_type.into()], false)
            }
            "alsh_std_writefile" | "alsh_std_appendfile" => {
                // int alsh_std_writefile(const char *path, const char *contents)
                i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false)
            }
            "alsh_std_exists" => {
                // int alsh_std_exists(const char *path)
                i32_type.fn_type(&[ptr_type.into()], false)
            }
            // String utilities
            "alsh_std_strlen" => {
                // usize alsh_std_strlen(const char *s)
                i64_type.fn_type(&[ptr_type.into()], false)
            }
            "alsh_std_upper" | "alsh_std_lower" | "alsh_std_trim" => {
                // char* alsh_std_upper(const char *s)
                ptr_type.fn_type(&[ptr_type.into()], false)
            }
            "alsh_std_contains" | "alsh_std_startswith" | "alsh_std_endswith" => {
                // int alsh_std_contains(const char *s, const char *sub)
                i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false)
            }
            "alsh_std_replace" => {
                // char* alsh_std_replace(const char *s, const char *from, const char *to)
                ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false)
            }
            "alsh_std_repeat" => {
                // char* alsh_std_repeat(const char *s, i32 n)
                ptr_type.fn_type(&[ptr_type.into(), i32_type.into()], false)
            }
            "alsh_std_strip" => {
                // char* alsh_std_strip(const char *s, const char *chars)
                ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false)
            }
            _ => {
                return Err(format!("Unknown stdlib function: {}", c_func_name));
            }
        };

        let c_fn = if let Some(existing) = self.module.get_function(c_func_name) {
            existing
        } else {
            self.module.add_function(c_func_name, fn_type, None)
        };

        // Convert arguments to C-compatible form
        let mut arg_vals = Vec::new();
        for arg in args {
            let val = self.generate_expression(arg)?;
            // Convert to C string if needed
            match val {
                BasicValueEnum::PointerValue(ptr_val) => {
                    // Check if this is an alsh_str - extract the .data field
                    let i64_type = self.context.i64_type();
                    let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
                    let alsh_str_ty = self.context.struct_type(
                        &[i64_type.into(), i64_type.into(), i8_ptr_type.into()],
                        false,
                    );

                    // Try to extract .data field from alsh_str
                    match self.builder.build_struct_gep(alsh_str_ty, ptr_val, 2, "data_ptr") {
                        Ok(data_ptr_field) => {
                            let data = self
                                .builder
                                .build_load(i8_ptr_type, data_ptr_field, "data_load")
                                .map_err(|e| e.to_string())?;
                            arg_vals.push(data.into());
                        }
                        Err(_) => {
                            // Not an alsh_str, use pointer directly
                            arg_vals.push(ptr_val.into());
                        }
                    }
                }
                BasicValueEnum::IntValue(int_val) => {
                    arg_vals.push(int_val.into());
                }
                _ => {
                    arg_vals.push(val.into());
                }
            }
        }

        let call_result = self
            .builder
            .build_call(c_fn, &arg_vals, c_func_name)
            .map_err(|e| e.to_string())?;

        // Some stdlib functions return `char *` (C string). Convert those to
        // `alsh_str *` using runtime `alsh_make_heap_str` so downstream code
        // (string interpolation and concatenation) can treat them uniformly.
        let c_string_returns = [
            "alsh_std_upper",
            "alsh_std_lower",
            "alsh_std_trim",
            "alsh_std_replace",
            "alsh_std_repeat",
            "alsh_std_strip",
            "alsh_std_readfile",
            "alsh_std_env",
            "alsh_std_input",
        ];

        if c_string_returns.contains(&c_func_name) {
            // Expect a pointer result
            match call_result.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(bv) => {
                    let c_ptr = bv.into_pointer_value();

                    // call strlen(c_ptr)
                    let strlen_fn = self.get_c_function("strlen")?;
                    let strlen_call = self
                        .builder
                        .build_call(strlen_fn, &[c_ptr.into()], "strlen")
                        .map_err(|e| e.to_string())?;
                    let len = match strlen_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(lv) => lv.into_int_value(),
                        _ => {
                            return Err("strlen returned non-integer".to_string());
                        }
                    };

                    // call runtime alsh_make_heap_str(c_ptr, len)
                    let mk_fn = self.get_runtime_fn("alsh_make_heap_str");
                    let mk_call = self
                        .builder
                        .build_call(mk_fn, &[c_ptr.into(), len.into()], "mk_str")
                        .map_err(|e| e.to_string())?;
                    let result_ptr = match mk_call.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(rv) => rv,
                        _ => return Err("alsh_make_heap_str returned void".to_string()),
                    };

                    // free the original C string returned by the stdlib (Rust side allocated)
                    if let Ok(free_fn) = self.get_c_function("alsh_std_free_string") {
                        let _ = self
                            .builder
                            .build_call(free_fn, &[c_ptr.into()], "free_cstr")
                            .map_err(|e| e.to_string())?;
                    }

                    Ok(result_ptr)
                }
                _ => Err("expected pointer return from stdlib C function".to_string()),
            }
        } else {
            match call_result.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => Ok(v),
                inkwell::values::ValueKind::Instruction(_) => {
                    // Function returned void, return dummy value
                    Ok(i32_type.const_int(0, false).into())
                }
            }
        }
    }

    fn generate_c_call(
        &mut self,
        name: &str,
        args: &[Expression],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let c_fn = self.get_c_function(name)?;
        let mut arg_vals: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
        for arg in args {
            arg_vals.push(self.generate_c_call_argument(arg)?.into());
        }
        // Adjust integer widths for variadic C functions (e.g., printf).
        // For printf we promote/truncate integers to 32-bit as expected by common format specifiers.
        // Convert metadata args to basic values so we can adjust integer widths,
        // then convert back to metadata values for the call.
        let mut final_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
        let is_vararg = c_fn.get_type().is_var_arg();
        for meta in arg_vals.into_iter() {
            let basic: BasicValueEnum = match meta {
                inkwell::values::BasicMetadataValueEnum::IntValue(iv) => BasicValueEnum::IntValue(iv),
                inkwell::values::BasicMetadataValueEnum::FloatValue(fv) => BasicValueEnum::FloatValue(fv),
                inkwell::values::BasicMetadataValueEnum::PointerValue(pv) => BasicValueEnum::PointerValue(pv),
                inkwell::values::BasicMetadataValueEnum::StructValue(sv) => BasicValueEnum::StructValue(sv),
                inkwell::values::BasicMetadataValueEnum::VectorValue(vv) => BasicValueEnum::VectorValue(vv),
                inkwell::values::BasicMetadataValueEnum::ArrayValue(av) => BasicValueEnum::ArrayValue(av),
                inkwell::values::BasicMetadataValueEnum::ScalableVectorValue(sv) => BasicValueEnum::ScalableVectorValue(sv),
                inkwell::values::BasicMetadataValueEnum::MetadataValue(_) => {
                    return Err("unsupported metadata value in C call arguments".to_string());
                }
            };

            let adjusted = match basic {
                BasicValueEnum::IntValue(iv) if is_vararg || name == "printf" => {
                    let bits = iv.get_type().get_bit_width();
                    if bits != 32 {
                        let cast = if bits > 32 {
                            self.builder
                                .build_int_truncate(iv, self.context.i32_type(), "vararg_trunc")
                                .map_err(|e| e.to_string())?
                        } else {
                            self.builder
                                .build_int_s_extend(iv, self.context.i32_type(), "vararg_ext")
                                .map_err(|e| e.to_string())?
                        };
                        BasicValueEnum::IntValue(cast)
                    } else {
                        BasicValueEnum::IntValue(iv)
                    }
                }
                other => other,
            };
            final_args.push(adjusted.into());
        }

        let _ = self
            .builder
            .build_call(c_fn, &final_args, &format!("c_{}", name))
            .map_err(|e| e.to_string())?;
        // Return a dummy value - the actual return depends on the C function
        Ok(self.context.i32_type().const_int(0, false).into())
    }

    fn generate_c_call_argument(
        &mut self,
        expr: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
                Expression::Literal(crate::control_flow::Value::String(s)) => {
                let str_val = self.context.const_string(s.as_bytes(), true);
                let idx = self.global_counter;
                self.global_counter += 1;
                let global_name = format!("cstr_{}", idx);
                let global_chars = self.module.add_global(str_val.get_type(), None, &global_name);
                global_chars.set_initializer(&str_val);
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let cstr_ptr = self
                    .builder
                    .build_pointer_cast(global_chars.as_pointer_value(), ptr_type, "cstr_ptr")
                    .map_err(|e| e.to_string())?;
                Ok(cstr_ptr.into())
            }
            Expression::Variable(name) => {
                if let Some(var) = self.variables.get(name) {
                    if let Some(var_type) = self.variable_types.get(name) {
                        if var_type == "pointer" {
                            let loaded = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    *var,
                                    name,
                                )
                                .map_err(|e| e.to_string())?;
                            if let BasicValueEnum::PointerValue(ptr_val) = loaded {
                                let i64_type = self.context.i64_type();
                                let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
                                let alsh_str_ty = self.context.struct_type(
                                    &[i64_type.into(), i64_type.into(), i8_ptr_type.into()],
                                    false,
                                );
                                let data_ptr = self
                                    .builder
                                    .build_struct_gep(alsh_str_ty, ptr_val, 2, "data_ptr")
                                    .map_err(|e| e.to_string())?;
                                let data = self
                                    .builder
                                    .build_load(i8_ptr_type, data_ptr, "data_load")
                                    .map_err(|e| e.to_string())?;
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
                "alsh_std_free_string" => {
                    // void alsh_std_free_string(char *ptr)
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

    fn get_runtime_fn(&self, name: &str) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = match name {
            "alsh_str_concat" => ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
            "alsh_int_to_str" => ptr_type.fn_type(&[self.context.i64_type().into()], false),
            "alsh_float_to_str" => ptr_type.fn_type(&[self.context.f64_type().into()], false),
            "alsh_make_heap_str" => ptr_type.fn_type(&[ptr_type.into(), self.context.i64_type().into()], false),
            "alsh_make_varargs_array_i32" | "alsh_make_varargs_array_i64" | "alsh_make_varargs_array_f64" | "alsh_make_varargs_array_ptr" => {
                ptr_type.fn_type(&[self.context.i64_type().into()], false)
            }
            "alsh_varargs_store_i32" => self.context.void_type().fn_type(&[ptr_type.into(), self.context.i64_type().into(), self.context.i32_type().into()], false),
            "alsh_varargs_store_i64" => self.context.void_type().fn_type(&[ptr_type.into(), self.context.i64_type().into(), self.context.i64_type().into()], false),
            "alsh_varargs_store_f64" => self.context.void_type().fn_type(&[ptr_type.into(), self.context.i64_type().into(), self.context.f64_type().into()], false),
            "alsh_varargs_store_ptr" => self.context.void_type().fn_type(&[ptr_type.into(), self.context.i64_type().into(), ptr_type.into()], false),
            "alsh_varargs_get_i32" => self.context.i32_type().fn_type(&[ptr_type.into(), self.context.i64_type().into()], false),
            "alsh_varargs_get_i64" => self.context.i64_type().fn_type(&[ptr_type.into(), self.context.i64_type().into()], false),
            "alsh_varargs_get_f64" => self.context.f64_type().fn_type(&[ptr_type.into(), self.context.i64_type().into()], false),
            "alsh_varargs_get_ptr" => ptr_type.fn_type(&[ptr_type.into(), self.context.i64_type().into()], false),
            _ => unreachable!("unknown runtime fn: {}", name),
        };
        self.module.add_function(name, fn_type, None)
    }

    fn generate_string_interpolation(
        &mut self,
        parts: &[StringPart],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let mut acc: Option<PointerValue<'ctx>> = None;

        for part in parts {
            let piece_ptr = match part {
                StringPart::Literal(s) => {
                    // reuse existing static-string global builder
                    let lit_expr =
                        Expression::Literal(crate::control_flow::Value::String(s.clone()));
                    match self.generate_expression(&lit_expr)? {
                        BasicValueEnum::PointerValue(p) => p,
                        _ => return Err("expected pointer for string literal".to_string()),
                    }
                }
                StringPart::Interpolation(expr) => {
                    match expr {
                        Expression::Variable(name) => {
                            let var_type =
                                self.variable_types.get(name).cloned().ok_or_else(|| {
                                    format!("unknown type for variable: {}", name)
                                })?;
                            match var_type.as_str() {
                                "i32" | "int" | "bool" => {
                                    let v = self.generate_expression(expr)?.into_int_value();
                                    let widened = self
                                        .builder
                                        .build_int_z_extend(v, self.context.i64_type(), "widen")
                                        .map_err(|e| e.to_string())?;
                                    let f = self.get_runtime_fn("alsh_int_to_str");
                                    let call_result = self
                                        .builder
                                        .build_call(f, &[widened.into()], "to_str")
                                        .map_err(|e| e.to_string())?;
                                    match call_result.try_as_basic_value() {
                                        inkwell::values::ValueKind::Basic(v) => {
                                            v.into_pointer_value()
                                        }
                                        inkwell::values::ValueKind::Instruction(_) => {
                                            return Err("alsh_int_to_str returned void".to_string());
                                        }
                                    }
                                }
                                "i64" | "long" => {
                                    let v = self.generate_expression(expr)?.into_int_value();
                                    let f = self.get_runtime_fn("alsh_int_to_str");
                                    let call_result = self
                                        .builder
                                        .build_call(f, &[v.into()], "to_str")
                                        .map_err(|e| e.to_string())?;
                                    match call_result.try_as_basic_value() {
                                        inkwell::values::ValueKind::Basic(v) => {
                                            v.into_pointer_value()
                                        }
                                        inkwell::values::ValueKind::Instruction(_) => {
                                            return Err("alsh_int_to_str returned void".to_string());
                                        }
                                    }
                                }
                                "pointer" | "str" => {
                                    // already an alsh_str*
                                    self.generate_expression(expr)?.into_pointer_value()
                                }
                                other => {
                                    return Err(format!(
                                        "interpolation not yet supported for type: {}",
                                        other
                                    ))
                                }
                            }
                        }
                        _ => {
                            return Err("only simple variable interpolation is supported currently"
                                .to_string())
                        }
                    }
                }
            };

            acc = Some(match acc {
                None => piece_ptr,
                Some(prev) => {
                    let concat_fn = self.get_runtime_fn("alsh_str_concat");
                    let call_result = self
                        .builder
                        .build_call(concat_fn, &[prev.into(), piece_ptr.into()], "concat")
                        .map_err(|e| e.to_string())?;
                    match call_result.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                        inkwell::values::ValueKind::Instruction(_) => {
                            return Err("alsh_str_concat returned void".to_string());
                        }
                    }
                }
            });
        }

        Ok(acc.ok_or("empty string interpolation")?.into())
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
        let target =
            Target::from_triple(&triple).map_err(|_| "Failed to get target".to_string())?;

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                self.opt_level,
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
        let target =
            Target::from_triple(&triple).map_err(|_| "Failed to get target".to_string())?;

        let target_machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                self.opt_level,
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
