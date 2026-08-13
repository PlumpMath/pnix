// Parity probe: process-spawn Eval vs experimental in-process (net10).
// Exit 0 only when outcome_kind + value/error JSON agree on the fixed corpus.
using Pnix.Clr;

var corpus = new (string id, string source)[]
{
    // arithmetic / bool / control
    ("add", "1 + 2"),
    ("mul-neg", "(-7) * (-6)"),
    ("bool", "true && !false"),
    ("if", "if true then 40 + 2 else 0"),
    ("if-else", "if false then 1 else 2 + 3"),
    // attrs / select
    ("attrset", "{ a = 1; b = 2; }.a + { a = 1; b = 2; }.b"),
    ("select-path", "builtins.getAttrFromPath [ \"foo\" \"bar\" ] { foo.bar = 42; }"),
    // builtins
    ("typeof", "builtins.typeOf 1"),
    ("length", "builtins.length [ 1 2 3 4 ]"),
    ("hasAttr", "builtins.hasAttr \"x\" { x = 1; }"),
    // lists
    ("list-sum-shape", "builtins.foldl' (a: b: a + b) 0 [ 1 2 3 4 ]"),
    // failures (structured parity)
    ("fail-div0", "1 / 0"),
    ("fail-unbound", "missingVar"),
    ("fail-missing-attr", "{ a = 1; }.b"),
};

var opts = new EvalOptions
{
    Root = Environment.GetEnvironmentVariable("PNIX_CLR_ROOT"),
    PnixClrPath = Environment.GetEnvironmentVariable("PNIX_CLR"),
    SubstrateDir = Environment.GetEnvironmentVariable("PNIX_CLR_SUBSTRATE"),
};

var fail = 0;
var pass = 0;

foreach (var (id, source) in corpus)
{
    if (!CheckParity(id, source, opts, file: null, ref fail, ref pass))
        continue;
}

// File mode: same hello.px as HelloPnix example.
var hello = Path.GetFullPath(Path.Combine(
    AppContext.BaseDirectory, "..", "..", "..", "..", "hello.px"));
if (!File.Exists(hello))
{
    // bin/pnix-clr-inprocess-eval-gate runs from repo; also try env root.
    var root = Environment.GetEnvironmentVariable("PNIX_CLR_ROOT");
    if (!string.IsNullOrEmpty(root))
        hello = Path.Combine(root, "csharp", "examples", "hello.px");
}

if (File.Exists(hello))
{
    Console.WriteLine($"== file-hello: {hello}");
    try
    {
        var proc = Eval.File(hello, opts);
        var inp = Eval.FileInProcess(hello, opts);
        var ok = SameOutcome(proc, inp);
        Console.WriteLine($"  process: {proc.OutcomeKind} value={ValueText(proc)}");
        Console.WriteLine($"  inproc:  {inp.OutcomeKind} value={ValueText(inp)}");
        Console.WriteLine(ok ? "  OK" : "  FAIL parity");
        if (ok) pass++; else fail++;
    }
    catch (Exception ex)
    {
        Console.WriteLine("  FAIL threw: " + ex.Message);
        fail++;
    }
}
else
{
    Console.WriteLine("== file-hello: SKIP (hello.px not found)");
}

// Negatives: missing substrate / missing artifact fail closed.
try
{
    var missing = Path.Combine(Path.GetTempPath(), "pnix-missing-substrate-" + Guid.NewGuid().ToString("n"));
    Directory.CreateDirectory(missing);
    Eval.SourceInProcess("1", new EvalOptions { SubstrateDir = missing });
    Console.WriteLine("== negative substrate: FAIL (should have thrown)");
    fail++;
}
catch (Exception ex)
{
    Console.WriteLine("== negative substrate: OK closed (" + ex.GetType().Name + ")");
    pass++;
}

try
{
    var badArt = Path.Combine(Path.GetTempPath(), "pnix-missing-artifact-" + Guid.NewGuid().ToString("n"));
    Directory.CreateDirectory(badArt);
    // Clear env-based artifact resolution for this call via ExtraEnv empty dir.
    var bad = new EvalOptions
    {
        SubstrateDir = opts.SubstrateDir,
        ExtraEnv = new Dictionary<string, string>
        {
            ["PNIX_CLR_ARTIFACT"] = badArt,
            ["PNIX_CLR_RUNTIME_ARTIFACT"] = badArt,
            ["PNIX_CLR_LIBRARY"] = "",
        },
    };
    // ResolveArtifactDir checks ExtraEnv first for PNIX_CLR_ARTIFACT.
    Eval.SourceInProcess("1", bad);
    Console.WriteLine("== negative artifact: FAIL (should have thrown)");
    fail++;
}
catch (Exception ex)
{
    Console.WriteLine("== negative artifact: OK closed (" + ex.GetType().Name + ")");
    pass++;
}

Console.WriteLine($"== summary: pass={pass} fail={fail}");
return fail == 0 ? 0 : 1;

static bool CheckParity(
    string id, string source, EvalOptions opts, string? file, ref int fail, ref int pass)
{
    Console.WriteLine($"== {id}: {source}");
    EvalResult proc;
    try
    {
        proc = file is null ? Eval.Source(source, opts) : Eval.File(file, opts);
    }
    catch (Exception ex)
    {
        Console.WriteLine("  PROCESS threw: " + ex.Message);
        fail++;
        return false;
    }

    EvalResult inp;
    try
    {
        inp = file is null ? Eval.SourceInProcess(source, opts) : Eval.FileInProcess(file, opts);
    }
    catch (Exception ex)
    {
        Console.WriteLine("  INPROC threw: " + ex.Message);
        fail++;
        return false;
    }

    var ok = SameOutcome(proc, inp);
    Console.WriteLine($"  process: {proc.OutcomeKind} value={ValueText(proc)} exit={proc.ExitCode}");
    Console.WriteLine($"  inproc:  {inp.OutcomeKind} value={ValueText(inp)} exit={inp.ExitCode}");
    Console.WriteLine(ok ? "  OK" : "  FAIL parity");
    if (ok) pass++; else fail++;
    return ok;
}

static bool SameOutcome(EvalResult a, EvalResult b)
{
    if (!string.Equals(a.OutcomeKind, b.OutcomeKind, StringComparison.Ordinal))
        return false;
    if (!string.Equals(a.Schema, b.Schema, StringComparison.Ordinal))
        return false;
    return ValueText(a) == ValueText(b);
}

static string ValueText(EvalResult r)
{
    if (r.Value.HasValue)
        return r.Value.Value.GetRawText();
    if (r.Error.HasValue)
        return "error:" + r.Error.Value.GetRawText();
    return "";
}
