// Host-language (C#) import surface for the pnix-clr product.
//
// Evaluates pnix (`.px`) by spawning the `pnix-clr` CLI and parsing its
// JSON result contract (schema pnix-clr.cli-result.v1). This is the supported
// way for ordinary C# projects to load/eval host-bound pnix programs without
// embedding ClojureCLR in-process.
//
// Guest AOT DLLs (pnix_clr.*.clj.dll) remain available for ClojureCLR hosts
// via PNIX_CLR_ARTIFACT / MSBuild props — they are not loaded by this class.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace Pnix.Clr
{
    /// <summary>Structured result from a pnix-clr evaluation.</summary>
    public sealed class EvalResult
    {
        public string Schema { get; init; } = "";
        public string Host { get; init; } = "";
        public string OutcomeKind { get; init; } = "";
        public JsonElement? Value { get; init; }
        public JsonElement? Error { get; init; }
        public string RawJson { get; init; } = "";
        public int ExitCode { get; init; }
        public string Stderr { get; init; } = "";

        public bool IsDone =>
            string.Equals(OutcomeKind, "done", StringComparison.Ordinal);

        public bool IsFailed =>
            string.Equals(OutcomeKind, "failed", StringComparison.Ordinal)
            || ExitCode != 0;

        /// <summary>Throw if the evaluation did not complete successfully.</summary>
        public EvalResult EnsureDone()
        {
            if (IsDone && ExitCode == 0)
                return this;
            var detail = Error.HasValue
                ? Error.Value.GetRawText()
                : (string.IsNullOrWhiteSpace(Stderr) ? RawJson : Stderr);
            throw new PnixEvalException(
                $"pnix-clr evaluation failed (outcome={OutcomeKind}, exit={ExitCode}): {detail}");
        }
    }

    /// <summary>Thrown when a pnix-clr evaluation fails or the process misbehaves.</summary>
    public sealed class PnixEvalException : Exception
    {
        public PnixEvalException(string message) : base(message) { }
        public PnixEvalException(string message, Exception inner) : base(message, inner) { }
    }

    /// <summary>
    /// Options for locating and invoking the pnix-clr host CLI.
    /// Resolution order for the executable:
    ///   1. <see cref="PnixClrPath"/> if set
    ///   2. env PNIX_CLR
    ///   3. env PNIX_CLR_ROOT + "/bin/pnix-clr"
    ///   4. "pnix-clr" on PATH
    /// </summary>
    public sealed class EvalOptions
    {
        /// <summary>Absolute path to the pnix-clr executable.</summary>
        public string? PnixClrPath { get; init; }

        /// <summary>
        /// Root directory for pnix import resolution (passed as trailing ROOT
        /// only when using file mode and the CLI supports it). Currently used
        /// to set the process working directory for relative paths.
        /// </summary>
        public string? Root { get; init; }

        /// <summary>Process working directory. Defaults to <see cref="Root"/> or cwd.</summary>
        public string? WorkingDirectory { get; init; }

        /// <summary>Kill the process after this many milliseconds (default 120s).</summary>
        public int TimeoutMs { get; init; } = 120_000;

        /// <summary>Extra environment variables for the child process.</summary>
        public IReadOnlyDictionary<string, string>? ExtraEnv { get; init; }
    }

    /// <summary>
    /// Evaluate pnix source or a <c>.px</c> file from C#.
    /// </summary>
    public static class Eval
    {
        /// <summary>Evaluate an inline pnix expression.</summary>
        public static EvalResult Source(string source, EvalOptions? options = null)
        {
            if (source is null)
                throw new ArgumentNullException(nameof(source));
            return Run(new[] { "-e", source }, options);
        }

        /// <summary>Evaluate a <c>.px</c> file (host-bound import of a pnix program).</summary>
        public static EvalResult File(string path, EvalOptions? options = null)
        {
            if (string.IsNullOrWhiteSpace(path))
                throw new ArgumentException("path is required", nameof(path));
            var full = System.IO.Path.GetFullPath(path);
            if (!System.IO.File.Exists(full))
                throw new FileNotFoundException("pnix source file not found", full);
            return Run(new[] { full }, options);
        }

        /// <summary>Async evaluate an inline expression.</summary>
        public static Task<EvalResult> SourceAsync(
            string source, EvalOptions? options = null, CancellationToken ct = default)
        {
            if (source is null)
                throw new ArgumentNullException(nameof(source));
            return RunAsync(new[] { "-e", source }, options, ct);
        }

        /// <summary>Async evaluate a <c>.px</c> file.</summary>
        public static Task<EvalResult> FileAsync(
            string path, EvalOptions? options = null, CancellationToken ct = default)
        {
            if (string.IsNullOrWhiteSpace(path))
                throw new ArgumentException("path is required", nameof(path));
            var full = System.IO.Path.GetFullPath(path);
            if (!System.IO.File.Exists(full))
                throw new FileNotFoundException("pnix source file not found", full);
            return RunAsync(new[] { full }, options, ct);
        }

        /// <summary>
        /// Resolve the directory of guest AOT DLLs (runtime-artifact).
        /// Order: options ExtraEnv, PNIX_CLR_ARTIFACT, PNIX_CLR_RUNTIME_ARTIFACT,
        /// PNIX_CLR_LIBRARY/lib/net10.0/runtime-artifact, null if unset.
        /// </summary>
        public static string? ResolveArtifactDir(EvalOptions? options = null)
        {
            if (options?.ExtraEnv is not null
                && options.ExtraEnv.TryGetValue("PNIX_CLR_ARTIFACT", out var fromOpt)
                && !string.IsNullOrWhiteSpace(fromOpt))
                return fromOpt;

            var a = Environment.GetEnvironmentVariable("PNIX_CLR_ARTIFACT");
            if (!string.IsNullOrWhiteSpace(a))
                return a;

            var b = Environment.GetEnvironmentVariable("PNIX_CLR_RUNTIME_ARTIFACT");
            if (!string.IsNullOrWhiteSpace(b))
                return b;

            var lib = Environment.GetEnvironmentVariable("PNIX_CLR_LIBRARY");
            if (!string.IsNullOrWhiteSpace(lib))
            {
                var nested = System.IO.Path.Combine(lib, "lib", "net10.0", "runtime-artifact");
                if (Directory.Exists(nested))
                    return nested;
            }

            return null;
        }

        static string ResolveExecutable(EvalOptions? options)
        {
            if (!string.IsNullOrWhiteSpace(options?.PnixClrPath))
                return options!.PnixClrPath!;

            var fromEnv = Environment.GetEnvironmentVariable("PNIX_CLR");
            if (!string.IsNullOrWhiteSpace(fromEnv))
                return fromEnv!;

            var root = Environment.GetEnvironmentVariable("PNIX_CLR_ROOT");
            if (!string.IsNullOrWhiteSpace(root))
            {
                var candidate = System.IO.Path.Combine(root!, "bin", "pnix-clr");
                if (System.IO.File.Exists(candidate))
                    return candidate;
            }

            return "pnix-clr";
        }

        static EvalResult Run(string[] args, EvalOptions? options)
            => RunAsync(args, options, CancellationToken.None).GetAwaiter().GetResult();

        static async Task<EvalResult> RunAsync(
            string[] args, EvalOptions? options, CancellationToken ct)
        {
            options ??= new EvalOptions();
            var exe = ResolveExecutable(options);
            var workDir = options.WorkingDirectory
                ?? options.Root
                ?? Directory.GetCurrentDirectory();

            var psi = new ProcessStartInfo
            {
                FileName = exe,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true,
                WorkingDirectory = workDir,
            };
            foreach (var a in args)
                psi.ArgumentList.Add(a);

            // Propagate artifact env so a bare pnix-clr on PATH still finds DLLs.
            var artifact = ResolveArtifactDir(options);
            if (!string.IsNullOrWhiteSpace(artifact))
            {
                psi.Environment["PNIX_CLR_ARTIFACT"] = artifact;
                psi.Environment["PNIX_CLR_RUNTIME_ARTIFACT"] = artifact;
            }
            if (options.ExtraEnv is not null)
            {
                foreach (var kv in options.ExtraEnv)
                    psi.Environment[kv.Key] = kv.Value;
            }

            using var proc = new Process { StartInfo = psi };
            var stdout = new StringBuilder();
            var stderr = new StringBuilder();
            proc.OutputDataReceived += (_, e) =>
            {
                if (e.Data is not null)
                    stdout.AppendLine(e.Data);
            };
            proc.ErrorDataReceived += (_, e) =>
            {
                if (e.Data is not null)
                    stderr.AppendLine(e.Data);
            };

            try
            {
                if (!proc.Start())
                    throw new PnixEvalException($"failed to start pnix-clr at '{exe}'");
            }
            catch (Exception ex) when (ex is not PnixEvalException)
            {
                throw new PnixEvalException(
                    $"failed to start pnix-clr at '{exe}': {ex.Message}. "
                    + "Set PNIX_CLR or PNIX_CLR_ROOT, or install pnix-clr on PATH.",
                    ex);
            }

            proc.BeginOutputReadLine();
            proc.BeginErrorReadLine();

            using var reg = ct.Register(() =>
            {
                try { if (!proc.HasExited) proc.Kill(entireProcessTree: true); }
                catch { /* best effort */ }
            });

            var finished = await Task.Run(() => proc.WaitForExit(options.TimeoutMs), ct)
                .ConfigureAwait(false);
            if (!finished)
            {
                try { proc.Kill(entireProcessTree: true); }
                catch { /* best effort */ }
                throw new PnixEvalException(
                    $"pnix-clr timed out after {options.TimeoutMs}ms");
            }

            // Drain async readers
            proc.WaitForExit();

            var raw = stdout.ToString().Trim();
            var err = stderr.ToString().Trim();
            return ParseResult(raw, err, proc.ExitCode);
        }

        static EvalResult ParseResult(string raw, string stderr, int exitCode)
        {
            // CLI prints one JSON object on stdout (may be multi-line pretty, but
            // current pnix-clr emits a single line). Take the last non-empty line
            // that looks like JSON if mixed with noise.
            var jsonLine = PickJsonLine(raw);
            if (string.IsNullOrWhiteSpace(jsonLine))
            {
                return new EvalResult
                {
                    Schema = "",
                    Host = "pnix-clr",
                    OutcomeKind = exitCode == 0 ? "unknown" : "failed",
                    RawJson = raw,
                    Stderr = stderr,
                    ExitCode = exitCode == 0 ? 2 : exitCode,
                };
            }

            try
            {
                using var doc = JsonDocument.Parse(jsonLine);
                var root = doc.RootElement;
                JsonElement? value = null;
                JsonElement? error = null;
                if (root.TryGetProperty("value", out var v))
                    value = v.Clone();
                if (root.TryGetProperty("error", out var e))
                    error = e.Clone();

                return new EvalResult
                {
                    Schema = root.TryGetProperty("schema", out var s) ? s.GetString() ?? "" : "",
                    Host = root.TryGetProperty("host", out var h) ? h.GetString() ?? "" : "",
                    OutcomeKind = root.TryGetProperty("outcome_kind", out var o)
                        ? o.GetString() ?? ""
                        : "",
                    Value = value,
                    Error = error,
                    RawJson = jsonLine,
                    Stderr = stderr,
                    ExitCode = exitCode,
                };
            }
            catch (JsonException ex)
            {
                throw new PnixEvalException(
                    $"pnix-clr returned non-JSON stdout: {jsonLine}", ex);
            }
        }

        static string PickJsonLine(string raw)
        {
            if (string.IsNullOrWhiteSpace(raw))
                return "";
            // Prefer last line that starts with '{'
            var lines = raw.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
            for (var i = lines.Length - 1; i >= 0; i--)
            {
                var t = lines[i].Trim();
                if (t.StartsWith('{'))
                    return t;
            }
            var whole = raw.Trim();
            return whole.StartsWith('{') ? whole : "";
        }
    }
}
