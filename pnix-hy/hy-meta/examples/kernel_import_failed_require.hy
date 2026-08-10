(defmacro require-failure-local []
  '999)

(require kernel_import_require_provider [ok-macro missing-macro])

(setv REQUIRE_FAILURE_VALUE (ok-macro))
