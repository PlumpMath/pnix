(import kernel-import-pkg [child])
(import . [sibling])
(import .sibling [SIBLING_VALUE])

(setv CHILD_VALUE (. child CHILD_VALUE))
(setv RELATIVE_PACKAGE_VALUE (+ (. sibling SIBLING_VALUE) SIBLING_VALUE))
(setv PACKAGE_VALUE (+ CHILD_VALUE 1))

PACKAGE_VALUE
