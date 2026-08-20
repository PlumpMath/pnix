use anyhow::{Context, Result};
use pnix_query_runtime::px_eval_json;
use std::env;
use std::io::Read;
use std::path::PathBuf;

fn main() -> Result<()> {
  let mut args = env::args().skip(1);
  match args.next().as_deref() {
    Some("--file") => {
      let path = args
        .next()
        .map(PathBuf::from)
        .context("missing path after --file")?;
      let json = px_eval_json::eval_px_file_to_json(&path)
        .with_context(|| format!("evaluate {}", path.display()))?;
      println!("{}", json);
      Ok(())
    }
    Some("--stdin") => {
      let mut source = String::new();
      std::io::stdin()
        .read_to_string(&mut source)
        .context("read .px source from stdin")?;
      let json =
        px_eval_json::eval_px_source_to_json(&source).context("evaluate .px source from stdin")?;
      println!("{}", json);
      Ok(())
    }
    Some(flag) => Err(anyhow::anyhow!(
      "unsupported flag '{flag}' — use --file <path> or --stdin"
    )),
    None => Err(anyhow::anyhow!(
      "missing arguments — use --file <path> or --stdin"
    )),
  }
}
