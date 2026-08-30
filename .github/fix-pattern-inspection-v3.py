from pathlib import Path

p = Path('.github/pattern-inspection-v3.py')
text = p.read_text()
old = 'replace_once("crates/nova-inspect/src/lib.rs", old_match, new_match)'
new = '''p = Path("crates/nova-inspect/src/lib.rs")
source = p.read_text()
start = source.index("                        let binding = match (&variant.payload")
end = source.index("                    })();", start)
p.write_text(source[:start] + new_match + source[end:])'''
if text.count(old) != 1:
    raise RuntimeError(f'expected one old match replacement call, found {text.count(old)}')
p.write_text(text.replace(old, new, 1))
