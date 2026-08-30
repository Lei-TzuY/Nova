from pathlib import Path

path = Path("crates/nova-sema/src/analyzer.rs")
text = path.read_text()
old = "        assert!(matches!(callee.kind, ExpressionKind::Function(_)));"
new = "        assert!(matches!(callee.kind, ExpressionKind::Function { .. }));"
count = text.count(old)
if count != 1:
    raise SystemExit(f"analyzer function pattern assertion: expected one anchor, found {count}")
path.write_text(text.replace(old, new, 1))
