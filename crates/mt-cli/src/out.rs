//! The one stdout writer (FIX_bh1119_mt_output_writes S1, T6).
//!
//! Every byte `mt` writes to stdout goes through [`stdout_write`], which writes
//! and then flushes and checks both results. std's `print!` family panics on a
//! failed write and a bare `write!` to `stdout()` that ignores its result loses
//! the output at exit 0 (`> /dev/full`); here the first failure stops every
//! later stdout write of the run (S1a).
//!
//! A reader that closed the pipe (`ErrorKind::BrokenPipe`) only stops the
//! output: the exit code is the verb's own (S1b). Any other error is kept, and
//! [`exit_code`] prints `mt: cannot write the output: <error>` once, as the last
//! stderr line, and adds IO (32) to the verb's own bits (exit-code spec R8). A
//! failed stderr write is ignored and never changes the exit code (S1e).
//!
//! [`note`] records a bit a verb has reported without stopping (REPAIRED when
//! it used a corrected chunk, IO when a named helper failed); [`exit_code`]
//! sums it into the run's status.
//!
//! `encode --out` failures reach the same line through [`failed_text`] (W3).

use std::io::Write;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::exit::Exit;

static NOTED: AtomicU8 = AtomicU8::new(0);
static CLOSED: AtomicBool = AtomicBool::new(false);
static WRITE_ERROR: OnceLock<String> = OnceLock::new();

/// Writes `text` to stdout; after the first failure, writes nothing.
pub(crate) fn stdout_write(text: &str) {
    if CLOSED.load(Ordering::Relaxed) || read_only() {
        return;
    }
    let mut o = std::io::stdout().lock();
    if let Err(e) = o.write_all(text.as_bytes()).and_then(|()| o.flush()) {
        failed(&e);
    }
}

#[cfg(unix)]
unsafe extern "C" {
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
}

#[cfg(unix)]
static STDOUT_WAS_CLOSED: AtomicBool = AtomicBool::new(false);

/// A closed fd 0, 1 or 2 at exec (`mt ... >&-`). std's runtime opens
/// `/dev/null` read-write on each closed standard fd before `main`, so by
/// `main` a closed stdout looks like a writable one and its output is lost
/// at exit 0. This runs earlier, from the executable's initializer list: it
/// records a closed fd 1 and plugs each closed fd with `/dev/null` itself
/// (the lowest free fd, so it lands on that number and stays open for the
/// whole run), so no file `mt` opens later (an `--out` file) can become
/// stdout. A target without such a list keeps std's plug only.
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
    target_os = "macos"
))]
mod closed_fds {
    use std::sync::atomic::Ordering;

    unsafe extern "C" {
        fn open(path: *const std::ffi::c_char, flags: i32, ...) -> i32;
        fn close(fd: i32) -> i32;
    }

    const F_GETFD: i32 = 1;
    const O_RDWR: i32 = 2;
    const EBADF: i32 = 9;

    extern "C" fn probe() {
        for fd in 0..=2 {
            // SAFETY: fcntl(F_GETFD) and open() of a NUL-terminated literal
            // take no other pointer; a bad fd or failed open returns -1.
            unsafe {
                if super::fcntl(fd, F_GETFD) != -1
                    || std::io::Error::last_os_error().raw_os_error() != Some(EBADF)
                {
                    continue;
                }
                if fd == 1 {
                    super::STDOUT_WAS_CLOSED.store(true, Ordering::Relaxed);
                }
                let n = open(c"/dev/null".as_ptr(), O_RDWR);
                if n >= 0 && n != fd {
                    close(n);
                }
            }
        }
    }

    #[used]
    #[cfg_attr(not(target_os = "macos"), unsafe(link_section = ".init_array"))]
    #[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__mod_init_func"))]
    static PROBE: extern "C" fn() = probe;
}

/// std's stdout reports EBADF as a successful write, so a stdout opened
/// read-only (`1</dev/null`) or closed at exec (`>&-`, see `closed_fds`)
/// would lose the output with exit 0 (S1d). Checked once, at the first stdout
/// write (or clap's `--help`/`--version`): then EBADF is recorded as the write
/// error. A verb that writes nothing to stdout never checks.
pub(crate) fn read_only() -> bool {
    static RO: OnceLock<bool> = OnceLock::new();
    *RO.get_or_init(|| {
        #[cfg(unix)]
        {
            const F_GETFL: i32 = 3;
            const O_ACCMODE: i32 = 3;
            const O_WRONLY: i32 = 1;
            const O_RDWR: i32 = 2;
            // SAFETY: fcntl(F_GETFL) takes no pointer; a bad fd returns -1.
            let fl = unsafe { fcntl(1, F_GETFL) };
            let ro = STDOUT_WAS_CLOSED.load(Ordering::Relaxed)
                || (fl >= 0 && !matches!(fl & O_ACCMODE, O_WRONLY | O_RDWR));
            if ro {
                failed(&std::io::Error::from_raw_os_error(9));
            }
            ro
        }
        #[cfg(not(unix))]
        {
            false
        }
    })
}

/// Records a failed stdout write (also used for clap's `--help`/`--version`).
pub(crate) fn failed(e: &std::io::Error) {
    CLOSED.store(true, Ordering::Relaxed);
    if e.kind() != std::io::ErrorKind::BrokenPipe {
        let _ = WRITE_ERROR.set(e.to_string());
    }
}

/// Records a failed `--out` write, with the text that follows
/// `mt: cannot write the output: ` (W3).
pub(crate) fn failed_text(text: String) {
    let _ = WRITE_ERROR.set(text);
}

/// Writes to stderr, ignoring a failure.
pub(crate) fn stderr_write(args: std::fmt::Arguments<'_>) {
    let _ = std::io::stderr().lock().write_fmt(args);
}

/// Records a bit the run has reported without stopping on it (R1: the status
/// is the sum of what the reported outcome contains).
pub(crate) fn note(bit: Exit) {
    NOTED.fetch_or(bit.to_exit_code(), Ordering::Relaxed);
}

/// The process exit code: `outcome` plus every [`note`]d bit, plus IO (32) if
/// an output write failed for a reason other than a closed pipe; then the line
/// is printed, as the last stderr line. A code that never combines (usage,
/// internal) is kept as it is.
pub(crate) fn exit_code(outcome: Exit) -> u8 {
    let mut code = outcome;
    if outcome != Exit::USAGE {
        code |= noted();
    }
    if let Some(e) = WRITE_ERROR.get() {
        stderr_write(format_args!("mt: cannot write the output: {e}\n"));
        code |= Exit::IO;
    }
    code.to_exit_code()
}

fn noted() -> Exit {
    let n = NOTED.load(Ordering::Relaxed);
    [
        Exit::NO,
        Exit::INVALID,
        Exit::UNVERIFIED,
        Exit::REFUSED,
        Exit::REPAIRED,
        Exit::IO,
    ]
    .into_iter()
    .filter(|b| n & b.to_exit_code() != 0)
    .fold(Exit::OK, |a, b| a | b)
}
