from pathlib import Path

p = Path('crates/nova-inspect/src/lib.rs')
text = p.read_text()
alias = '''type BuildParts = (\n    v1::Document,\n    Vec<v3::MatchPattern>,\n    Vec<v5::Closure>,\n    Vec<Vec<hir::CaptureMode>>,\n);\n\n'''
anchor = "struct Builder<'a> {"
if text.count(anchor) != 1:
    raise SystemExit(f'Builder anchor count={text.count(anchor)}')
text = text.replace(anchor, alias + anchor, 1)
old = '''    fn build_parts(\n        mut self,\n    ) -> Result<\n        (\n            v1::Document,\n            Vec<v3::MatchPattern>,\n            Vec<v5::Closure>,\n            Vec<Vec<hir::CaptureMode>>,\n        ),\n        InspectionError,\n    > {'''
new = '''    fn build_parts(mut self) -> Result<BuildParts, InspectionError> {'''
if text.count(old) != 1:
    # Before rustfmt the generated signature is one line.
    old = '''    fn build_parts(mut self) -> Result<(v1::Document, Vec<v3::MatchPattern>, Vec<v5::Closure>, Vec<Vec<hir::CaptureMode>>), InspectionError> {'''
if text.count(old) != 1:
    raise SystemExit(f'build_parts signature count={text.count(old)}')
text = text.replace(old, new, 1)
p.write_text(text)
