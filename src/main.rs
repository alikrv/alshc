use alshc::{CodeGen};
use std::ffi::{CStr, CString};
use libc::c_char;
use inkwell::context::Context;
use inkwell::OptimizationLevel;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("alshc: fatal: no input files");
        std::process::exit(1);
    }

    let mut debug = false;
    let mut dump_tokens = false;
    let mut emit_ir = false;
    let mut emit_asm = false;
    let mut just_run = false;
    let mut output_file: Option<String> = None;
    let mut input_file: Option<String> = None;
    let opt_level = OptimizationLevel::Default;

    // Parse arguments. Unknown flags (and all after `--`) are forwarded to clang
    // so `alshc` behaves like a clang-compatible frontend.
    let mut clang_args: Vec<String> = Vec::new();
    let mut i = 0;
    let mut forward_to_clang = false;
    while i < args.len() {
        let a = args[i].as_str();
        if forward_to_clang {
            clang_args.push(args[i].clone());
            i += 1;
            continue;
        }

        match a {
            "-d" | "--debug" => {
                debug = true;
            }
            "--" => {
                forward_to_clang = true;
            }
            "--dump-tokens" => {
                dump_tokens = true;
            }
            "-ir" => {
                emit_ir = true;
            }
            "-S" => {
                // alshc supports -S to emit assembly directly; also forward to clang
                emit_asm = true;
                clang_args.push(args[i].clone());
            }
            "--just-run" | "-jr" => {
                just_run = true;
            }
            "-o" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                }
            }
            _ => {
                if a.starts_with('-') {
                    // Unknown flag: forward to clang so users can pass any clang/llvm opts
                    clang_args.push(args[i].clone());
                } else {
                    // Positional: assume input file
                    input_file = Some(a.to_string());
                }
            }
        }
        i += 1;
    }

    let input_file = match input_file {
        Some(f) => f,
        None => {
            eprintln!("alshc: fatal: no input files");
            std::process::exit(1);
        }
    };

    // Call preprocessor via its static library (alshpp/libalshpp.a)
    extern "C" {
        fn alshpp_preprocess_file(path: *const c_char) -> *mut c_char;
        fn alshpp_free_output(output: *mut c_char);
    }

    let c_path = match CString::new(input_file.clone()) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("alshc: invalid input filename");
            std::process::exit(1);
        }
    };

    let raw_out = unsafe { alshpp_preprocess_file(c_path.as_ptr() as *const c_char) };
    if raw_out.is_null() {
        eprintln!("alshc: preprocessor returned null");
        std::process::exit(1);
    }

    let test_code = unsafe {
        let cstr = CStr::from_ptr(raw_out);
        let s = match cstr.to_str() {
            Ok(st) => st.to_string(),
            Err(e) => {
                eprintln!("alshc: preprocessor returned invalid UTF-8: {}", e);
                alshpp_free_output(raw_out);
                std::process::exit(1);
            }
        };
        alshpp_free_output(raw_out);
        s
    };

    if dump_tokens {
        match alshc::lexer::Lexer::new(&test_code).tokenize() {
            Ok(tokens) => {
                println!("Tokens for {}:", input_file);
                for (i, t) in tokens.iter().enumerate() {
                    println!("  {}: {:?} ({}:{})", i, t.kind, t.line, t.col);
                }
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Lexer error: {}", e);
                std::process::exit(1);
            }
        }
    }

    if debug {
        println!("ALSH Compiler");
        println!("============");
        println!("Compiling code from {}:\n{}", input_file, test_code);
    }

    let mut parser = match alshc::parser2::Parser::from_source(&test_code) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("alshc: parse error: message='{}' line={} col={}", e.message, e.line, e.col);
            // Try to show the token stream for deeper diagnostics
            match alshc::lexer::Lexer::new(&test_code).tokenize() {
                Ok(tokens) => {
                    eprintln!("Tokens:");
                    for (i, t) in tokens.iter().enumerate() {
                        eprintln!("  {}: {:?} ({}:{})", i, t.kind, t.line, t.col);
                    }
                }
                Err(le) => {
                    eprintln!("Lexer error: {}", le);
                }
            }
            // Print surrounding source lines for context
            let lines: Vec<&str> = test_code.lines().collect();
            if e.line > 0 && e.line <= lines.len() {
                let idx = e.line - 1;
                let start = if idx >= 2 { idx - 1 } else { 0 };
                let end = std::cmp::min(lines.len() - 1, idx + 1);
                eprintln!("Context:");
                for i in start..=end {
                    eprintln!("{}: {}", i + 1, lines[i]);
                }
            }
            std::process::exit(1);
        }
    };
    match parser.parse() {
        Ok(statements) => {
            if debug {
                println!("Parsed {} statements", statements.len());
                for (i, stmt) in statements.iter().enumerate() {
                    println!("  Statement {}: {:?}", i, stmt);
                }
            }

            // Override just_run if @justrunit was found
            let effective_just_run = just_run || parser.just_run;

            let context = Context::create();
            let mut codegen = CodeGen::new(&context, effective_just_run, opt_level);
            codegen.set_has_main(parser.has_main);
            codegen.set_main_function_name(parser.main_function_name.clone());
            match codegen.generate(&statements) {
                Ok(_) => {
                    let ir = codegen.get_ir();

                    if emit_ir {
                        // Output LLVM IR
                        let output = output_file.unwrap_or_else(|| {
                            format!(
                                "{}.ll",
                                Path::new(&input_file)
                                    .file_stem()
                                    .unwrap()
                                    .to_string_lossy()
                            )
                        });
                        match fs::write(&output, &ir) {
                            Ok(_) => {
                                if debug {
                                    println!("Wrote LLVM IR to {}", output);
                                }
                            }
                            Err(e) => {
                                eprintln!("alshc: error writing {}: {}", output, e);
                                std::process::exit(1);
                            }
                        }
                    } else if emit_asm {
                        // Emit assembly file
                        match codegen.get_assembly() {
                            Ok(asm) => {
                                let output = output_file.unwrap_or_else(|| {
                                    format!(
                                        "{}.s",
                                        Path::new(&input_file)
                                            .file_stem()
                                            .unwrap()
                                            .to_string_lossy()
                                    )
                                });
                                match fs::write(&output, &asm) {
                                    Ok(_) => {
                                        if debug {
                                            println!("Wrote assembly to {}", output);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("alshc: error writing {}: {}", output, e);
                                        std::process::exit(1);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("alshc: assembly error: {}", e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        // Default: compile to executable
                        // Instead of using our internal object emission, write LLVM IR
                        // and invoke `clang` so the user can pass arbitrary LLVM/clang
                        // options (via args after `--`). This makes `alshc` behave like
                        // a true LLVM frontend: generate IR and let clang/LLVM do the
                        // final optimization/compilation steps.
                        let ir = codegen.get_ir();
                        let ir_file = format!(
                            "{}.ll",
                            Path::new(&input_file)
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                        );
                        let out_file = output_file.clone().unwrap_or_else(|| "a.out".to_string());

                        match fs::write(&ir_file, &ir) {
                            Ok(_) => {
                                // Compile the ALSH runtime
                                let runtime_src = "runtime/alsh_runtime.c";
                                let runtime_obj = "/tmp/alsh_runtime.o";

                                let cc_status = std::process::Command::new("clang")
                                    .arg("-c")
                                    .arg(runtime_src)
                                    .arg("-o")
                                    .arg(runtime_obj)
                                    .status();

                                match cc_status {
                                    Ok(s) if s.success() => {}
                                    _ => {
                                        eprintln!(
                                            "alshc: failed to compile runtime ({})",
                                            runtime_src
                                        );
                                        std::process::exit(1);
                                    }
                                }

                                // To avoid clang interpreting later object files as IR,
                                // first compile the IR to an object, then link objects.
                                // Prepare a temporary object filename for the IR.
                                let ir_obj = format!("{}.ir.o", Path::new(&input_file).file_stem().unwrap().to_string_lossy());

                                // Build clang args for compilation stage: filter out linking-only
                                // flags like `-o` and `-c` since we control output here.
                                let mut compile_args: Vec<String> = Vec::new();
                                let mut skip_next = false;
                                for a in &clang_args {
                                    if skip_next {
                                        skip_next = false;
                                        continue;
                                    }
                                    if a == "-o" {
                                        skip_next = true;
                                        continue;
                                    }
                                    if a == "-c" { continue; }
                                    compile_args.push(a.clone());
                                }

                                // clang -x ir -c ir_file -o ir_obj [user compile args]
                                let mut cc_ir = std::process::Command::new("clang");
                                for a in &compile_args {
                                    cc_ir.arg(a);
                                }
                                cc_ir.arg("-x").arg("ir").arg("-c").arg(&ir_file).arg("-o").arg(&ir_obj);

                                let status_compile_ir = cc_ir.status();
                                match status_compile_ir {
                                    Ok(s) if s.success() => {}
                                    Ok(s) => {
                                        eprintln!("alshc: clang (compile IR) failed with status: {}", s);
                                        std::process::exit(1);
                                    }
                                    Err(e) => {
                                        eprintln!("alshc: failed to run clang: {}", e);
                                        std::process::exit(1);
                                    }
                                }

                                // If user requested compile-only, emit the IR object and exit
                                let user_requested_compile_only = clang_args.iter().any(|s| s == "-c");
                                if user_requested_compile_only {
                                    // Move ir_obj to user-specified output if any
                                    if let Some(of) = &output_file {
                                        if let Err(e) = std::fs::rename(&ir_obj, of) {
                                            eprintln!("alshc: failed to move output object: {}", e);
                                            std::process::exit(1);
                                        }
                                    }
                                    return;
                                }

                                // Now link IR object with runtime object and any other user args
                                let mut link_cmd = std::process::Command::new("clang");
                                // pass through user args for linking stage
                                for a in &clang_args {
                                    link_cmd.arg(a);
                                }
                                
                                // Add standard library linking
                                // Add search path for alsh-std library
                                link_cmd.arg("-L").arg("alsh-std/impl/rust/target/release");
                                // Also try linking with the preprocessor lib directory
                                link_cmd.arg("-L").arg("alshpp");
                                
                                link_cmd.arg(&ir_obj).arg(runtime_obj).arg("-o").arg(&out_file)
                                    .arg("-lalsh_std")  // Link with our Rust stdlib
                                    .arg("-lc");        // Link with C stdlib

                                let status_link = link_cmd.status();
                                match status_link {
                                    Ok(s) if s.success() => {}
                                    Ok(s) => {
                                        eprintln!("alshc: clang (link) failed with status: {}", s);
                                        std::process::exit(1);
                                    }
                                    Err(e) => {
                                        eprintln!("alshc: failed to run clang: {}", e);
                                        std::process::exit(1);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("alshc: error writing {}: {}", ir_file, e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("alshc: codegen error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("alshc: parse error: {}", e);
            // show tokens and context for debugging
            match alshc::lexer::Lexer::new(&test_code).tokenize() {
                Ok(tokens) => {
                    eprintln!("Tokens:");
                    for (i, t) in tokens.iter().enumerate() {
                        eprintln!("  {}: {:?} ({}:{})", i, t.kind, t.line, t.col);
                    }
                }
                Err(le) => {
                    eprintln!("Lexer error: {}", le);
                }
            }
            let lines: Vec<&str> = test_code.lines().collect();
            if e.line > 0 && e.line <= lines.len() {
                let idx = e.line - 1;
                let start = if idx >= 2 { idx - 1 } else { 0 };
                let end = std::cmp::min(lines.len() - 1, idx + 1);
                eprintln!("Context:");
                for i in start..=end {
                    eprintln!("{}: {}", i + 1, lines[i]);
                }
            }
            if dump_tokens {
                std::process::exit(2);
            }
            std::process::exit(1);
        }
    }
}
