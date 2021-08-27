use std::cell::RefCell;
use std::rc::Rc;

use crate::lexer::Lexer;
use crate::TokenKind;
use crate::{lexer, Token};

pub use generated::syntax_kind::SyntaxKind;
pub use language::{SyntaxElement, SyntaxElementChildren, SyntaxNodeChildren, SyntaxToken};
pub use syntax_tree::SyntaxTree;

pub(crate) use language::{GraphQLLanguage, SyntaxNode};
pub(crate) use syntax_tree::SyntaxTreeBuilder;
pub(crate) use token_text::TokenText;

mod argument;
mod directive;
mod enum_;
mod field;
mod fragment;
mod generated;
mod input;
mod interface;
mod language;
mod name;
mod object;
mod operation;
mod scalar;
mod schema;
mod selection;
mod syntax_tree;
mod token_text;
mod ty;
mod union_;
mod utils;
mod value;
mod variable;

/// Parse text into an AST.
#[derive(Debug)]
pub struct Parser {
    /// input tokens, including whitespace,
    /// in *reverse* order.
    tokens: Vec<lexer::Token>,
    /// the in-progress tree.
    builder: Rc<RefCell<SyntaxTreeBuilder>>,
    /// the list of syntax errors we've accumulated
    /// so far.
    errors: Vec<crate::Error>,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let lexer = Lexer::new(input);

        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        for s in lexer.tokens().to_owned() {
            match s {
                Ok(t) => tokens.push(t),
                Err(e) => errors.push(e),
            }
        }

        tokens.reverse();
        errors.reverse();

        Self {
            tokens,
            builder: Rc::new(RefCell::new(SyntaxTreeBuilder::new())),
            errors,
        }
    }

    pub fn parse(mut self) -> SyntaxTree {
        let guard = self.start_node(SyntaxKind::DOCUMENT);

        loop {
            match self.peek_data() {
                None => break,
                Some(node) => match node.as_str() {
                    "fragment" => fragment::fragment_definition(&mut self),
                    "directive" => directive::directive_definition(&mut self),
                    "schema" => schema::schema_definition(&mut self),
                    "scalar" => scalar::scalar_type_definition(&mut self),
                    "type" => object::object_type_definition(&mut self),
                    "interface" => interface::interface_type_definition(&mut self),
                    "union" => union_::union_type_definition(&mut self),
                    "enum" => enum_::enum_type_definition(&mut self),
                    "input" => input::input_object_type_definition(&mut self),
                    "query" | "mutation" | "subscription" | "{" => {
                        operation::operation_definition(&mut self)
                    }
                    _ => break,
                },
            }
        }

        guard.finish_node();

        let builder = Rc::try_unwrap(self.builder)
            .expect("More than one reference to builder left")
            .into_inner();
        builder.finish(self.errors)
    }

    /// Consume a token from the lexer and insert it into the AST.
    pub(crate) fn bump(&mut self, kind: SyntaxKind) {
        let token = self.tokens.pop().unwrap();
        self.builder.borrow_mut().token(kind, token.data());
    }

    /// Consume a token from the lexer.
    pub(crate) fn pop(&mut self) -> Token {
        self.tokens.pop().unwrap()
    }

    /// Insert a token into the AST.
    pub(crate) fn push_ast(&mut self, kind: SyntaxKind, token: Token) {
        self.builder.borrow_mut().token(kind, token.data())
    }

    pub(crate) fn push_err(&mut self, err: crate::error::Error) {
        self.errors.push(err);
    }

    pub(crate) fn start_node(&mut self, kind: SyntaxKind) -> NodeGuard {
        self.builder.borrow_mut().start_node(kind);
        NodeGuard::new(self.builder.clone())
    }

    pub(crate) fn peek(&self) -> Option<TokenKind> {
        self.tokens.last().map(|token| token.kind())
    }

    pub(crate) fn peek_n(&self, n: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.tokens.len() - n)
            .map(|token| token.kind())
    }

    pub(crate) fn peek_data(&self) -> Option<String> {
        self.tokens.last().map(|token| token.data().to_string())
    }

    pub(crate) fn peek_data_n(&self, n: usize) -> Option<String> {
        self.tokens
            .get(self.tokens.len() - n)
            .map(|token| token.data().to_string())
    }
}

#[must_use]
pub(crate) struct NodeGuard {
    builder: Rc<RefCell<SyntaxTreeBuilder>>,
}

impl NodeGuard {
    fn new(builder: Rc<RefCell<SyntaxTreeBuilder>>) -> Self {
        Self { builder }
    }

    pub(crate) fn finish_node(self) {
        drop(self);
    }
}

impl Drop for NodeGuard {
    fn drop(&mut self) {
        self.builder.borrow_mut().finish_node();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn smoke_directive_definition_with_errors() {
        let input = "\"Description\"directive @example(isTreat: Boolean, treatKind: String)";
        let parser = Parser::new(input);

        println!("{:?}", parser.parse());
    }
}
