using System;
using Pnix.Clr;

static class Program
{
    static int Main(string[] args)
    {
        try
        {
            // Experimental net10 in-process path (see pnix-clr/docs/IMPLEMENTATION.md §8).
            var inProcess = args.Length >= 1 && args[0] == "--inprocess";
            if (inProcess)
                args = args[1..];

            if (args.Length >= 1 && args[0] == "--file")
            {
                if (args.Length < 2)
                {
                    Console.Error.WriteLine("usage: HelloPnix [--inprocess] --file FILE.px");
                    return 2;
                }
                var fr = inProcess
                    ? Eval.FileInProcess(args[1]).EnsureDone()
                    : Eval.File(args[1]).EnsureDone();
                Console.WriteLine(fr.Value.HasValue ? fr.Value.Value.GetRawText() : fr.RawJson);
                return 0;
            }

            if (args.Length >= 1 && args[0] == "--call")
            {
                if (inProcess || args.Length < 4)
                {
                    Console.Error.WriteLine(
                        "usage: HelloPnix --call FILE.px ENTRY ARGS_JSON (process mode)");
                    return 2;
                }
                var called = Eval.CallFile(args[1], args[2], args[3]).EnsureDone();
                Console.WriteLine(
                    called.Value.HasValue ? called.Value.Value.GetRawText() : called.RawJson);
                return 0;
            }

            var source = args.Length > 0 ? string.Join(" ", args) : "1 + 2";
            var r = inProcess
                ? Eval.SourceInProcess(source).EnsureDone()
                : Eval.Source(source).EnsureDone();
            Console.WriteLine(r.Value.HasValue ? r.Value.Value.GetRawText() : r.RawJson);
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine(ex.Message);
            return 1;
        }
    }
}
