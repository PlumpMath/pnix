"""pnix-hy 방식: Hy 언어 구성체 프로젝션 5종.

defmacro/import/매크로 전개 단계/quasiquote 템플릿/reader macro를 각각
구조화된 값으로 뽑아낸다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy import hy_mirror as hm  # noqa: F401 - import order matters (circular init)
import pnix_hy as ph

dm = ph.hy_defmacro_projection("(defmacro my-when [c &rest body] `(if ~c (do ~@body) None))")
print(f"defmacro: name={dm['defmacros'][0]['name']} params={dm['defmacros'][0]['params']}")
assert dm["defmacro_count"] == 1

imp = ph.hy_import_projection("(import os)")
print(f"import: stage={imp['entries'][0]['stage']} python={imp['entries'][0]['python_source']!r}")
assert imp["entries"][0]["stage"] == "run-time"

trace = ph.hy_macro_step_trace("(when True 1)")
print(f"macro-steps: head={trace['forms'][0]['head']} steps={trace['forms'][0]['step_count']} fixpoint={trace['forms'][0]['fixpoint']}")
assert trace["forms"][0]["is_macro"] and trace["forms"][0]["fixpoint"]

qq = ph.hy_quasiquote_projection("`(1 2 ~(+ 1 2))")
print(f"quasiquote: template_kind={qq['template_kind']}")
assert qq["template_kind"] == "quasiquote"

rm = ph.hy_reader_macro_projection("(defreader up [expr] `(.upper ~expr))")
print(f"reader-macro: name={rm['defreaders'][0]['name']} registered_ok={rm['defreaders'][0]['registered_ok']}")
assert rm["defreaders"][0]["registered_ok"]

print("→ Hy 구성체(매크로/import/quasiquote/reader-macro/전개 단계) 다섯 종류 전부 구조화된 값.")
