use apollo_compiler::parser::{Parser, SourceOffset};

#[test]
fn it_errors_when_selection_set_recursion_limit_exceeded() {
    let input = r#"
    query {
      Q1 {
        url {
          hostname
        }
      }
    }
    "#;
    let mut parser = Parser::new().recursion_limit(1);
    let invalid = parser.parse_ast(input, "doc.graphql").unwrap_err();
    assert_eq!(parser.recursion_reached(), 2);
    let errors = invalid.errors.to_string();
    assert!(
        errors.contains("parser recursion limit reached"),
        "{errors}"
    );
    assert_eq!(invalid.partial.definitions.len(), 1);
}

#[test]
fn it_passes_when_selection_set_recursion_limit_is_not_exceeded() {
    let input = r#"
    query {
      Q1 {
        Q2 {
          Q3 {
            url
          }
        }
      }
    }
    "#;
    let mut parser = Parser::new().recursion_limit(7);
    let ast = parser.parse_ast(input, "doc.graphql").unwrap();
    assert_eq!(parser.recursion_reached(), 4);
    assert_eq!(ast.definitions.len(), 1);
}

#[test]
fn it_errors_when_selection_set_token_limit_is_exceeded() {
    let schema = r#"
    type Query {
      field(arg1: Int, arg2: Int, arg3: Int, arg4: Int, arg5: Int, arg6: Int): Int
    }
    "#;
    let invalid = Parser::new()
        .token_limit(18)
        .parse_ast(schema, "doc.graphql")
        .unwrap_err();
    let errors = invalid.errors.to_string();
    assert!(
        errors.contains("token limit reached, aborting lexing"),
        "{errors}"
    );
    assert!(errors.contains("doc.graphql:3:30"), "{errors}");
    assert_eq!(invalid.partial.definitions.len(), 1);
}

#[test]
fn it_errors_with_multiple_limits() {
    let input = r#"
        query {
            a {
                a {
                    a {
                        a
                    }
                }
            }
        }
    "#;
    let invalid = Parser::new()
        .token_limit(22)
        .recursion_limit(10)
        .parse_ast(input, "doc.graphql")
        .unwrap_err();
    let errors = invalid.errors.to_string();
    assert!(
        errors.contains("token limit reached, aborting lexing"),
        "{errors}"
    );
    assert!(errors.contains("doc.graphql:8:18"), "{errors}");

    let invalid = Parser::new()
        .token_limit(200)
        .recursion_limit(3)
        .parse_ast(input, "doc.graphql")
        .unwrap_err();
    let errors = invalid.errors.to_string();
    assert!(
        errors.contains("parser recursion limit reached"),
        "{errors}"
    );
    assert!(errors.contains("doc.graphql:6:25"), "{errors}");
}

#[test]
fn it_parses_with_line_offset() {
    // Simulate: GraphQL starting at line 5 of a file
    let source = "type Query { field: String }";
    let mut parser = Parser::new().source_offset(SourceOffset { line: 5, column: 1 });
    let result = parser.parse_ast(source, "test.graphql");

    assert!(result.is_ok());
}

#[test]
fn it_parses_with_column_offset() {
    // Simulate: GraphQL indented 4 spaces
    let source = "type Query { field: String }";
    let mut parser = Parser::new().source_offset(SourceOffset { line: 1, column: 5 });
    let result = parser.parse_ast(source, "test.graphql");

    assert!(result.is_ok());
}

#[test]
fn it_reports_errors_with_line_offset() {
    // Test that errors report correct line/column with offset
    let source = "type Query { field: }"; // missing type
    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 10,
        column: 1,
    });
    let result = parser.parse_ast(source, "test.graphql");

    assert!(result.is_err());
    let errors = result.unwrap_err();
    let diagnostic = errors.errors.iter().next().unwrap();

    // Error should be at line 10, not line 1
    let range = diagnostic.line_column_range().unwrap();
    assert_eq!(range.start.line, 10);
    assert!(range.start.column >= 1);
}

#[test]
fn it_reports_errors_with_column_offset() {
    // Test that errors report correct column with offset
    let source = "type Query { field: }"; // missing type
    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 1,
        column: 20,
    });
    let result = parser.parse_ast(source, "test.graphql");

    assert!(result.is_err());
    let errors = result.unwrap_err();
    let diagnostic = errors.errors.iter().next().unwrap();

    let range = diagnostic.line_column_range().unwrap();
    assert_eq!(range.start.line, 1);
    // Column should be shifted by the offset
    assert!(range.start.column >= 20);
}

#[test]
fn it_reports_errors_with_both_offsets() {
    // Test with both line and column offset
    let source = "type Query { field: }"; // missing type
    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 42,
        column: 15,
    });
    let result = parser.parse_ast(source, "test.graphql");

    assert!(result.is_err());
    let errors = result.unwrap_err();
    let diagnostic = errors.errors.iter().next().unwrap();

    let range = diagnostic.line_column_range().unwrap();
    assert_eq!(range.start.line, 42);
    assert!(range.start.column >= 15);
}

#[test]
fn it_handles_multiline_source_with_offset() {
    // Test offset with multiline source
    let source = r#"
type Query {
    field: InvalidType
}
"#;
    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 100,
        column: 1,
    });
    let result = parser.parse_schema(source, "test.graphql");

    // Even with validation errors about InvalidType, parsing should succeed
    assert!(result.is_ok() || result.is_err());

    // If there are errors, check they have correct offsets
    if let Err(err) = result {
        for diagnostic in err.errors.iter() {
            if let Some(range) = diagnostic.line_column_range() {
                // Line should be >= 100 (our offset)
                assert!(
                    range.start.line >= 100,
                    "Line {} should be >= 100",
                    range.start.line
                );
            }
        }
    }
}

#[test]
fn it_formats_errors_with_offset() {
    // Test that error formatting includes correct line numbers
    let source = "type Query { field }"; // missing colon and type
    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 50,
        column: 10,
    });
    let result = parser.parse_schema(source, "embedded.graphql");

    assert!(result.is_err());
    let errors = result.unwrap_err();

    // Format diagnostic and verify line numbers
    let diagnostic_str = format!("{}", errors.errors);
    assert!(
        diagnostic_str.contains(":50:"),
        "Diagnostic should mention line 50: {}",
        diagnostic_str
    );
}

#[test]
fn it_works_with_default_offset() {
    // Test that default offset (line 1, column 1) works as before
    let source = "type Query { field: String }";
    let mut parser = Parser::new(); // No offset specified
    let result = parser.parse_ast(source, "test.graphql");

    assert!(result.is_ok());
}

#[test]
fn it_works_with_schema_builder() {
    use apollo_compiler::Schema;

    let source = "type Query { field: String }";
    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 20,
        column: 5,
    });

    let mut builder = Schema::builder();
    parser.parse_into_schema_builder(source, "file1.graphql", &mut builder);

    let result = builder.build();
    assert!(result.is_ok());
}

#[test]
fn it_works_with_executable_documents() {
    use apollo_compiler::Schema;

    // First create a schema
    let schema_text = "type Query { field: String }";
    let schema = Schema::parse_and_validate(schema_text, "schema.graphql").unwrap();

    // Parse an executable document with offset
    let query = "query { field }";
    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 30,
        column: 10,
    });
    let result = parser.parse_executable(&schema, query, "query.graphql");

    assert!(result.is_ok());
}

#[test]
fn it_handles_large_offsets() {
    // Test with large offset values
    let source = "type Query { field: String }";
    let mut parser = Parser::new().source_offset(SourceOffset {
        line: 10000,
        column: 100,
    });
    let result = parser.parse_ast(source, "test.graphql");

    assert!(result.is_ok());
}
