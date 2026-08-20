use anyhow::{Context, Result};
use pnix_query_runtime::{response_document, KernelPaths, PnixReplKernel};
use std::env;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::PathBuf;

fn main() -> Result<()> {
  let mut kernel = PnixReplKernel::new(KernelPaths::default());
  let mut args = env::args().skip(1);
  match args.next().as_deref() {
    Some("--file") => {
      let path = args
        .next()
        .map(PathBuf::from)
        .context("missing path after --file")?;
      run_file(&mut kernel, &path)
    }
    Some("--stdin") => run_stdin_once(&mut kernel),
    Some(flag) => Err(anyhow::anyhow!(
      "unsupported flag '{flag}' — use --file <path> or pipe a .px query document to stdin"
    )),
    None => {
      if io::stdin().is_terminal() {
        run_repl(&mut kernel)
      } else {
        run_stdin_once(&mut kernel)
      }
    }
  }
}

fn run_file(kernel: &mut PnixReplKernel, path: &PathBuf) -> Result<()> {
  let response = kernel
    .evaluate_px_file(path)
    .with_context(|| format!("evaluate {}", path.display()))?;
  print_response(&response);
  Ok(())
}

fn run_stdin_once(kernel: &mut PnixReplKernel) -> Result<()> {
  let mut buffer = String::new();
  io::stdin()
    .read_to_string(&mut buffer)
    .context("read .px query document from stdin")?;
  if buffer.trim().is_empty() {
    return Err(anyhow::anyhow!(
      "stdin was empty — pnix-query-repl accepts only non-empty .px ontology-query documents"
    ));
  }
  let response = kernel
    .evaluate_px_source(&buffer)
    .context("evaluate .px query document from stdin")?;
  print_response(&response);
  Ok(())
}

fn run_repl(kernel: &mut PnixReplKernel) -> Result<()> {
  let stdin = io::stdin();
  let mut handle = stdin.lock();
  let mut stdout = io::stdout();
  let mut chunk = String::new();

  eprintln!("pnix-query-repl");
  eprintln!("enter a .px ontology-query document, then submit a blank line");
  eprintln!(":quit to exit");

  loop {
    if chunk.is_empty() {
      write!(stdout, "pnix> ")?;
    } else {
      write!(stdout, "....> ")?;
    }
    stdout.flush()?;

    let mut line = String::new();
    let read = handle.read_line(&mut line).context("read repl line")?;
    if read == 0 {
      if chunk.trim().is_empty() {
        break;
      }
      let response = kernel
        .evaluate_px_source(&chunk)
        .context("evaluate pending .px query document")?;
      print_response(&response);
      break;
    }

    let trimmed = line.trim_end();
    if chunk.trim().is_empty() && trimmed == ":quit" {
      break;
    }
    if trimmed.is_empty() {
      if chunk.trim().is_empty() {
        continue;
      }
      match kernel.evaluate_px_source(&chunk) {
        Ok(response) => print_response(&response),
        Err(err) => eprintln!("[pnix-query-repl] {err:#}"),
      }
      chunk.clear();
      continue;
    }

    chunk.push_str(&line);
  }

  Ok(())
}

fn print_response(response: &pnix_query_runtime::KernelResponse) {
  println!(
    "{}",
    response_document::response_document_native_text(response)
  );
  if let Some(hint) = response.follow_up_hint.as_deref() {
    eprintln!("[follow-up] {hint}");
  }
}
