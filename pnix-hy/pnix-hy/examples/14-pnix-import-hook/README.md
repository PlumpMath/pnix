# 14 · pnix import hook — .px 모듈 로딩 + sys.modules 트랜잭션

> hy-meta 트리 + Hy 1.3.0 필요 (`nix develop` / `PNIX_HY_PYTHON`, 저장소 루트에서 실행).

## 쉽게 말하면 (비유)
**되돌리기 가능한 설치(트랜잭션)**. `.px` 파일을 모듈처럼 설치해 쓰고, 실패하면 설치 전 상태로
말끔히 **롤백**한다(전역 오염 방지).
```py
snap = host.snapshot_sys_modules(["demo_px_mod"])
with io.install_pnix_import_hook([root]):  import demo_px_mod   # .px -> module
host.rollback_sys_modules(snap)            # 원상복구
```
직관: **다른-언어 모듈 로딩 + 실패 시 전역 상태 롤백**.

## 무엇을
`.px`(pnix) 파일을 **Python 모듈처럼 import**하고, sys.modules를 **스냅샷→import→롤백**하는
트랜잭션으로 전역 상태를 안전하게 다룬다.

## plain의 한계 (`limit_python.py`)
Python importlib는 `.py`만 안다(.px 로더 없음). import 실패 시 전역(sys.modules)이 부분 오염될 수
있고, "스냅샷→시도→실패 시 롤백" 트랜잭션 프리미티브가 표준에 없다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `install_pnix_import_hook([root])` — hy-meta의 호스트 import-hook 서비스(SR4) 위에서 `.px`를
  pnix 런타임으로 컴파일해 모듈로 로드(컨텍스트 매니저: 설치/해제 자동).
- `snapshot_sys_modules` / `rollback_sys_modules` — import 트랜잭션(원상복구).

## 어디에 쓰나
- 순수 설정/DSL(.px)을 **모듈 시스템에 통합**해 코드처럼 import
- 플러그인/신뢰경계 로딩에서 **실패 시 전역 상태 롤백**(오염 방지)
- 격리된 import 실험(로드→검사→되돌리기)

## 실행
```sh
nix develop
python pnix-hy/examples/14-pnix-import-hook/pnix_hy_way.py
```
