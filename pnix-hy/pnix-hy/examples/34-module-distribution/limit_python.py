"""한계: 평범한 패키지는 '자기 증명 레인'을 찾지 못한다.

pnix-hy는 pip로 설치돼 일반 Python에서 import 돼도 원래 기능이 그대로 작동해야 한다. 그러려면
런타임이 자기 위치·hy-meta 증명 레인·proof-python을 스스로 찾아야 하는데 — plain 패키지엔 그런
레이어링·자기 발견이 없다(경로 하드코딩은 이사하면 깨진다).
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
# 평범한 패키지는 보통 경로를 하드코딩하거나, 설치 위치를 스스로 알지 못한다.
print("plain: 설치 위치·증명 레인·proof-python을 자기 발견하는 레이어링 없음.")
print("한계: 이사/설치 방식이 바뀌면 하드코딩 경로가 깨진다.")
