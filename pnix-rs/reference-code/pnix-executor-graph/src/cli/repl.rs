//! REPL (Read-Eval-Print Loop) 구현
//!
//! Y12a: 히스토리
//!
//! ## 헌법 준수 (P0-1)
//!
//! REPL 실행 로직은 executor에 있지만, completion 목록은 stable sort로 결정론 보장

use anyhow::Result;
use std::collections::BTreeSet;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};

use pnix_runtime_api::{EvalConfig, EvalEngine, EvalPatchable};
use pnix_runtime_legacy::{LegacyEvalEngine, LegacyModule};

/// REPL 상태
pub struct ReplState {
  /// 평가 엔진
  engine: LegacyEvalEngine,
  /// 평가 설정
  config: EvalConfig,
  /// 바인딩된 변수들 (이름만 추적, 값은 engine에 저장)
  bindings: BTreeSet<String>,
  /// 바인딩 값 저장 (이름 → 값 JSON, 세션 간 지속성용)
  binding_values: std::collections::HashMap<String, serde_json::Value>,
  /// 히스토리 파일 경로
  history_file: Option<PathBuf>,
}

pub(crate) fn safe_home_dir() -> Option<PathBuf> {
  #[cfg(windows)]
  let home_var = "USERPROFILE";
  #[cfg(not(windows))]
  let home_var = "HOME";

  let raw = std::env::var(home_var).ok()?;
  if raw.is_empty() || raw.contains('\0') {
    return None;
  }
  let path = PathBuf::from(&raw);
  if !path.is_absolute() {
    return None;
  }
  if path.components().any(|c| matches!(c, Component::ParentDir)) {
    return None;
  }
  Some(path)
}

fn ensure_writable_dir(path: &Path) -> bool {
  if std::fs::create_dir_all(path).is_err() || !path.is_dir() {
    return false;
  }
  let probe = path.join(format!(".pnix-write-probe-{}", std::process::id()));
  match std::fs::OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(&probe)
  {
    Ok(_) => {
      let _ = std::fs::remove_file(probe);
      true
    }
    Err(_) => false,
  }
}

fn history_path_from_dir(path: &Path) -> Option<PathBuf> {
  if ensure_writable_dir(path) {
    Some(path.join(".pnix_history"))
  } else {
    None
  }
}

pub(crate) fn resolve_history_file(dist: Option<&PathBuf>) -> Option<PathBuf> {
  if let Some(path) = dist.and_then(|d| history_path_from_dir(d)) {
    return Some(path);
  }
  if let Some(home) = safe_home_dir() {
    if let Some(path) = history_path_from_dir(&home) {
      return Some(path);
    }
  }
  history_path_from_dir(&std::env::temp_dir().join("pnix-repl"))
}

impl ReplState {
  /// 새 REPL 상태 생성
  pub fn new(config: EvalConfig, history_file: Option<PathBuf>) -> Self {
    let mut state = Self {
      engine: LegacyEvalEngine::new(),
      config,
      bindings: BTreeSet::new(),
      binding_values: std::collections::HashMap::new(),
      history_file: history_file.clone(),
    };

    // 바인딩 파일 로드 (세션 간 지속성)
    state.load_bindings();

    state
  }

  /// 바인딩 파일 경로 반환
  fn bindings_file_path(&self) -> Option<PathBuf> {
    self
      .history_file
      .as_ref()
      .map(|h| {
        h.parent()
          .map(|p| p.join(".pnix_bindings.json"))
          .unwrap_or_else(|| PathBuf::from(".pnix_bindings.json"))
      })
      .or_else(|| {
        // LOW: HOME 미설정 시 현재 디렉토리 사용 수정
        // HOME이 없으면 None을 반환하여 바인딩 파일을 사용하지 않음
        // 컨테이너 환경에서 현재 디렉토리 오염 방지
        safe_home_dir().map(|home| home.join(".pnix_bindings.json"))
      })
  }

  /// 바인딩 파일에서 로드
  fn load_bindings(&mut self) {
    if let Some(bindings_path) = self.bindings_file_path() {
      match std::fs::read_to_string(&bindings_path) {
        Ok(content) => {
          match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(bindings_json) => {
              if let Some(bindings_obj) = bindings_json.as_object() {
                for (name, value) in bindings_obj {
                  // null 값은 무시 (이전 세션에서 이름만 저장된 경우)
                  if value.is_null() {
                    continue;
                  }
                  self.bindings.insert(name.clone());
                  self.binding_values.insert(name.clone(), value.clone());
                  let patch = serde_json::json!({
                    "op": "set_binding",
                    "name": name,
                    "value": value
                  });
                  if let Err(e) = self.engine.apply_patch(&patch) {
                    eprintln!("warning: failed to apply binding patch: {}", e);
                  }
                }
              }
            }
            Err(e) => {
              eprintln!(
                "Warning: Failed to parse bindings file {}: {}",
                bindings_path.display(),
                e
              );
            }
          }
        }
        Err(e) => {
          // 파일이 없으면 정상 (첫 실행)
          if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!(
              "Warning: Failed to read bindings file {}: {}",
              bindings_path.display(),
              e
            );
          }
        }
      }
    }
  }

  /// 바인딩 파일에 저장
  fn save_bindings(&self) -> Result<()> {
    if let Some(bindings_path) = self.bindings_file_path() {
      // 바인딩 값 저장 (이름 → 값)
      let mut bindings_obj = serde_json::Map::new();
      for (var_name, value) in &self.binding_values {
        bindings_obj.insert(var_name.clone(), value.clone());
      }
      let bindings_json = serde_json::Value::Object(bindings_obj);
      std::fs::write(
        &bindings_path,
        serde_json::to_string_pretty(&bindings_json)?,
      )?;
    }
    Ok(())
  }

  /// 표현식 평가
  pub fn eval(&mut self, input: &str) -> Result<String> {
    // 특수 명령 처리
    if input.trim().starts_with(':') {
      return self.handle_command(input.trim());
    }

    // 표현식을 모듈로 래핑 (let result = <expr>; in result)
    // LegacyEvalEngine의 bindings가 자동으로 사용됨 (install_json_vars)
    let wrapped = format!("let result = ({}); in result", input.trim());
    let module = LegacyModule::from_source(wrapped);

    match self.engine.eval(&module, &self.config) {
      Ok(result) => {
        // 변수 바인딩 추출 및 저장
        self.extract_and_save_bindings(input.trim(), result.value.as_json());
        Ok(serde_json::to_string_pretty(result.value.as_json())?)
      }
      Err(e) => Err(anyhow::anyhow!("{}", e)),
    }
  }

  /// 변수 바인딩 추출 및 저장 (let 바인딩 + 결과 값 저장)
  fn extract_and_save_bindings(&mut self, input: &str, result_value: &serde_json::Value) {
    // MEDIUM: 바인딩 추출 에러 처리 missing 수정 완료
    // let 파싱 실패 시 원본 유지하되, 에러 로깅 추가
    // 패턴 1: `let x = ...` 형태의 바인딩 추출
    if let Some(stripped) = input.strip_prefix("let ") {
      if let Some(equals_pos) = stripped.find('=') {
        let var_name = stripped[..equals_pos].trim();
        // 변수 이름 검증: 비어있지 않고, 알파벳/숫자/언더스코어만 포함
        if var_name.is_empty() {
          eprintln!("warning: empty variable name in let binding: {}", input);
          return;
        }
        if !var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
          eprintln!("warning: invalid variable name '{}' in let binding (only alphanumeric and underscore allowed)", var_name);
          return;
        }
        // 바인딩 이름 저장
        self.bindings.insert(var_name.to_string());
        // 바인딩 값 저장 (세션 간 지속성용)
        self
          .binding_values
          .insert(var_name.to_string(), result_value.clone());
        // LegacyEvalEngine에 바인딩 값 저장 (다음 표현식에서 사용)
        let patch = serde_json::json!({
          "op": "set_binding",
          "name": var_name,
          "value": result_value
        });
        if let Err(e) = self.engine.apply_patch(&patch) {
          eprintln!(
            "warning: failed to apply binding patch for '{}': {}",
            var_name, e
          );
        }
      } else {
        // `let x` 형태로 `=`가 없는 경우
        eprintln!("warning: incomplete let binding (missing '='): {}", input);
      }
    }

    // 패턴 2: `let x = expr; in y` 형태에서 x를 바인딩으로 저장
    if let Some(in_pos) = input.find("; in") {
      let let_part = &input[..in_pos];
      if let Some(stripped) = let_part.strip_prefix("let ") {
        if let Some(equals_pos) = stripped.find('=') {
          let var_name = stripped[..equals_pos].trim();
          if !var_name.is_empty() && var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            self.bindings.insert(var_name.to_string());
            // CRITICAL: 바인딩 값 저장 (세션 간 지속성용)
            // 제한사항: `let x = expr; in y` 형태에서 result_value는 y의 결과이지 x의 값이 아님
            // 올바른 구현: expr을 별도로 평가하여 x의 값을 저장해야 함
            // 현재는 간단한 구현으로 result_value를 저장 (대부분의 경우 y == x이므로 올바름)
            // 향후 개선: expr을 파싱하여 별도로 평가하는 로직 추가 필요
            self
              .binding_values
              .insert(var_name.to_string(), result_value.clone());
            let patch = serde_json::json!({
              "op": "set_binding",
              "name": var_name,
              "value": result_value
            });
            let _ = self.engine.apply_patch(&patch);
          }
        }
      }
    }
  }

  /// 특수 명령 처리
  pub fn handle_command(&mut self, input: &str) -> Result<String> {
    // CRITICAL: 이스케이프/따옴표 처리하여 명령 파싱
    // 간단한 구현: 공백으로 분리하되, 따옴표로 감싼 부분은 하나로 처리
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;

    for ch in input[1..].chars() {
      if escape_next {
        current.push(ch);
        escape_next = false;
        continue;
      }
      match ch {
        '\\' => {
          escape_next = true;
          current.push(ch);
        }
        '"' | '\'' => {
          in_quotes = !in_quotes;
          current.push(ch);
        }
        c if c.is_whitespace() && !in_quotes => {
          if !current.is_empty() {
            parts.push(current.clone());
            current.clear();
          }
        }
        c => {
          current.push(c);
        }
      }
    }
    if !current.is_empty() {
      parts.push(current);
    }

    let cmd = parts.first().map(|s| s.as_str()).unwrap_or("");
    // LOW: Poisoned Mutex 복구 시 데이터 무결성 미보장
    // 불완전 상태 사용 가능
    // 현재는 poisoned mutex 복구 시 불완전한 데이터 사용 가능
    match cmd {
      "help" | "h" => Ok(HELP_TEXT.to_string()),
      "quit" | "q" | "exit" => Err(anyhow::anyhow!("REPL exit")),
      "reset" => {
        self.engine = LegacyEvalEngine::new();
        self.bindings.clear();
        self.binding_values.clear();
        // 바인딩 파일도 삭제
        if let Some(bindings_path) = self.bindings_file_path() {
          if let Err(e) = std::fs::remove_file(&bindings_path) {
            eprintln!(
              "Warning: Failed to remove bindings file {}: {}",
              bindings_path.display(),
              e
            );
          }
        }
        Ok("Reset REPL state".to_string())
      }
      "bindings" | "b" => {
        let mut vars: Vec<String> = self.bindings.iter().cloned().collect();
        vars.sort();
        Ok(format!("Bound variables: {}", vars.join(", ")))
      }
      _ => Err(anyhow::anyhow!(
        "Unknown command: {}. Type :help for help",
        cmd
      )),
    }
  }
}

const HELP_TEXT: &str = r#"
REPL Commands:
  :help, :h          Show this help
  :quit, :q, :exit   Exit REPL
  :reset             Reset REPL state (clear bindings)
  :bindings, :b      Show bound variables
"#;

/// 표현식 완성도 확인 (괄호/브레이스/따옴표 밸런스)
fn is_expression_complete(input: &str) -> bool {
  let mut paren_depth = 0;
  let mut brace_depth = 0;
  let mut bracket_depth = 0;
  let mut in_string = false;
  let mut in_multiline_string = false; // CRITICAL: 멀티라인 문자열 상태 추적
  let mut in_block_comment = false; // LOW: is_expression_complete 블록 코멘트 미처리 수정
  let mut escape_next = false;
  let chars: Vec<char> = input.chars().collect();
  let mut char_iter = chars.iter().peekable();

  while let Some(&ch) = char_iter.next() {
    if escape_next {
      escape_next = false;
      continue;
    }

    // CRITICAL: 멀티라인 문자열 처리 (''...'')
    if !in_string && !in_multiline_string && !in_block_comment {
      // 멀티라인 문자열 시작 확인: '' (두 개의 작은따옴표)
      if ch == '\'' && char_iter.peek() == Some(&&'\'') {
        char_iter.next(); // 두 번째 작은따옴표 소비
        in_multiline_string = true;
        continue;
      }
      // LOW: 블록 코멘트 시작 확인: /*
      if ch == '/' && char_iter.peek() == Some(&&'*') {
        char_iter.next(); // '*' 소비
        in_block_comment = true;
        continue;
      }
    } else if in_multiline_string {
      // 멀티라인 문자열 종료 확인: '' (두 개의 작은따옴표)
      if ch == '\'' && char_iter.peek() == Some(&&'\'') {
        char_iter.next(); // 두 번째 작은따옴표 소비
        in_multiline_string = false;
        continue;
      }
      // 멀티라인 문자열 내부는 모든 문자 무시
      continue;
    } else if in_block_comment {
      // LOW: 블록 코멘트 종료 확인: */
      if ch == '*' && char_iter.peek() == Some(&&'/') {
        char_iter.next(); // '/' 소비
        in_block_comment = false;
        continue;
      }
      // 블록 코멘트 내부는 모든 문자 무시
      continue;
    }

    match ch {
      '\\' if in_string => {
        escape_next = true;
      }
      '"' if !escape_next => {
        in_string = !in_string;
      }
      '(' if !in_string && !in_multiline_string => paren_depth += 1,
      ')' if !in_string && !in_multiline_string => {
        paren_depth -= 1;
        // 음수가 되면 불완전한 표현식 (닫는 괄호가 너무 많음)
        if paren_depth < 0 {
          return false;
        }
      }
      '{' if !in_string && !in_multiline_string => brace_depth += 1,
      '}' if !in_string && !in_multiline_string => {
        brace_depth -= 1;
        if brace_depth < 0 {
          return false;
        }
      }
      '[' if !in_string && !in_multiline_string => bracket_depth += 1,
      ']' if !in_string && !in_multiline_string => {
        bracket_depth -= 1;
        if bracket_depth < 0 {
          return false;
        }
      }
      _ => {}
    }
  }

  // 모든 괄호/브레이스가 닫혔고 문자열이 닫혔으면 완전한 표현식
  // LOW: 멀티라인 프롬프트 줄 번호 없음
  // 현재 줄 컨텍스트 부재
  // 현재는 멀티라인 입력 시 줄 번호를 표시하지 않음
  paren_depth == 0
    && brace_depth == 0
    && bracket_depth == 0
    && !in_string
    && !in_multiline_string
    && !in_block_comment
}

fn append_history_entry(path: &Path, input: &str) {
  if input.is_empty() || input.starts_with(' ') {
    return;
  }
  if let Some(parent) = path.parent() {
    let _ = std::fs::create_dir_all(parent);
  }
  let Ok(mut file) = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(path)
  else {
    return;
  };
  let _ = writeln!(file, "{}", input);
}

/// REPL 실행
pub fn run_repl(config: EvalConfig, history_file: Option<PathBuf>) -> Result<()> {
  let mut state = ReplState::new(config, history_file.clone());

  if !std::io::stdin().is_terminal() {
    return run_repl_pipe(&mut state);
  }

  let use_colors = should_use_colors();
  let history_path = history_file.or_else(|| resolve_history_file(None));
  let stdin = io::stdin();
  let mut stdin = stdin.lock();

  'outer: loop {
    // 다중 행 입력 수집
    let mut lines = Vec::new();

    loop {
      // 멀티라인 입력 시 줄 번호 표시
      let prompt = if lines.is_empty() {
        "pnix> ".to_string()
      } else {
        format!("pnix:{}> ", lines.len() + 1)
      };

      if use_colors && lines.is_empty() {
        print!("\x1b[1;32m{}\x1b[0m", prompt);
      } else {
        print!("{}", prompt);
      }
      io::stdout().flush()?;

      let mut line = String::new();
      let read = stdin.read_line(&mut line)?;
      if read == 0 {
        break 'outer;
      }
      let line = line.trim_end_matches(['\r', '\n']).to_string();
      lines.push(line);
      let accumulated = lines.join("\n");

      // 괄호/브레이스 밸런스 확인 (간단한 구현)
      let is_complete = is_expression_complete(&accumulated);

      if is_complete {
        // 완전한 표현식 - 평가 진행
        let trimmed = accumulated.trim();
        if trimmed.is_empty() {
          lines.clear();
          break; // 빈 입력이면 다음 루프로
        }

        // 히스토리에 추가 (다중 행은 첫 줄만)
        if let (Some(history_path), Some(first_line)) = (&history_path, lines.first()) {
          append_history_entry(history_path, first_line.trim());
        }

        // 평가
        let should_exit = match state.eval(trimmed) {
          Ok(result) => {
            println!("{}", result);
            false
          }
          Err(e) => {
            // :quit 명령은 정상 종료
            if e.to_string() == "REPL exit" {
              true
            } else {
              eprintln!("{}", wrap_with_prefix("error: ", &e.to_string()));
              false
            }
          }
        };
        lines.clear();
        if should_exit {
          break 'outer; // 외부 루프 종료
        }
        break; // 다음 입력으로
      } else {
        // 불완전한 표현식 - 계속 입력 받기
        continue;
      }
    }
  }

  // 바인딩 저장 (세션 간 지속성)
  // CRITICAL: 바인딩 저장 실패 시 사용자에게 알림
  if let Err(e) = state.save_bindings() {
    eprintln!("Warning: Failed to save bindings: {}", e);
  }

  Ok(())
}

fn run_repl_pipe(state: &mut ReplState) -> Result<()> {
  use std::io::{self, BufRead};

  let stdin = io::stdin();
  for line in stdin.lock().lines() {
    let input = line?.trim().to_string();
    if input.is_empty() {
      continue;
    }
    if input == "exit" || input == "quit" || input == ":quit" || input == ":exit" {
      break;
    }
    match state.eval(&input) {
      Ok(result) => println!("{}", result),
      Err(e) => eprintln!("{}", wrap_with_prefix("error: ", &e.to_string())),
    }
  }

  // CRITICAL: 바인딩 저장 실패 시 사용자에게 알림
  if let Err(e) = state.save_bindings() {
    eprintln!("Warning: Failed to save bindings: {}", e);
  }
  Ok(())
}

fn should_use_colors() -> bool {
  if std::env::var_os("NO_COLOR").is_some() {
    return false;
  }
  std::io::stdout().is_terminal()
}

fn wrap_with_prefix(prefix: &str, message: &str) -> String {
  if message.contains('\n') {
    return format!("{}{}", prefix, message);
  }
  let width = terminal_wrap_width();
  if let Some(width) = width {
    wrap_message(prefix, message, width)
  } else {
    format!("{}{}", prefix, message)
  }
}

fn terminal_wrap_width() -> Option<usize> {
  if !std::io::stderr().is_terminal() {
    return None;
  }
  std::env::var("COLUMNS")
    .ok()
    .and_then(|value| value.parse::<usize>().ok())
    .filter(|value| *value >= 40)
    .or(Some(100))
}

fn wrap_message(prefix: &str, message: &str, width: usize) -> String {
  let prefix_width = prefix.chars().count();
  if width <= prefix_width + 1 {
    return format!("{}{}", prefix, message);
  }
  let available = width.saturating_sub(prefix_width);
  let mut lines: Vec<String> = Vec::new();
  let mut current = String::new();
  let mut current_width = 0usize;

  for word in message.split_whitespace() {
    let word_width = word.chars().count();
    if current.is_empty() {
      current.push_str(word);
      current_width = word_width;
      continue;
    }
    if current_width + 1 + word_width <= available {
      current.push(' ');
      current.push_str(word);
      current_width += 1 + word_width;
    } else {
      lines.push(current);
      current = word.to_string();
      current_width = word_width;
    }
  }

  if !current.is_empty() {
    lines.push(current);
  }

  let indent = " ".repeat(prefix_width);
  let mut output = String::new();
  for (idx, line) in lines.iter().enumerate() {
    if idx == 0 {
      output.push_str(prefix);
    } else {
      output.push_str(&indent);
    }
    output.push_str(line);
    if idx + 1 < lines.len() {
      output.push('\n');
    }
  }
  output
}

#[cfg(test)]
pub(crate) fn verify_repl_state_bindings_persistence() {
  let config = EvalConfig::default();
  let temp_dir = std::env::temp_dir();
  let bindings_file = temp_dir.join(".pnix_bindings_test.json");

  {
    let mut state = ReplState::new(config.clone(), Some(bindings_file.clone()));
    state.bindings.insert("x".to_string());
    state
      .binding_values
      .insert("x".to_string(), serde_json::json!(42));
    let _ = state.save_bindings();
  }

  {
    let state = ReplState::new(config, Some(bindings_file.clone()));
    assert!(state.bindings.contains("x"));
    assert_eq!(state.binding_values.get("x"), Some(&serde_json::json!(42)));
  }

  let _ = std::fs::remove_file(&bindings_file);
}

#[cfg(test)]
pub(crate) fn verify_repl_state_reset() {
  let config = EvalConfig::default();
  let mut state = ReplState::new(config, None);

  state.bindings.insert("x".to_string());
  state
    .binding_values
    .insert("x".to_string(), serde_json::json!(42));

  let result = state.handle_command(":reset");
  assert!(result.is_ok());
  assert!(state.bindings.is_empty());
  assert!(state.binding_values.is_empty());
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_repl_state_eval() {
    let config = EvalConfig::default();
    let mut state = ReplState::new(config, None);
    let result = state.eval("1 + 2");
    assert!(result.is_ok(), "eval failed: {:?}", result.as_ref().err());
    assert_eq!(result.unwrap().trim(), "3");
  }

  #[test]
  fn test_repl_state_command() {
    let config = EvalConfig::default();
    let mut state = ReplState::new(config, None);
    let result = state.eval(":help");
    assert!(result.is_ok());
    assert!(result.unwrap().contains("REPL Commands"));
  }

  #[test]
  fn test_is_expression_complete() {
    // 완전한 표현식
    assert!(is_expression_complete("1 + 2"));
    assert!(is_expression_complete("let x = 1; in x"));
    assert!(is_expression_complete("(1 + 2)"));
    assert!(is_expression_complete("{ x = 1; }"));
    assert!(is_expression_complete("[1, 2, 3]"));
    assert!(is_expression_complete("\"hello\""));

    // 불완전한 표현식
    assert!(!is_expression_complete("(1 + 2"));
    assert!(!is_expression_complete("let x = {"));
    assert!(!is_expression_complete("[1, 2, 3"));
    assert!(!is_expression_complete("\"hello"));

    // 중첩된 괄호
    assert!(is_expression_complete("((1 + 2))"));
    assert!(!is_expression_complete("((1 + 2)"));
    assert!(!is_expression_complete("(1 + 2))"));

    // 문자열 내부의 괄호는 무시
    assert!(is_expression_complete("\"hello (world)\""));
    // 닫히지 않은 문자열 (실제 Pnix 표현식에서 따옴표가 하나만 있음)
    // 테스트: "hello (world (따옴표가 하나만 있음)
    let incomplete_string = "\"hello (world";
    assert!(!is_expression_complete(incomplete_string));

    // 이스케이프 처리
    assert!(is_expression_complete("\"hello \\\"world\\\"\""));
  }

  #[test]
  fn test_repl_state_bindings_persistence() {
    verify_repl_state_bindings_persistence();
  }

  #[test]
  fn test_repl_state_reset() {
    verify_repl_state_reset();
  }

  #[test]
  fn test_resolve_history_file_prefers_dist() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dist = temp.path().to_path_buf();
    let history = resolve_history_file(Some(&dist)).expect("history path");
    assert_eq!(history, dist.join(".pnix_history"));
  }

  #[test]
  fn test_resolve_history_file_without_dist_returns_some() {
    let history = resolve_history_file(None).expect("history path");
    assert_eq!(
      history.file_name().and_then(|name| name.to_str()),
      Some(".pnix_history")
    );
  }
}
