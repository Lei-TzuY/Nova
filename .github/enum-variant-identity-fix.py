from pathlib import Path

path = Path("crates/nova-sema/src/constant_condition.rs")
text = path.read_text()
old = """        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
        } if payload.is_none() => Some((*enumeration, *variant_index)),
"""
new = """        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
            ..
        } if payload.is_none() => Some((*enumeration, *variant_index)),
"""
if text.count(old) != 1:
    raise SystemExit(f"constant condition enum constructor anchor count: {text.count(old)}")
path.write_text(text.replace(old, new, 1))
