"""plain의 한계 — 오류가 '호스트 트레이스백'이라, DSL용 구조화 진단이 아니다.

Python에서 잘못된 소스를 eval하면 SyntaxError/traceback이 난다. 이는 사람이 읽는 텍스트일 뿐,
"몇 줄 몇 칸에서, 어떤 단계(reader/parse/eval)에서, 무엇이 문제인가"를 '데이터로' 소비하려면
트레이스백을 직접 파싱해야 한다. DSL 사용자에게 돌려줄 구조화 진단 표준이 없다.
"""
try:
    eval("1 +")            # 잘못된 소스
except SyntaxError as e:
    print("SyntaxError:", e.msg, "| lineno:", e.lineno, "| offset:", e.offset)
    print("구조화 진단(단계/캐럿/스키마)?: 표준 아님 (traceback을 직접 파싱해야 한다)")

print("\n결론: DSL 사용자에게 돌려줄 '구조화 위치 진단'이 기본 제공되지 않는다.")
