//! 레거시 호환성: 레거시 pnix 실행기와의 호환성 유지

use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use pnix_runtime_legacy::config::{
  load_config, resolve_format, resolve_stack_limit, resolve_timeout,
};
use pnix_runtime_legacy::eval_and_format_with_stack;
use pnix_runtime_legacy::web_server::{run as run_web_server, ServerConfig};

/// 기본 파일 확장자
const DEFAULT_EXTENSION: &str = "px";

/// 레거시 평가 인자: 레거시 실행기 스타일 인자
#[derive(Debug, Default)]
struct LegacyEvalArgs {
  file: Option<PathBuf>,
  expr: Option<String>,
  json: bool,
  raw: bool,
  pretty: bool,
  edn: bool,
  timeout_ms: Option<u64>,
  stack_limit: Option<usize>,
}

#[derive(Debug)]
struct LegacyServeArgs {
  file: Option<PathBuf>,
  expr: Option<String>,
  host: String,
  port: u16,
  max_body_bytes: usize,
  stack_limit: Option<usize>,
}

#[derive(Debug)]
enum InputSource {
  File(PathBuf),
  Expr(String),
  Stdin,
}

pub(super) fn is_serve_invocation(argv: &[String]) -> bool {
  matches!(argv.get(1).map(String::as_str), Some("serve"))
}

pub(super) fn should_use_eval_compat(argv: &[String]) -> bool {
  if argv.len() <= 1 || is_serve_invocation(argv) {
    return false;
  }
  if matches!(argv.get(1).map(String::as_str), Some("fmt" | "lint")) {
    return false;
  }
  if has_explicit_mode_flags(argv) {
    return false;
  }

  let has_legacy_style = argv.iter().skip(1).any(|arg| {
    matches!(
      arg.as_str(),
      "-e"
        | "--expr"
        | "--source"
        | "--json"
        | "--raw"
        | "--pretty"
        | "--edn"
        | "--timeout"
        | "--stack-limit"
    ) || !arg.starts_with('-')
  });
  if !has_legacy_style {
    return false;
  }

  parse_legacy_eval_args(argv).is_ok()
}

pub(super) fn run_eval_compat(argv: &[String]) -> Result<()> {
  let args = parse_legacy_eval_args(argv)?;
  let config = load_config().map_err(|e| anyhow::anyhow!("{}", e))?;
  let _timeout_ms = resolve_timeout(args.timeout_ms, &config);
  let stack_limit = resolve_stack_limit(args.stack_limit, &config);
  let format = resolve_format(args.json, args.raw, args.pretty, args.edn, &config);

  let input_source = resolve_input(args.file, args.expr)?;
  let (input, is_file) = read_eval_input(input_source)?;
  let output = eval_and_format_with_stack(&input, is_file, format, Some(stack_limit))
    .map_err(|e| anyhow::anyhow!("{}", e))?;
  println!("{}", output);
  Ok(())
}

pub(super) fn run_serve_compat(argv: &[String]) -> Result<()> {
  let args = parse_legacy_serve_args(argv)?;
  let config = load_config().map_err(|e| anyhow::anyhow!("{}", e))?;
  let stack_limit = resolve_stack_limit(args.stack_limit, &config);

  let input_source = resolve_input(args.file, args.expr)?;
  let (source, import_base) = read_source_text(input_source)?;
  let server_config = ServerConfig {
    host: args.host,
    port: args.port,
    max_body_bytes: args.max_body_bytes,
    source,
    import_base,
    stack_limit,
  };

  let exit_code = run_web_server(server_config);
  if exit_code != ExitCode::SUCCESS {
    anyhow::bail!("pnix serve exited with status {:?}", exit_code);
  }
  Ok(())
}

fn has_explicit_mode_flags(argv: &[String]) -> bool {
  argv.iter().skip(1).any(|arg| {
    matches!(
      arg.as_str(),
      "--mode"
        | "--run"
        | "--interpret"
        | "--compile"
        | "--legacy-eval"
        | "--legacy-frp"
        | "--ct"
        | "--llvm"
        | "--test"
        | "--fmt"
        | "--lint"
        | "--dist"
        | "--engine"
        | "--emit"
    )
  })
}

fn parse_legacy_eval_args(argv: &[String]) -> Result<LegacyEvalArgs> {
  let mut args = LegacyEvalArgs::default();
  let mut i = 1usize;
  while i < argv.len() {
    match argv[i].as_str() {
      "-e" | "--expr" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("{} requires a value", argv[i - 1]);
        }
        args.expr = Some(argv[i].clone());
      }
      "--source" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("--source requires a value");
        }
        args.file = Some(PathBuf::from(argv[i].clone()));
      }
      "--json" => {
        args.json = true;
      }
      "--raw" => {
        args.raw = true;
      }
      "--pretty" => {
        args.pretty = true;
      }
      "--edn" => {
        args.edn = true;
      }
      "--timeout" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("--timeout requires a value");
        }
        args.timeout_ms = Some(
          argv[i]
            .parse::<u64>()
            .map_err(|err| anyhow::anyhow!("invalid --timeout '{}': {}", argv[i], err))?,
        );
      }
      "--stack-limit" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("--stack-limit requires a value");
        }
        args.stack_limit = Some(
          argv[i]
            .parse::<usize>()
            .map_err(|err| anyhow::anyhow!("invalid --stack-limit '{}': {}", argv[i], err))?,
        );
      }
      flag if flag.starts_with('-') => {
        anyhow::bail!("unknown flag '{}'", flag);
      }
      path => {
        if args.file.is_some() {
          anyhow::bail!("unexpected argument '{}'", path);
        }
        args.file = Some(PathBuf::from(path));
      }
    }
    i += 1;
  }

  let format_flags = [args.json, args.raw, args.pretty, args.edn]
    .into_iter()
    .filter(|v| *v)
    .count();
  if format_flags > 1 {
    anyhow::bail!("only one of --json/--raw/--pretty/--edn can be specified");
  }

  Ok(args)
}

fn parse_legacy_serve_args(argv: &[String]) -> Result<LegacyServeArgs> {
  if !is_serve_invocation(argv) {
    anyhow::bail!("internal error: serve parser called without 'serve' subcommand");
  }

  let mut args = LegacyServeArgs {
    file: None,
    expr: None,
    host: "127.0.0.1".to_string(),
    port: 3000,
    max_body_bytes: 1_048_576,
    stack_limit: None,
  };

  let mut i = 2usize;
  while i < argv.len() {
    match argv[i].as_str() {
      "-e" | "--expr" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("{} requires a value", argv[i - 1]);
        }
        args.expr = Some(argv[i].clone());
      }
      "--source" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("--source requires a value");
        }
        args.file = Some(PathBuf::from(argv[i].clone()));
      }
      "--host" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("--host requires a value");
        }
        args.host = argv[i].clone();
      }
      "--port" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("--port requires a value");
        }
        args.port = argv[i]
          .parse::<u16>()
          .map_err(|err| anyhow::anyhow!("invalid --port '{}': {}", argv[i], err))?;
      }
      "--max-body-bytes" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("--max-body-bytes requires a value");
        }
        args.max_body_bytes = argv[i]
          .parse::<usize>()
          .map_err(|err| anyhow::anyhow!("invalid --max-body-bytes '{}': {}", argv[i], err))?;
      }
      "--stack-limit" => {
        i += 1;
        if i >= argv.len() {
          anyhow::bail!("--stack-limit requires a value");
        }
        args.stack_limit = Some(
          argv[i]
            .parse::<usize>()
            .map_err(|err| anyhow::anyhow!("invalid --stack-limit '{}': {}", argv[i], err))?,
        );
      }
      flag if flag.starts_with('-') => {
        anyhow::bail!("unknown flag '{}'", flag);
      }
      path => {
        if args.file.is_some() {
          anyhow::bail!("unexpected argument '{}'", path);
        }
        args.file = Some(PathBuf::from(path));
      }
    }
    i += 1;
  }

  Ok(args)
}

fn resolve_input(file: Option<PathBuf>, expr: Option<String>) -> Result<InputSource> {
  match (file, expr) {
    (Some(_), Some(_)) => anyhow::bail!("conflicting inputs: both file and --expr specified"),
    (Some(path), None) => {
      if path.as_os_str() == "-" {
        Ok(InputSource::Stdin)
      } else {
        Ok(InputSource::File(path))
      }
    }
    (None, Some(expr)) => Ok(InputSource::Expr(expr)),
    (None, None) => {
      if io::stdin().is_terminal() {
        anyhow::bail!("no input specified");
      }
      Ok(InputSource::Stdin)
    }
  }
}

fn read_eval_input(source: InputSource) -> Result<(String, bool)> {
  match source {
    InputSource::Expr(expr) => Ok((expr, false)),
    InputSource::Stdin => {
      let mut buf = String::new();
      io::stdin()
        .read_to_string(&mut buf)
        .map_err(|err| anyhow::anyhow!("stdin read error: {}", err))?;
      if buf.is_empty() {
        anyhow::bail!("empty input from stdin");
      }
      Ok((buf, false))
    }
    InputSource::File(path) => {
      let resolved = resolve_file_path(&path);
      ensure_source_file(&resolved)?;
      let source = std::fs::read_to_string(&resolved)
        .map_err(|err| anyhow::anyhow!("failed to read {}: {}", resolved.display(), err))?;
      Ok((source, false))
    }
  }
}

fn read_source_text(source: InputSource) -> Result<(String, Option<PathBuf>)> {
  match source {
    InputSource::Expr(expr) => Ok((expr, None)),
    InputSource::Stdin => {
      let mut buf = String::new();
      io::stdin()
        .read_to_string(&mut buf)
        .map_err(|err| anyhow::anyhow!("stdin read error: {}", err))?;
      if buf.is_empty() {
        anyhow::bail!("empty input from stdin");
      }
      Ok((buf, None))
    }
    InputSource::File(path) => {
      let resolved = resolve_file_path(&path);
      ensure_source_file(&resolved)?;
      let source = std::fs::read_to_string(&resolved)
        .map_err(|err| anyhow::anyhow!("failed to read {}: {}", resolved.display(), err))?;
      let import_base = resolved.parent().map(Path::to_path_buf);
      Ok((source, import_base))
    }
  }
}

fn ensure_source_file(path: &Path) -> Result<()> {
  let meta = std::fs::metadata(path)
    .map_err(|err| anyhow::anyhow!("failed to read {}: {}", path.display(), err))?;
  if meta.is_dir() {
    anyhow::bail!("source path is a directory: {}", path.display());
  }
  if !meta.is_file() {
    anyhow::bail!("source path is not a file: {}", path.display());
  }
  Ok(())
}

fn resolve_file_path(path: &Path) -> PathBuf {
  if path.extension().is_some() {
    return path.to_path_buf();
  }
  if path.exists() {
    return path.to_path_buf();
  }
  let with_ext = path.with_extension(DEFAULT_EXTENSION);
  if with_ext.exists() {
    return with_ext;
  }
  path.to_path_buf()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detect_eval_compat_for_positional_json() {
    let argv = vec![
      "pnix".to_string(),
      "examples/foo.px".to_string(),
      "--json".to_string(),
    ];
    assert!(should_use_eval_compat(&argv));
  }

  #[test]
  fn detect_eval_compat_for_expr() {
    let argv = vec![
      "pnix".to_string(),
      "--expr".to_string(),
      "1 + 1".to_string(),
    ];
    assert!(should_use_eval_compat(&argv));
  }

  #[test]
  fn modern_mode_does_not_use_eval_compat() {
    let argv = vec![
      "pnix".to_string(),
      "--mode".to_string(),
      "run".to_string(),
      "--dist".to_string(),
      "dist".to_string(),
    ];
    assert!(!should_use_eval_compat(&argv));
  }

  #[test]
  fn parse_serve_args_basics() {
    let argv = vec![
      "pnix".to_string(),
      "serve".to_string(),
      "app".to_string(),
      "--port".to_string(),
      "4000".to_string(),
    ];
    let parsed = parse_legacy_serve_args(&argv).unwrap();
    assert_eq!(parsed.file, Some(PathBuf::from("app")));
    assert_eq!(parsed.port, 4000);
    assert_eq!(parsed.host, "127.0.0.1".to_string());
  }
}
