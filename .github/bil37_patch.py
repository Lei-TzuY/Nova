from pathlib import Path

p = Path("crates/nova-parser/src/parser.rs")
s = p.read_text()

old = '''            if self.at(TokenKind::LeftParen) {
                if POSTFIX_BINDING_POWER < minimum {
                    break;
                }
                left = self.parse_call_expression(left)?;
                continue;
            }
            if self.at(TokenKind::Dot) {'''
new = '''            if self.at(TokenKind::LeftParen) {
                if POSTFIX_BINDING_POWER < minimum {
                    break;
                }
                left = self.parse_call_expression(left, Vec::new())?;
                continue;
            }
            if self.at(TokenKind::Less)
                && POSTFIX_BINDING_POWER >= minimum
                && matches!(left.kind, ExpressionKind::Name(_))
            {
                let checkpoint = self.position;
                let diagnostic_checkpoint = self.diagnostics.len();
                if let Some(type_arguments) = self.parse_explicit_call_type_arguments() {
                    if self.at(TokenKind::LeftParen) {
                        left = self.parse_call_expression(left, type_arguments)?;
                        continue;
                    }
                }
                self.position = checkpoint;
                self.diagnostics.truncate(diagnostic_checkpoint);
            }
            if self.at(TokenKind::Dot) {'''
assert old in s, "postfix anchor missing"
s = s.replace(old, new, 1)

old = '''    fn parse_call_expression(&mut self, callee: Expression) -> Option<Expression> {
        self.expect(TokenKind::LeftParen, "to start the argument list")?;'''
new = '''    fn parse_explicit_call_type_arguments(&mut self) -> Option<Vec<TypeRef>> {
        self.expect(TokenKind::Less, "to start explicit call type arguments")?;
        if self.at(TokenKind::Greater) {
            return None;
        }
        let mut arguments = Vec::new();
        loop {
            arguments.push(self.parse_type_ref("as an explicit call type argument")?);
            if self.consume(TokenKind::Comma).is_none() {
                break;
            }
            if self.at(TokenKind::Greater) {
                break;
            }
        }
        self.expect(TokenKind::Greater, "to close explicit call type arguments")?;
        Some(arguments)
    }

    fn parse_call_expression(
        &mut self,
        callee: Expression,
        type_arguments: Vec<TypeRef>,
    ) -> Option<Expression> {
        self.expect(TokenKind::LeftParen, "to start the argument list")?;'''
assert old in s, "call function anchor missing"
s = s.replace(old, new, 1)

old = '''            kind: ExpressionKind::Call {
                callee: Box::new(callee),
                arguments,
            },'''
new = '''            kind: ExpressionKind::Call {
                callee: Box::new(callee),
                type_arguments,
                arguments,
            },'''
assert old in s, "call construction anchor missing"
p.write_text(s.replace(old, new, 1))
