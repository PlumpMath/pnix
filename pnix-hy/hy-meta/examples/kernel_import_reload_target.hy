(import builtins)

(setv builtins._hy_meta_kernel_reload_count
      (+ (getattr builtins "_hy_meta_kernel_reload_count" 0) 1))

(setv RELOAD_COUNT builtins._hy_meta_kernel_reload_count)
(setv VALUE (+ RELOAD_COUNT 40))
