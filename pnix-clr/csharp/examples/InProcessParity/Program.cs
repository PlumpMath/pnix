// Parity probe: process-spawn Eval.Source vs experimental SourceInProcess.
// Exit 0 only when outcome_kind + value/error JSON agree on the fixed corpus.
using Pnix.Clr;

var corpus = new (string id, string source)[]
{
    ("add", "1 + 2"),
    ("bool", "true && !false"),
    ("if", "if true then 40 + 2 else 0"),
    ("fail-div0", "1 / 0"),
};

var opts = new EvalOptions
{
    Root = Environment.GetEnvironmentVariable("PNIX_CLR_ROOT"),
    PnixClrPath = Environment.GetEnvironmentVariable("PNIX_CLR"),
    SubstrateDir = Environment.GetEnvironmentVariable("PNIX_CLR_SUBSTRATE"),
};

var fail = 0;
foreach (var (id, source) in corpus)
{
    Console.WriteLine($"== {id}: {source}");
    EvalResult proc;
    try
    {
        proc = Eval.Source(source, opts);
    }
    catch (Exception ex)
    {
        Console.WriteLine("  PROCESS threw: " + ex.Message);
        fail++;
        continue;
    }

    EvalResult inp;
    try
    {
        inp = Eval.SourceInProcess(source, opts);
    }
    catch (Exception ex)
    {
        Console.WriteLine("  INPROC threw: " + ex.Message);
        fail++;
        continue;
    }

    var ok = SameOutcome(proc, inp);
    Console.WriteLine($"  process: {proc.OutcomeKind} value={ValueText(proc)} exit={proc.ExitCode}");
    Console.WriteLine($"  inproc:  {inp.OutcomeKind} value={ValueText(inp)} exit={inp.ExitCode}");
    Console.WriteLine(ok ? "  OK" : "  FAIL parity");
    if (!ok)
        fail++;
}

// Negative: missing substrate should fail closed (not hang).
try
{
    var missing = Path.Combine(Path.GetTempPath(), "pnix-missing-substrate-" + Guid.NewGuid().ToString("n"));
    Directory.CreateDirectory(missing);
    var bad = new EvalOptions { SubstrateDir = missing };
    Eval.SourceInProcess("1", bad);
    Console.WriteLine("== negative substrate: FAIL (should have thrown)");
    fail++;
}
catch (Exception ex)
{
    Console.WriteLine("== negative substrate: OK closed (" + ex.GetType().Name + ")");
}

Console.WriteLine(fail == 0 ? "== summary: PASS" : $"== summary: FAIL count={fail}");
return fail == 0 ? 0 : 1;

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
