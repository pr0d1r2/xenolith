//! just's expressions (GRAMMAR.md `expression` down to `value`), read to
//! be checked: an assignment, a setting, a default or an interpolation
//! holding something just would reject fails the file. Nothing an
//! expression computes is kept.

use xenolith_lang_api::Result;

use super::Parser;
use crate::lexer::Kind;

#[cfg(test)]
mod tests;

impl Parser<'_> {
    /// `disjunct (|| expression)?`.
    pub(super) fn expression(&mut self) -> Result<()> {
        self.disjunct()?;
        if self.at("||")? {
            self.bump()?;
            self.expression()?;
        }
        Ok(())
    }

    /// `comparison (&& disjunct)?`.
    fn disjunct(&mut self) -> Result<()> {
        self.comparison()?;
        if self.at("&&")? {
            self.bump()?;
            self.disjunct()?;
        }
        Ok(())
    }

    /// `conjunct ((== | != | =~ | !~) conjunct)?`.
    fn comparison(&mut self) -> Result<()> {
        self.conjunct()?;
        for op in ["==", "!=", "=~", "!~"] {
            if self.at(op)? {
                self.bump()?;
                return self.conjunct();
            }
        }
        Ok(())
    }

    /// A conditional, `/ expression`, or a value maybe joined to more.
    fn conjunct(&mut self) -> Result<()> {
        if self.at("if")? {
            return self.conditional();
        }
        if self.at("/")? {
            self.bump()?;
            return self.expression();
        }
        self.value()?;
        for op in ["+", "++", "/"] {
            if self.at(op)? {
                self.bump()?;
                return self.expression();
            }
        }
        Ok(())
    }

    /// `if expression { expression } (else (conditional | { expression }))?`.
    fn conditional(&mut self) -> Result<()> {
        self.bump()?;
        self.expression()?;
        self.block()?;
        if self.at("else")? {
            self.bump()?;
            if self.at("if")? {
                return self.conditional();
            }
            self.block()?;
        }
        Ok(())
    }

    /// `{ expression }`.
    fn block(&mut self) -> Result<()> {
        self.expect("{")?;
        self.expression()?;
        self.expect("}")?;
        Ok(())
    }

    /// `! value`, a call, a backtick, a name, a list, a string or a
    /// parenthesised expression.
    pub(super) fn value(&mut self) -> Result<()> {
        let token = self.peek()?;
        match (token.kind, token.text(self.src)) {
            (Kind::Punct, "!") => {
                self.bump()?;
                self.value()
            }
            (Kind::Name, _) => {
                self.bump()?;
                if self.at("(")? {
                    self.bump()?;
                    self.sequence(")")?;
                    self.expect(")")?;
                }
                Ok(())
            }
            (Kind::Str | Kind::Backtick, _) => self.bump().map(|_| ()),
            (Kind::Punct, "[") => {
                self.bump()?;
                self.sequence("]")?;
                self.expect("]").map(|_| ())
            }
            (Kind::Punct, "(") => {
                self.bump()?;
                self.expression()?;
                self.expect(")").map(|_| ())
            }
            _ => Err(self.error(token.span.start, "expected an expression")),
        }
    }

    /// `(expression (, expression)* ,?)?` up to (not past) `close`.
    fn sequence(&mut self, close: &str) -> Result<()> {
        while !self.at(close)? {
            self.expression()?;
            if !self.at(close)? {
                self.expect(",")?;
            }
        }
        Ok(())
    }
}
