// Experimental in-process evaluation path (net10+ only).
// Loads ClojureCLR substrate + guest AOT into the default ALC via Resolving,
// then invokes pnix-clr.evaluator without Process.Start.
//
// Design: pnix-clr/docs/IMPLEMENTATION.md §8
// Status: experimental opt-in — process-spawn remains the supported default.

#if NET10_0_OR_GREATER

using System;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text.Json;
using System.Threading;

namespace Pnix.Clr
{
    internal static class InProcessEval
    {
        static readonly object Gate = new();
        static bool s_resolvingHooked;
        static string? s_substrateDir;
        static string? s_artifactDir;
        static bool s_rtReady;
        static MethodInfo? s_varMethod;
        static Type? s_symbolType;
        static Type? s_keywordType;
        static Type? s_rtType;

        internal static EvalResult Source(string source, EvalOptions? options)
        {
            if (source is null)
                throw new ArgumentNullException(nameof(source));
            return EvalCore(source, filePath: null, options);
        }

        internal static EvalResult File(string path, EvalOptions? options)
        {
            if (string.IsNullOrWhiteSpace(path))
                throw new ArgumentException("path is required", nameof(path));
            var full = Path.GetFullPath(path);
            if (!System.IO.File.Exists(full))
                throw new FileNotFoundException("pnix source file not found", full);
            return EvalCore(source: null, filePath: full, options);
        }

        static EvalResult EvalCore(string? source, string? filePath, EvalOptions? options)
        {
            options ??= new EvalOptions();
            var paths = ResolvePaths(options);
            EnsureRuntime(paths);

            var prevCwd = Directory.GetCurrentDirectory();
            try
            {
                Directory.SetCurrentDirectory(paths.ArtifactDir);
                Environment.SetEnvironmentVariable("CLOJURE_LOAD_PATH", paths.ArtifactDir);
                Environment.SetEnvironmentVariable("PNIX_CLR_ROOT", paths.PnixClrRoot);
                if (!string.IsNullOrWhiteSpace(paths.ArtifactDir))
                {
                    Environment.SetEnvironmentVariable("PNIX_CLR_ARTIFACT", paths.ArtifactDir);
                    Environment.SetEnvironmentVariable("PNIX_CLR_RUNTIME_ARTIFACT", paths.ArtifactDir);
                }

                string json;
                lock (Gate)
                {
                    json = filePath is null
                        ? EvalSourceJson(source!, paths.PnixClrRoot)
                        : EvalFileJson(filePath, paths.PnixClrRoot);
                }

                return ParseCliJson(json, exitCode: 0);
            }
            catch (Exception ex) when (ex is not PnixEvalException and not NotSupportedException)
            {
                var inner = ex is TargetInvocationException tie ? tie.InnerException ?? ex : ex;
                throw new PnixEvalException(
                    "in-process pnix-clr evaluation failed: " + inner.Message, inner);
            }
            finally
            {
                try { Directory.SetCurrentDirectory(prevCwd); }
                catch { /* best effort */ }
            }
        }

        sealed record Paths(string SubstrateDir, string ArtifactDir, string PnixClrRoot);

        static Paths ResolvePaths(EvalOptions options)
        {
            var artifact = Eval.ResolveArtifactDir(options)
                ?? throw new NotSupportedException(
                    "in-process eval needs guest AOT: set PNIX_CLR_ARTIFACT or PNIX_CLR_LIBRARY "
                    + "(export-pnix-clr-library / build-pnix-clr-artifact).");

            artifact = Path.GetFullPath(artifact);
            if (!Directory.Exists(artifact)
                || !System.IO.File.Exists(Path.Combine(artifact, "manifest.json")))
            {
                throw new NotSupportedException(
                    "in-process eval: artifact dir missing or has no manifest.json: " + artifact);
            }

            var substrate = ResolveSubstrateDir(options);
            if (substrate is null
                || !System.IO.File.Exists(Path.Combine(substrate, "Clojure.dll")))
            {
                throw new NotSupportedException(
                    "in-process eval needs ClojureCLR substrate (Clojure.dll). "
                    + "Set PNIX_CLR_SUBSTRATE to the net10.0 publish dir, or PNIX_CLR_ROOT "
                    + "to the pnix-clr checkout (…/clojure-clr-…/publish).");
            }

            substrate = Path.GetFullPath(substrate);

            var root = options.Root
                ?? Environment.GetEnvironmentVariable("PNIX_CLR_ROOT")
                ?? InferPnixClrRoot(substrate, artifact)
                ?? Directory.GetCurrentDirectory();
            root = Path.GetFullPath(root);

            return new Paths(substrate, artifact, root);
        }

        static string? ResolveSubstrateDir(EvalOptions options)
        {
            if (!string.IsNullOrWhiteSpace(options.SubstrateDir))
                return options.SubstrateDir;

            if (options.ExtraEnv is not null
                && options.ExtraEnv.TryGetValue("PNIX_CLR_SUBSTRATE", out var fromOpt)
                && !string.IsNullOrWhiteSpace(fromOpt))
                return fromOpt;

            var env = Environment.GetEnvironmentVariable("PNIX_CLR_SUBSTRATE");
            if (!string.IsNullOrWhiteSpace(env))
                return env;

            var checkout = Environment.GetEnvironmentVariable("PNIX_CLR_ROOT");
            if (!string.IsNullOrWhiteSpace(checkout))
            {
                var candidate = FindPublishDir(checkout!);
                if (candidate is not null)
                    return candidate;
            }

            // Walk up from this assembly / cwd for a monorepo checkout layout.
            foreach (var start in new[]
                     {
                         AppContext.BaseDirectory,
                         Directory.GetCurrentDirectory(),
                     })
            {
                var dir = new DirectoryInfo(start);
                for (var i = 0; i < 8 && dir is not null; i++, dir = dir.Parent)
                {
                    var found = FindPublishDir(dir.FullName);
                    if (found is not null)
                        return found;
                }
            }

            return null;
        }

        static string? FindPublishDir(string root)
        {
            // Prefer the pinned bootstrap path used by bin/pnix-clr.
            var exact = Path.Combine(
                root,
                "clojure-clr-clojure-1.12.3-alpha8",
                "Clojure",
                "Clojure.Main",
                "bin",
                "Release",
                "net10.0",
                "publish");
            if (System.IO.File.Exists(Path.Combine(exact, "Clojure.dll")))
                return exact;

            try
            {
                foreach (var d in Directory.EnumerateDirectories(root, "clojure-clr-*"))
                {
                    var publish = Path.Combine(
                        d, "Clojure", "Clojure.Main", "bin", "Release", "net10.0", "publish");
                    if (System.IO.File.Exists(Path.Combine(publish, "Clojure.dll")))
                        return publish;
                }
            }
            catch (IOException)
            {
                // ignore
            }

            return null;
        }

        static string? InferPnixClrRoot(string substrate, string artifact)
        {
            // substrate: <checkout>/clojure-clr-…/…/publish
            var fromSub = Directory.GetParent(substrate);
            for (var i = 0; i < 6 && fromSub is not null; i++, fromSub = fromSub.Parent)
            {
                if (System.IO.File.Exists(Path.Combine(fromSub.FullName, "bin", "pnix-clr")))
                    return fromSub.FullName;
            }

            // artifact: <checkout>/pnix-clr/target/runtime-artifact
            var fromArt = Directory.GetParent(artifact);
            for (var i = 0; i < 4 && fromArt is not null; i++, fromArt = fromArt.Parent)
            {
                if (System.IO.File.Exists(Path.Combine(fromArt.FullName, "bin", "pnix-clr")))
                    return fromArt.FullName;
            }

            return null;
        }

        static void EnsureRuntime(Paths paths)
        {
            lock (Gate)
            {
                if (!s_resolvingHooked)
                {
                    s_substrateDir = paths.SubstrateDir;
                    s_artifactDir = paths.ArtifactDir;
                    AssemblyLoadContext.Default.Resolving += ResolveFromSubstrate;
                    s_resolvingHooked = true;
                }
                else
                {
                    // Paths can change across calls; keep the latest for Resolving.
                    s_substrateDir = paths.SubstrateDir;
                    s_artifactDir = paths.ArtifactDir;
                }

                if (s_rtReady)
                    return;

                Environment.SetEnvironmentVariable("CLOJURE_LOAD_PATH", paths.ArtifactDir);
                Environment.SetEnvironmentVariable("PNIX_CLR_ROOT", paths.PnixClrRoot);

                foreach (var dll in Directory.GetFiles(paths.SubstrateDir, "*.dll"))
                {
                    try
                    {
                        AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(dll));
                    }
                    catch (Exception)
                    {
                        // Already loaded or not a managed assembly.
                    }
                }

                var clojure = AppDomain.CurrentDomain.GetAssemblies()
                    .FirstOrDefault(a => a.GetName().Name == "Clojure")
                    ?? throw new PnixEvalException(
                        "in-process: failed to load Clojure.dll from " + paths.SubstrateDir);

                s_rtType = clojure.GetType("clojure.lang.RT")
                    ?? throw new PnixEvalException("in-process: clojure.lang.RT missing");
                RuntimeHelpers.RunClassConstructor(s_rtType.TypeHandle);

                s_varMethod = s_rtType.GetMethod("var", new[] { typeof(string), typeof(string) })
                    ?? throw new PnixEvalException("in-process: RT.var missing");
                s_symbolType = clojure.GetType("clojure.lang.Symbol")
                    ?? throw new PnixEvalException("in-process: Symbol missing");
                s_keywordType = clojure.GetType("clojure.lang.Keyword")
                    ?? throw new PnixEvalException("in-process: Keyword missing");

                // Warm product namespaces (loads guest AOT via CLOJURE_LOAD_PATH).
                Require("pnix-clr.evaluator");
                Require("pnix-clr.main");
                Require("pnix-clr.json");

                s_rtReady = true;
            }
        }

        static Assembly? ResolveFromSubstrate(AssemblyLoadContext _, AssemblyName name)
        {
            foreach (var dir in new[] { s_substrateDir, s_artifactDir })
            {
                if (string.IsNullOrEmpty(dir) || string.IsNullOrEmpty(name.Name))
                    continue;
                var p = Path.Combine(dir, name.Name + ".dll");
                if (System.IO.File.Exists(p))
                {
                    try
                    {
                        return AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(p));
                    }
                    catch (FileLoadException)
                    {
                        return AppDomain.CurrentDomain.GetAssemblies()
                            .FirstOrDefault(a => a.GetName().Name == name.Name);
                    }
                }
            }

            return null;
        }

        static void Require(string nsName)
        {
            var require = Var("clojure.core", "require");
            var sym = InternSymbol(nsName);
            InvokeIFn(require, sym);
        }

        static object Var(string ns, string name)
            => s_varMethod!.Invoke(null, new object[] { ns, name })!;

        static object InternSymbol(string name)
        {
            var intern = s_symbolType!.GetMethod("intern", new[] { typeof(string) })!;
            return intern.Invoke(null, new object[] { name })!;
        }

        static object InternKeyword(string name)
        {
            var intern = s_keywordType!.GetMethod("intern", new[] { typeof(string) })!;
            return intern.Invoke(null, new object[] { name })!;
        }

        static object MakeOpts(string root, string file)
        {
            var map4 = s_rtType!.GetMethod(
                "map",
                new[] { typeof(object), typeof(object), typeof(object), typeof(object) });
            if (map4 is not null)
            {
                return map4.Invoke(null, new object[]
                {
                    InternKeyword("root"), root,
                    InternKeyword("file"), file,
                })!;
            }

            foreach (var m in s_rtType.GetMethods())
            {
                if (m.Name != "map")
                    continue;
                var ps = m.GetParameters();
                if (ps.Length == 1 && ps[0].ParameterType.IsArray)
                {
                    return m.Invoke(null, new object[]
                    {
                        new object[]
                        {
                            InternKeyword("root"), root,
                            InternKeyword("file"), file,
                        }
                    })!;
                }
            }

            throw new PnixEvalException("in-process: RT.map unavailable");
        }

        static string EvalSourceJson(string source, string root)
        {
            var opts = MakeOpts(root, Path.Combine(root, "pnix-clr-inline.px"));
            var result = InvokeIFn(Var("pnix-clr.evaluator", "eval-source"), source, opts);
            return ProjectToJson(result);
        }

        static string EvalFileJson(string filePath, string hostRoot)
        {
            // Prefer eval-file when present (matches CLI file mode).
            try
            {
                Require("pnix-clr.evaluator");
                var evalFile = Var("pnix-clr.evaluator", "eval-file");
                // root for file: host root if file is under it, else file directory
                var fileDir = Path.GetDirectoryName(filePath) ?? hostRoot;
                var root = filePath.StartsWith(hostRoot, StringComparison.Ordinal)
                    ? hostRoot
                    : fileDir;
                var result = InvokeIFn(evalFile, root, filePath);
                return ProjectToJson(result);
            }
            catch (Exception)
            {
                var text = System.IO.File.ReadAllText(filePath);
                var opts = MakeOpts(hostRoot, filePath);
                var result = InvokeIFn(Var("pnix-clr.evaluator", "eval-source"), text, opts);
                return ProjectToJson(result);
            }
        }

        static string ProjectToJson(object result)
        {
            var proj = InvokeIFn(Var("pnix-clr.main", "projection"), result);
            var json = InvokeIFn(Var("pnix-clr.json", "write-json"), proj);
            return json?.ToString()
                ?? throw new PnixEvalException("in-process: write-json returned null");
        }

        static object InvokeIFn(object fn, params object?[] args)
        {
            foreach (var m in fn.GetType().GetMethods(BindingFlags.Instance | BindingFlags.Public))
            {
                if (m.Name != "invoke")
                    continue;
                if (m.GetParameters().Length == args.Length)
                {
                    try
                    {
                        return m.Invoke(fn, args)!;
                    }
                    catch (TargetInvocationException tie)
                    {
                        throw tie.InnerException ?? tie;
                    }
                }
            }

            throw new MissingMethodException(fn.GetType().FullName, "invoke/" + args.Length);
        }

        static EvalResult ParseCliJson(string jsonLine, int exitCode)
        {
            // Reuse the same field mapping as process path.
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

                var outcome = root.TryGetProperty("outcome_kind", out var o)
                    ? o.GetString() ?? ""
                    : "";
                // Match CLI: failed outcomes exit 1.
                var code = string.Equals(outcome, "failed", StringComparison.Ordinal)
                    ? 1
                    : exitCode;

                return new EvalResult
                {
                    Schema = root.TryGetProperty("schema", out var s) ? s.GetString() ?? "" : "",
                    Host = root.TryGetProperty("host", out var h) ? h.GetString() ?? "" : "",
                    OutcomeKind = outcome,
                    Value = value,
                    Error = error,
                    RawJson = jsonLine,
                    Stderr = "",
                    ExitCode = code,
                };
            }
            catch (JsonException ex)
            {
                throw new PnixEvalException(
                    "in-process returned non-JSON projection: " + jsonLine, ex);
            }
        }
    }
}

#endif
