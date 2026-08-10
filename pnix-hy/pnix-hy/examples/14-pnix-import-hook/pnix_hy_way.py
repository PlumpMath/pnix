"""pnix-hy의 방식 — .px 파일을 모듈로 import + sys.modules 트랜잭션(스냅샷/롤백).

install_pnix_import_hook은 (hy-meta의 호스트 import-hook 서비스 위에서) .px 파일을 pnix 런타임으로
컴파일해 진짜 Python 모듈처럼 로드한다. sys.modules를 스냅샷 -> import -> 롤백 하여 전역 상태를
원상복구하는 트랜잭션을 보여준다.

* hy-meta 트리 + Hy 1.3.0 필요 (`nix develop` / PNIX_HY_PYTHON, 저장소 루트에서 실행).
"""
import importlib
import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy import interop as io  # noqa: E402

host = io._host_import_hook()   # 스냅샷/롤백은 hy-meta 호스트 서비스가 제공(SR4)

with tempfile.TemporaryDirectory(prefix="pnix-import-demo-") as tmp:
    root = Path(tmp)
    (root / "demo_px_mod.px").write_text('{ answer = 42; label = "ok"; }\n', encoding="utf-8")

    snapshot = host.snapshot_sys_modules(["demo_px_mod"])       # 트랜잭션 시작
    with io.install_pnix_import_hook([root]) as finder:
        installed = finder in sys.meta_path
        mod = importlib.import_module("demo_px_mod")            # .px 를 모듈로 import
    removed = finder not in sys.meta_path
    answer, label = getattr(mod, "answer", None), getattr(mod, "label", None)
    host.rollback_sys_modules(snapshot)                        # 트랜잭션 롤백
    restored = "demo_px_mod" not in sys.modules                # 전역 상태 원복

print(".px import 결과: answer =", answer, "| label =", label)
print("hook 설치/해제:", installed, "/", removed, "| sys.modules 원복:", restored)

assert answer == 42 and label == "ok"
assert installed and removed and restored

print("\n결론: 다른-언어(.px) 모듈 로딩 + 스냅샷/롤백 트랜잭션으로 전역 상태를 안전하게 다룬다.")
