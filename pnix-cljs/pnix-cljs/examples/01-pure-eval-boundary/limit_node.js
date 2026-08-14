// plain Node — 게스트 언어 경계 없음
// process / fs / 전역에 항상 닿을 수 있다.
// 신뢰할 수 없는 문자열을 eval/Function 에 넘기면 안 된다.

console.log("plain Node 에는 임의 코드용 순수 게스트 샌드박스가 없다");
console.log("eval / new Function 은 process, require, 전역에 닿을 수 있다");
// 실행하지 않음:
//   eval("require('fs').readFileSync('/etc/passwd','utf8')")
//   new Function("return process.env")()
console.log("결론: 호스트 eval 이 아니라 명시적 게스트 평가기를 쓴다");
