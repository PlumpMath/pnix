using System;
using Pnix.Clr;

static class Program
{
    static int Main(string[] args)
    {
        try
        {
            if (args.Length >= 1 && args[0] == "--file")
            {
                if (args.Length < 2)
                {
                    Console.Error.WriteLine("usage: HelloPnix --file FILE.px");
                    return 2;
                }
                var fr = Eval.File(args[1]).EnsureDone();
                Console.WriteLine(fr.Value.HasValue ? fr.Value.Value.GetRawText() : fr.RawJson);
                return 0;
            }

            var source = args.Length > 0 ? string.Join(" ", args) : "1 + 2";
            var r = Eval.Source(source).EnsureDone();
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
