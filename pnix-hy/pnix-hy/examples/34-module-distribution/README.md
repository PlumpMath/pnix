# 34. 모듈 배포 — 자기 발견 레이어링 + 티어 (proposal 0010)

## 무엇을
런타임이 스스로 찾은 배치를 보고하는 `deployment_info()`: 패키지 위치, `PNIX_HY_HOME` 환경, hy 루트·
hy-meta·proof-python, 그리고 지금 환경에서 동작하는 **능력 티어**(core / projection / full_gate).

## 왜
pnix-hy는 pip로 설치돼 일반 Python에서 import 돼도 원래 기능이 그대로 작동해야 한다. 그러려면 런타임이
자기 위치·증명 레인·proof-python을 **스스로 발견**해야 한다 — 경로 하드코딩은 이사/설치 방식이 바뀌면
깨진다(이 저장소는 하드코딩 repo 경로가 0, env/param 오라클로 발견).

## 티어
| 티어 | 동작 조건 |
|---|---|
| **core** | 순수 Python만으로 언제나 |
| **projection** | Hy 투영 레인 사용 가능 시 |
| **full_gate** | 4-lane 미러/게이트까지 가능 시 |

## 한 줄
> 런타임이 위치·증명 레인·능력 티어를 **스스로 발견**하면 — 경로 하드코딩 없이 pip 배포돼도 기능이
> 그대로 살아난다.

## 경계
- `--deployment` CLI가 이 정보를 낸다. 관련: 0011 능력 인덱스(`--capabilities`). 정본 평가기·4-lane 미러 무접촉.
