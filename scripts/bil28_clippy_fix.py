from pathlib import Path
import re

p = Path('crates/nova-inspect/src/lib.rs')
text = p.read_text()
alias = '''type BuildParts = (\n    v1::Document,\n    Vec<v3::MatchPattern>,\n    Vec<v5::Closure>,\n    Vec<Vec<hir::CaptureMode>>,\n);\n\n'''
anchor = "struct Builder<'a> {"
if text.count(anchor) != 1:
    raise SystemExit(f'Builder anchor count={text.count(anchor)}')
text = text.replace(anchor, alias + anchor, 1)
pattern = r"    fn build_parts\(\s*mut self,?\s*\)\s*->\s*Result<.*?InspectionError,?\s*>\s*\{"
text, count = re.subn(pattern, '    fn build_parts(mut self) -> Result<BuildParts, InspectionError> {', text, count=1, flags=re.S)
if count != 1:
    raise SystemExit(f'build_parts signature count={count}')
p.write_text(text)
