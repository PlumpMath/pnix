"""plain의 한계 — 한 프로그램을 '모든 단면(source/form/ast/ir/effect/value/witness)'으로
   일관되게 물화(reify)하는 통일 표면이 없다.

Python에서 이런 정보를 모으려면 ast, dis, symtable, hashlib, inspect를 '따로따로' 호출해
직접 꿰매야 하고, 각 단계가 같은 정본/해시 규약을 공유하지 않는다.
"""
import ast
import dis
import io

src = "a = 1\nb = a + 2"
tree = ast.dump(ast.parse(src))              # AST는 여기
buf = io.StringIO()
dis.dis(compile(src, "<ex>", "exec"), file=buf)  # 바이트코드는 저기
print("ast 있음:", len(tree) > 0, "| dis 있음:", len(buf.getvalue()) > 0)
print("통일 물화 표면?: 없음 (ast/dis/symtable/inspect를 직접 꿰매야 한다)")
print("공유 정본/해시 규약?: 없음 (단면들이 같은 witness로 묶이지 않는다)")

print("\n결론: source→form→ast→ir→value→witness를 한 규약으로 묶는 표면이 기본에 없다.")
