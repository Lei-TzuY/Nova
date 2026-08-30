from pathlib import Path
p = Path('crates/nova-parser/src/parser.rs')
text = p.read_text()
old = '    MatchArm, Name, Parameter, Program, Record, RecordField, RecordLiteralField, Statement,\n'
new = '    MatchArm, MatchPattern, Name, Parameter, Program, Record, RecordField, RecordLiteralField, Statement,\n'
if text.count(old) != 1:
    raise RuntimeError('parser import anchor changed')
p.write_text(text.replace(old, new, 1))
