from pathlib import Path
p = Path("crates/nova-parser/tests/lambda_expressions.rs")
s = p.read_text()
old = "let ExpressionKind::Call { callee, arguments } = &tail.kind else {"
new = "let ExpressionKind::Call { callee, arguments, .. } = &tail.kind else {"
assert old in s, "lambda call pattern anchor missing"
p.write_text(s.replace(old, new, 1))
