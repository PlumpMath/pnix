using clojure.lang;

namespace Pnix.Clr.Bootstrap
{
    /// <summary>
    /// Minimal `clojure.main` entry point over the pinned upstream ClojureCLR
    /// runtime. It adds no language behaviour: it initializes the runtime and
    /// hands every argument to upstream `clojure.main/main` unchanged.
    /// </summary>
    public static class Program
    {
        public static void Main(string[] args)
        {
            RT.Init();
            RT.var("clojure.core", "require")
              .invoke(Symbol.intern("clojure.main"));
            RT.var("clojure.main", "main").applyTo(RT.seq(args));
        }
    }
}
