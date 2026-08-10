"""한계: 인터프리터를 그냥 두면 '해석 오버헤드'가 매 실행에 남는다.

인터프리터는 프로그램을 실행할 때마다 노드마다 tag를 재판별한다. 특화(1차 Futamura 사영)로 그
오버헤드를 '제거'해야 컴파일한 것과 같아지는데 — plain Python엔 "특화 결과가 인터프리터 계층을 정말
없앴는가"를 검증할 척도가 없다. 이게 Jones-optimality(특화기 품질의 gold standard)다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
# 인터프리터 없이 프로그램 p를 그냥 실행하면, 매번 dispatch 오버헤드가 든다.
# "특화가 그 계층을 없앴는지"를 확인할 수단이 plain Python엔 없다.
print("plain: 특화기 품질(해석 계층 제거)을 검증할 척도 없음.")
print("한계: ir(specialize(int,p)) == ir(p) 인지 (Jones-optimality) 확인 불가.")
