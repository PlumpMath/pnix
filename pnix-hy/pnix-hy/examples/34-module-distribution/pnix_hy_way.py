"""pnix-hy 방식: 자기 발견 배포 레이어링 + 능력 티어 (proposal 0010).

`deployment_info()`는 런타임이 스스로 찾은 배치를 보고한다: 패키지 위치, PNIX_HY_HOME 환경(있으면),
hy 루트·hy-meta·proof-python, 그리고 지금 이 환경에서 어떤 **능력 티어**가 동작하는지
(core / projection / full_gate). 경로 하드코딩이 아니라 env/param 오라클로 자기 발견한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
import pnix_hy.deploy as dep

info = dep.deployment_info()
print("package_path:", info["package_path"])
print("PNIX_HY_HOME env:", info["pnix_hy_home_env"], "| hy_root:", info["hy_root"])
print("hy_meta_found:", info["hy_meta_found"], "| proof_python_found:", info["proof_python_found"])
print("능력 티어:", info["tiers"])

# 스키마·필수 키가 자기 발견으로 채워진다(경로 하드코딩 아님)
assert info["schema"].startswith("pnix-hy.deployment")
for k in ("package_path", "hy_root", "tiers"):
    assert k in info
# core 티어는 순수 파이썬만으로 언제나 동작
assert info["tiers"]["core"] is True
print("→ 런타임이 위치·증명 레인·티어를 스스로 발견 — 경로 하드코딩 없이 배포 가능.")
