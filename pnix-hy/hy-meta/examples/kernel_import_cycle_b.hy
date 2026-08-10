(import sys)

(setv A_MODULE (get sys.modules "kernel_import_cycle_a"))
(setv B_SEES_A_STARTED (getattr A_MODULE "A_STARTED" False))
(setv B_SEES_A_VALUE_BEFORE (getattr A_MODULE "A_VALUE" "missing"))
(setv B_VALUE 32)
