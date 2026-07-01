use alshc::{CodeGen};
use inkwell::context::Context;
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
    let mut emit_ir = false;
    let mut emit_asm = false;
    let mut just_run = false;
    let mut output_file: Option<String> = None;
    let mut input_file: Option<String> = None;

    // Parse arguments
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--debug" => debug = true,
            "-ir" => emit_ir = true,
            "-S" => emit_asm = true,
            "--just-run" | "-jr" => just_run = true,
            "-o" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                }
            }
            arg if !arg.starts_with('-') => input_file = Some(arg.to_string()),
            _ => {}
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

    let test_code = match fs::read_to_string(&input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("alshc: error: {}: {}", input_file, e);
            std::process::exit(1);
        }
    };

    if debug {
        println!("ALSH Compiler");
        println!("============");
        println!("Compiling code from {}:\n{}", input_file, test_code);
    }

    let mut parser = match alshc::parser2::Parser::from_source(&test_code) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("alshc: parse error: {}", e);
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
            let mut codegen = CodeGen::new(&context, effective_just_run);
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
                        match codegen.get_object() {
                            Ok(obj) => {
                                let obj_file = format!(
                                    "{}.o",
                                    Path::new(&input_file)
                                        .file_stem()
                                        .unwrap()
                                        .to_string_lossy()
                                );
                                let out_file = output_file.unwrap_or_else(|| "a.out".to_string());

                                // Write object file
                                match fs::write(&obj_file, &obj) {
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

                                        // Link with clang
                                        let _status = std::process::Command::new("clang")
                                            .arg(&obj_file)
                                            .arg(runtime_obj)
                                            .arg("-o")
                                            .arg(&out_file)
                                            .arg("-lc")
                                            .status();
                                    }
                                    Err(e) => {
                                        eprintln!("alshc: error writing {}: {}", obj_file, e);
                                        std::process::exit(1);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("alshc: object file error: {}", e);
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
            std::process::exit(1);
        }
    }
}
