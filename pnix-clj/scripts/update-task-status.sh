#!/usr/bin/env bash
# 작업 상태 업데이트 스크립트
# 사용법: ./scripts/update-task-status.sh <task-id> <status> [todo.md 경로]
#
# 상태:
#   - start: [ ] → [🚧]
#   - done: [🚧] → [✅]
#   - complete: [✅] → [x] (머지 완료)

set -euo pipefail

TASK_ID="${1:-}"
STATUS="${2:-}"
TODO_FILE="${3:-todo.md}"

if [ -z "$TASK_ID" ] || [ -z "$STATUS" ]; then
    echo "사용법: $0 <task-id> <status> [todo.md 경로]"
    echo ""
    echo "상태:"
    echo "  start    - 작업 시작 ([ ] → [🚧])"
    echo "  done     - 작업 완료 ([🚧] → [✅])"
    echo "  complete - 머지 완료 ([✅] → [x])"
    echo ""
    echo "예시:"
    echo "  $0 Y07a start"
    echo "  $0 Y07a done"
    echo "  $0 Y07a complete"
    exit 1
fi

if [ ! -f "$TODO_FILE" ]; then
    echo "오류: $TODO_FILE 파일을 찾을 수 없습니다."
    exit 1
fi

# 백업 생성
cp "$TODO_FILE" "${TODO_FILE}.bak"

case "$STATUS" in
    start)
        # [ ] 또는 [ ] → [🚧]
        sed -i.tmp "s/^- \[ \] \[.*\] $TASK_ID/- [🚧] [&]/" "$TODO_FILE"
        sed -i.tmp "s/^- \[🚧\] \[.*\] $TASK_ID/- [🚧] [&]/" "$TODO_FILE"
        echo "✅ 작업 시작: $TASK_ID"
        ;;
    done)
        # [🚧] → [✅]
        sed -i.tmp "s/^- \[🚧\] \[.*\] $TASK_ID/- [✅] [&]/" "$TODO_FILE"
        echo "✅ 작업 완료: $TASK_ID (PR 생성 필요)"
        ;;
    complete)
        # [✅] → [x]
        sed -i.tmp "s/^- \[✅\] \[.*\] $TASK_ID/- [x] [&]/" "$TODO_FILE"
        echo "✅ 머지 완료: $TASK_ID (todo.md 완료 아카이브 섹션으로 이동 권장)"
        ;;
    *)
        echo "오류: 알 수 없는 상태 '$STATUS'"
        echo "사용 가능한 상태: start, done, complete"
        exit 1
        ;;
esac

# 임시 파일 정리
rm -f "${TODO_FILE}.tmp"

echo "📝 $TODO_FILE 업데이트 완료"
echo "💡 변경 사항 확인: git diff $TODO_FILE"
