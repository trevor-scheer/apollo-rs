//! Example demonstrating how to parse GraphQL with source offsets for embedded documents.
//!
//! This is useful when parsing GraphQL that's extracted from template strings in TypeScript,
//! Rust string literals, or other host languages.

use apollo_compiler::parser::{Parser, SourceOffset};

fn main() {
    // Simulate parsing GraphQL from a TypeScript template string:
    //
    // const query = gql`
    //     type Query {
    //         field: InvalidType
    //     }
    // `;
    //
    // The GraphQL starts at line 2, column 5 in the TypeScript file.

    let graphql_source = r#"
    type Query {
        field: InvalidType
    }
"#;

    // Parse without offset - errors show line numbers relative to the GraphQL string
    println!("=== Parsing without offset ===");
    let mut parser = Parser::new();
    let result = parser.parse_schema(graphql_source, "example.ts");

    match result {
        Ok(_) => println!("Schema parsed successfully"),
        Err(errors) => {
            println!("Errors (relative to GraphQL string):");
            println!("{}", errors.errors);
        }
    }

    println!("\n=== Parsing with offset ===");
    // Parse with offset - errors show line numbers relative to the original file
    let mut parser = Parser::new().source_offset(SourceOffset { line: 2, column: 5 });
    let result = parser.parse_schema(graphql_source, "example.ts");

    match result {
        Ok(_) => println!("Schema parsed successfully"),
        Err(errors) => {
            println!("Errors (relative to original TypeScript file):");
            println!("{}", errors.errors);
            println!("\nNote: Line numbers now reflect the position in example.ts,");
            println!("not the position within the GraphQL string.");
        }
    }

    // Example with a syntax error
    println!("\n=== Parsing with syntax error and offset ===");
    let invalid_graphql = "type Query { field }"; // missing colon and type

    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 10,
        column: 20,
    });
    let result = parser.parse_schema(invalid_graphql, "app.ts");

    match result {
        Ok(_) => println!("Schema parsed successfully"),
        Err(errors) => {
            println!("Syntax error at line 10, column 20+ of app.ts:");
            println!("{}", errors.errors);
        }
    }
}
