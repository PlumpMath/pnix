# plain .NET — no guest language boundary by default

- `Microsoft.CodeAnalysis.CSharp.Scripting` / `Eval` can reach the process,
  filesystem, and network unless you build a restricted host yourself.
- There is no stock “pure Nix-like guest” in the BCL.
- Conclusion: use an explicit guest evaluator (`pnix-clr`) for untrusted
  expression languages; do not treat host scripting as a sandbox.
