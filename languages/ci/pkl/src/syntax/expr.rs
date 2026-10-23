//! Reading past what the host does not need: expressions, types and the
//! members that are neither properties nor entries.
//!
//! The file is already known valid (pklr accepted it), so these only have
//! to find where each construct ENDS -- and keep every object inside it,
//! because a step can sit in an object built anywhere. An expression is
//! operands joined by binary operators; after an operand, a `.member`, a
//! `!!`, an `is`/`as` type, an amending `{ ... }`, or a call or subscript
//! ON THE SAME LINE continues it. A `[` or `(` on the next line starts a
//! new member, as in Pkl.

use pklr::lexer::TokenKind;

use super::{Body, Reader, Value, literal};

#[cfg(test)]
mod tests;

impl Reader<'_> {
    /// The value after `=`.
    pub(super) fn value(&mut self) -> Value {
        match self.kind() {
            Some(TokenKind::StringLit(_) | TokenKind::InterpolatedString(_)) => {
                let token = self.tokens.get(self.at).cloned();
                self.at += 1;
                match token {
                    Some(token) if !self.continues() => Value::Str(literal(&token, self.src)),
                    _ => Value::Other(self.continued()),
                }
            }
            // `new T` is always followed by its body in valid Pkl.
            Some(TokenKind::KwNew) => {
                self.at += 1;
                self.skip_until_brace();
                self.at += 1;
                let body = self.members(true);
                self.amended(Value::New(body))
            }
            _ => Value::Other(self.expression_bodies()),
        }
    }

    /// `value`, unless an operator or postfix continues the expression it
    /// starts: then the objects of the whole expression.
    pub(super) fn amended(&mut self, value: Value) -> Value {
        if !self.continues() {
            return value;
        }
        let mut bodies = match value {
            Value::Object(body) | Value::New(body) => vec![body],
            Value::Other(bodies) => bodies,
            Value::Str(_) => Vec::new(),
        };
        bodies.extend(self.continued());
        Value::Other(bodies)
    }

    /// The rest of an expression whose first operand is behind the cursor.
    fn continued(&mut self) -> Vec<Body> {
        let mut out = Vec::new();
        self.postfix(&mut out);
        if self.binary() {
            self.at += 1;
            self.expression(&mut out);
        }
        out
    }

    /// Whether the token at the cursor continues the expression before it.
    fn continues(&self) -> bool {
        self.binary() || self.postfix_starts()
    }

    fn binary(&self) -> bool {
        matches!(
            self.kind(),
            Some(
                TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::StarStar
                    | TokenKind::TildeSlash
                    | TokenKind::EqEq
                    | TokenKind::BangEq
                    | TokenKind::Lt
                    | TokenKind::Gt
                    | TokenKind::LtEq
                    | TokenKind::GtEq
                    | TokenKind::AmpAmp
                    | TokenKind::PipePipe
                    | TokenKind::QuestionQuestion
                    | TokenKind::PipeGt
                    | TokenKind::Arrow
                    | TokenKind::KwElse
            )
        )
    }

    fn postfix_starts(&self) -> bool {
        match self.kind() {
            Some(
                TokenKind::Dot
                | TokenKind::QuestionDot
                | TokenKind::BangBang
                | TokenKind::KwIs
                | TokenKind::KwAs
                | TokenKind::LBrace,
            ) => true,
            Some(TokenKind::LParen | TokenKind::LBracket) => self.same_line(),
            _ => false,
        }
    }

    /// Whether the token at the cursor starts on the line the one before
    /// it ends on.
    fn same_line(&self) -> bool {
        let prev = self.at.checked_sub(1).and_then(|i| self.tokens.get(i));
        prev.zip(self.tokens.get(self.at))
            .and_then(|(prev, next)| self.src.get(prev.end..next.offset))
            .is_some_and(|gap| !gap.contains('\n'))
    }

    /// Whether the token before the cursor joins a type to what follows.
    fn after_joiner(&self) -> bool {
        self.at
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .is_some_and(|t| matches!(t.kind, TokenKind::Pipe | TokenKind::Dot))
    }

    /// The objects inside one whole expression at the cursor.
    pub(super) fn expression_bodies(&mut self) -> Vec<Body> {
        let mut out = Vec::new();
        self.expression(&mut out);
        out
    }

    fn expression(&mut self, out: &mut Vec<Body>) {
        loop {
            self.operand(out);
            self.postfix(out);
            if !self.binary() {
                return;
            }
            self.at += 1;
        }
    }

    fn operand(&mut self, out: &mut Vec<Body>) {
        while matches!(self.kind(), Some(TokenKind::Bang | TokenKind::Minus)) {
            self.at += 1;
        }
        match self.kind() {
            Some(TokenKind::KwIf | TokenKind::KwLet) => {
                self.at += 1;
                self.balanced(out);
                self.expression(out);
            }
            Some(
                TokenKind::KwThrow
                | TokenKind::KwTrace
                | TokenKind::KwRead
                | TokenKind::KwReadOrNull
                | TokenKind::KwReadGlob
                | TokenKind::KwImport
                | TokenKind::KwImportStar,
            ) => {
                self.at += 1;
                self.balanced(out);
            }
            Some(TokenKind::KwNew) => {
                self.at += 1;
                self.skip_until_brace();
                self.object(out);
            }
            Some(TokenKind::LBrace) => self.object(out),
            Some(TokenKind::LParen | TokenKind::LBracket | TokenKind::LPred) => self.balanced(out),
            Some(TokenKind::Eof | TokenKind::RBrace) | None => {}
            Some(_) => self.at += 1,
        }
    }

    fn postfix(&mut self, out: &mut Vec<Body>) {
        loop {
            match self.kind() {
                Some(TokenKind::Dot | TokenKind::QuestionDot) => self.at += 2,
                Some(TokenKind::BangBang) => self.at += 1,
                Some(TokenKind::KwIs | TokenKind::KwAs) => self.skip_type(),
                Some(TokenKind::LBrace) => self.object(out),
                Some(TokenKind::LParen | TokenKind::LBracket) if self.same_line() => {
                    self.balanced(out);
                }
                _ => return,
            }
        }
    }

    /// `{ ... }` at the cursor, as a body.
    fn object(&mut self, out: &mut Vec<Body>) {
        if matches!(self.kind(), Some(TokenKind::LBrace)) {
            self.at += 1;
            out.push(self.members(true));
        }
    }

    /// A bracketed run at the cursor -- `(...)`, `[...]`, `[[...]]` --
    /// through its close, keeping the objects inside it.
    fn balanced(&mut self, out: &mut Vec<Body>) {
        let mut depth = 0usize;
        loop {
            match self.kind() {
                Some(TokenKind::LParen | TokenKind::LBracket) => depth += 1,
                Some(TokenKind::LPred) => depth += 2,
                Some(TokenKind::RParen | TokenKind::RBracket) => depth = depth.saturating_sub(1),
                // Inside the brackets: an object argument.
                Some(TokenKind::LBrace) => {
                    self.object(out);
                    continue;
                }
                Some(TokenKind::Eof) | None => return,
                Some(_) => {}
            }
            self.at += 1;
            if depth == 0 {
                return;
            }
        }
    }

    /// Past the `]` matching the `[` just consumed: the objects inside.
    pub(super) fn skip_to_close(&mut self) -> Vec<Body> {
        let mut inside = Vec::new();
        let mut depth = 1usize;
        while depth > 0 {
            match self.kind() {
                Some(TokenKind::LBracket | TokenKind::LParen) => depth += 1,
                Some(TokenKind::RBracket | TokenKind::RParen) => depth -= 1,
                Some(TokenKind::LBrace) => {
                    self.object(&mut inside);
                    continue;
                }
                Some(TokenKind::Eof) | None => break,
                Some(_) => {}
            }
            self.at += 1;
        }
        inside
    }

    /// Past a type after `:`, `is` or `as`. At depth 0 a type never runs
    /// onto the next line unless a `.` or `|` carries it there.
    pub(super) fn skip_type(&mut self) {
        self.at += 1;
        let mut depth = 0usize;
        let mut first = true;
        loop {
            if !first && depth == 0 && !self.same_line() && !self.after_joiner() {
                return;
            }
            first = false;
            match self.kind() {
                Some(TokenKind::Lt | TokenKind::LParen) => depth += 1,
                Some(TokenKind::Gt | TokenKind::RParen) if depth > 0 => depth -= 1,
                Some(TokenKind::Comma) if depth > 0 => {}
                Some(
                    TokenKind::Ident(_)
                    | TokenKind::Dot
                    | TokenKind::Pipe
                    | TokenKind::QuestionMark
                    | TokenKind::StringLit(_)
                    | TokenKind::Star
                    | TokenKind::Arrow
                    | TokenKind::KwModule,
                ) => {}
                _ => return,
            }
            self.at += 1;
        }
    }

    /// Past a type name to the `{` of `new T { ... }`, or to the first
    /// token that cannot be part of one.
    fn skip_until_brace(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.kind() {
                Some(TokenKind::LBrace | TokenKind::Eof) | None => return,
                Some(TokenKind::Lt) => depth += 1,
                Some(TokenKind::Gt) => depth = depth.saturating_sub(1),
                Some(TokenKind::Ident(_) | TokenKind::Dot | TokenKind::Comma) => {}
                Some(_) if depth > 0 => {}
                Some(_) => return,
            }
            self.at += 1;
        }
    }

    /// A member that is neither a property nor an entry: the objects in
    /// it.
    pub(super) fn other(&mut self) -> Vec<Body> {
        let mut out = Vec::new();
        match self.kind() {
            Some(TokenKind::LPred) => {
                self.balanced(&mut out);
                match self.assigned() {
                    Value::Object(body) | Value::New(body) => out.push(body),
                    Value::Other(bodies) => out.extend(bodies),
                    Value::Str(_) => {}
                }
            }
            Some(TokenKind::DotDotDot | TokenKind::QuestionDotDotDot) => {
                self.at += 1;
                self.expression(&mut out);
            }
            Some(TokenKind::KwWhen | TokenKind::KwFor) => {
                self.at += 1;
                self.balanced(&mut out);
                self.object(&mut out);
                if matches!(self.kind(), Some(TokenKind::KwElse)) {
                    self.at += 1;
                    self.object(&mut out);
                }
            }
            Some(TokenKind::KwImport | TokenKind::KwImportStar) => {
                self.at += 1;
                while matches!(
                    self.kind(),
                    Some(TokenKind::StringLit(_) | TokenKind::KwAs | TokenKind::Ident(_))
                ) {
                    self.at += 1;
                }
            }
            Some(TokenKind::KwClass) => {
                self.skip_until_brace_any();
                self.object(&mut out);
            }
            Some(TokenKind::KwTypeAlias) => {
                while !matches!(self.kind(), Some(TokenKind::Equals | TokenKind::Eof) | None) {
                    self.at += 1;
                }
                self.skip_type();
            }
            Some(TokenKind::KwFunction) => {
                while !matches!(self.kind(), Some(TokenKind::Equals | TokenKind::Eof) | None) {
                    self.at += 1;
                }
                self.at += 1;
                self.expression(&mut out);
            }
            Some(TokenKind::At) => {
                // `@Name` or `@a.b.Name`, and maybe its body.
                self.at += 2;
                while matches!(self.kind(), Some(TokenKind::Dot)) {
                    self.at += 2;
                }
                if self.same_line() {
                    self.object(&mut out);
                }
            }
            _ => self.expression(&mut out),
        }
        out
    }

    /// Past a class header (name, type parameters, `extends`) to its `{`.
    fn skip_until_brace_any(&mut self) {
        while !matches!(self.kind(), Some(TokenKind::LBrace | TokenKind::Eof) | None) {
            self.at += 1;
        }
    }
}
