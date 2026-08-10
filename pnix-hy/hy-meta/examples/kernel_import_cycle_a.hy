(setv A_STARTED True)

(import kernel-import-cycle-b [B_SEES_A_STARTED B_SEES_A_VALUE_BEFORE B_VALUE])

(setv A_VALUE 10)
(setv A_SEES_B_VALUE B_VALUE)
