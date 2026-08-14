# plain .NET — 기본적으로 게스트 언어 경계 없음

- `Microsoft.CodeAnalysis.CSharp.Scripting` / `Eval` 은 제한 호스트를
  직접 만들지 않으면 프로세스·파일시스템·네트워크에 닿을 수 있다.
- BCL 에 “순수 Nix 유사 게스트” 는 없다.
- 결론: 신뢰할 수 없는 표현 언어에는 명시적 게스트 평가기(`pnix-clr`)를 쓰고,
  호스트 스크립팅을 샌드박스로 취급하지 않는다.
