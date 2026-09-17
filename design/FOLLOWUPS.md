
### `no-file-mode-warning-on-windows` — §8.2g's permission warning is POSIX-only

- **Surfaced:** 2026-09-17, adding the cross-platform release matrix.
  `build mt (windows-x86_64)` failed to compile:
  `error[E0425]: cannot find function file_mode_warning in module validate`.
- **Cause:** `validate::file_mode_warning` is `#[cfg(unix)]` (it inspects
  `0o077` mode bits) but `main.rs:414` called it unconditionally. A
  `#[cfg(not(unix))]` arm returning `None` now exists, which is **exactly what
  the unix arm already returns** when it cannot stat the source — a pipe, a
  terminal, a failed `metadata`. This adds no new silent path; it reuses the
  existing unknown one.
- **The arm now WARNS rather than returning `None`.** The first version returned
  `None`, matching what unix returns when it cannot stat the source. That was
  wrong in a way worth naming: on unix `None` means *"this input has no mode to
  read"*; on Windows it would have meant *"mt did not look"* — rendered
  identically to *"mt looked and it was fine"*. Silence indistinguishable from an
  all-clear is the failure this module exists to avoid. It now says the check did
  not run, and that nothing is known either way.
- **Still open:** the real check is an NTFS ACL inspection, not a mode mask —
  different work than a cfg arm.
- **Not a downgrade of anything else:** every other refusal and warning `mt`
  makes is platform-independent and unaffected.
- **Status:** open.
- **Tier:** platform-support.
