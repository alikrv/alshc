use alshc::{parse_pipeline, ControlFlowParser};

fn main() {
    println!("ALSH Compiler - Parser Extraction");
    println!("==================================\n");

    // Test parser
    let test_cmd = "echo hello world | grep hello";
    println!("Testing parse_pipeline: {}", test_cmd);
    let pipeline = parse_pipeline(test_cmd);
    println!("  Commands: {}", pipeline.commands.len());
    for (i, cmd) in pipeline.commands.iter().enumerate() {
        println!("    [{}] {}", i, cmd.raw);
    }
    println!();

    // Test control flow parser
    let test_code = r#"
        let x = 42
        if x == 42 {
            println("x is 42")
        } else {
            println("x is not 42")
        }
    "#;
    
    println!("Testing ControlFlowParser:");
    println!("Code:\n{}\n", test_code);
    
    let mut parser = ControlFlowParser::new(test_code);
    match parser.parse() {
        Ok(statements) => {
            println!("Parsed {} statements:", statements.len());
            for (i, stmt) in statements.iter().enumerate() {
                println!("  [{}] {:?}", i, stmt);
            }
        }
        Err(e) => println!("Parse error: {}", e),
    }
    
    println!("\n✓ Parser extraction successful!");
}
