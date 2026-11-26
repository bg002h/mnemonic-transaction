//! bughunt-1119 FIX-J (mt): printed remedies and the documented live script,
//! keys bh-mt-I22 and bh-mt-I25 (architect ruling T15; Brian B8: remedy text
//! and README commands count as printed). Spec: design/FIX_bh1119_mt_printed_remedies.md.
//!
//! I22: a remedy that names `bitcoin-cli finalizepsbt` tells the operator to
//! pass the `hex` field of Core's answer, never the whole answer, which `encode`
//! refuses (T15: no new input form). I25: `scripts/live-smoke-test.sh`, run as
//! README.md prints it under the default umask 022, reaches its last stage, and
//! a failing mt step prints that step's stderr before the EXIT trap deletes it.

use assert_cmd::Command;
use std::io::Write;

mod common;
use common::exit::INVALID;

/// The remedy texts this FIX sets (PROPOSED, architect confirms). A wording
/// change updates these constants and nothing else.
const REMEDY_UNFINALIZED: &str = "Run `bitcoin-cli finalizepsbt <psbt>`. If its answer shows \
`\"complete\": true`, pass the value of its `hex` field (the signed transaction), not the \
whole answer. If it shows `\"complete\": false`, the PSBT still lacks signatures: sign it \
first (`walletprocesspsbt`), then run `finalizepsbt` again.";
const REMEDY_TXID: &str = "Fetch the transaction: `bitcoin-cli getrawtransaction <txid>`, then \
pass that. If you have not broadcast yet, run `bitcoin-cli finalizepsbt <psbt>` instead and \
pass the value of its `hex` field (present once it shows `\"complete\": true`).";

/// `mt`, offline by default: no `bitcoin-cli` on the child's `PATH`, so a
/// machine with one never sends it fixture txids. A test that wants a node
/// names a stub with `--bitcoin-cli`.
fn mt() -> Command {
    let mut c = Command::cargo_bin("mt").unwrap();
    c.env("PATH", "/nonexistent");
    c
}

fn base() -> serde_json::Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/p5_base.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn file_with(text: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(text.as_bytes()).unwrap();
    f.flush().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    f
}

struct Run {
    code: Option<i32>,
    out: String,
    err: String,
}

fn encode_offline(input: &str) -> Run {
    let f = file_with(input);
    let o = mt()
        .env("PATH", "/nonexistent")
        .args(["encode", "--in"])
        .arg(f.path())
        .output()
        .unwrap();
    Run {
        code: o.status.code(),
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
        err: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// The last paragraph of the refusal that starts at `header`, unwrapped: the
/// remedy (neither refusal below carries a verbatim block). `Refusal`'s Display
/// wraps at 68 columns and indents by two spaces; joining the lines with one
/// space restores the text, which has no double spaces.
fn remedy_of(err: &str, header: &str) -> String {
    let at = err
        .find(header)
        .unwrap_or_else(|| panic!("no refusal starting {header:?}; stderr:\n{err}"));
    let block = err[at..]
        .split("\n\n")
        .filter(|b| !b.trim().is_empty())
        .last()
        .unwrap();
    block.lines().map(str::trim).collect::<Vec<_>>().join(" ")
}

/// Row 1. The §8.1 refusal (a PSBT with unfinalized inputs) prints the remedy
/// that names the `hex` field and both `complete` outcomes.
#[test]
fn i22_unfinalized_psbt_remedy_names_hex_field() {
    let r = encode_offline(base()["unfinalized_psbt_b64"].as_str().unwrap());
    assert_eq!(
        r.code,
        Some(INVALID),
        "encode did not refuse; stderr:\n{}",
        r.err
    );
    assert!(r.out.is_empty(), "a refused encode wrote stdout: {}", r.out);
    assert_eq!(
        remedy_of(&r.err, "mt encode: REFUSED — §8.1, "),
        REMEDY_UNFINALIZED
    );
}

/// Row 2. The §10.10 refusal (a txid where a transaction belongs) prints the
/// remedy whose `finalizepsbt` branch names the `hex` field.
#[test]
fn i22_txid_remedy_names_hex_field() {
    let (own, _) = common::fixture_txids(&base());
    let r = encode_offline(&own);
    assert_eq!(
        r.code,
        Some(INVALID),
        "encode did not refuse; stderr:\n{}",
        r.err
    );
    assert!(r.out.is_empty(), "a refused encode wrote stdout: {}", r.out);
    assert_eq!(
        remedy_of(&r.err, "mt encode: REFUSED — §10.10, "),
        REMEDY_TXID
    );
}

/// Row 2b. The §10.10 refusal from a reading verb that takes only mt1 strings
/// (`decode`, `verify`) sends the operator to `mt inspect`.
#[test]
fn i22_txid_remedy_reading_verbs_point_to_inspect() {
    let (own, _) = common::fixture_txids(&base());
    for verb in ["decode", "verify"] {
        let f = file_with(&own);
        let o = mt()
            .env("PATH", "/nonexistent")
            .args([verb, "--in"])
            .arg(f.path())
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        assert_eq!(
            o.status.code(),
            Some(INVALID),
            "{verb} did not refuse; stderr:\n{err}"
        );
        assert_eq!(
            remedy_of(&err, &format!("mt {verb}: REFUSED — §10.10, ")),
            format!(
                "`mt {verb}` reads the mt1 strings from the plates, not a transaction. To look at \
the transaction this txid names, fetch it with `bitcoin-cli getrawtransaction <txid>` and pass \
that to `mt inspect`."
            )
        );
    }
}

/// Row 3 (guard, GREEN on base; pins T15). Core's whole `finalizepsbt` answer
/// is still refused, and the remedy as now printed works: its `hex` field
/// encodes.
#[test]
fn i22_finalizepsbt_answer_refused_hex_field_encodes() {
    let raw = base()["raw_hex"].as_str().unwrap().to_string();
    let answer = format!("{{\"hex\":\"{raw}\",\"complete\":true}}");
    let whole = encode_offline(&answer);
    assert_ne!(whole.code, Some(101), "mt panicked: {}", whole.err);
    assert_ne!(
        whole.code,
        Some(0),
        "encode accepted Core's whole answer (T15: no new input form)"
    );
    assert!(
        whole.out.is_empty(),
        "a refused encode wrote stdout: {}",
        whole.out
    );
    let field = encode_offline(&raw);
    assert_eq!(
        field.code,
        Some(0),
        "the hex field did not encode; stderr:\n{}",
        field.err
    );
    assert!(
        field.out.starts_with("mt1"),
        "no mt1 strings on stdout: {}",
        field.out
    );
}

/// A scripted `bitcoin-cli` that answers every verb the live script and mt
/// use. Not Core: it reproduces the script's control flow, not node behaviour.
/// An unknown txid exits 5, Core's not-found code (as `tests/common` does).
fn scripted_node(root: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let base = base();
    let fin = base["finalized_psbt_b64"].as_str().unwrap();
    let (_, parents) = common::fixture_txids(&base);
    let mut arms = String::new();
    for p in &parents {
        arms += &format!("    {p}*) echo '{{\"confirmations\": 5}}' ;;\n");
    }
    let rt = root.join("rt");
    std::fs::write(
        &rt,
        format!(
            r#"#!/bin/sh
if [ $# -gt 0 ] && [ "$1" != "-stdin" ]; then verb="$1"; arg1="$2"; else
verb=""; arg1=""
while read -r line; do
  if [ -z "$verb" ]; then verb="$line"; elif [ -z "$arg1" ]; then arg1="$line"; fi
done; fi
case "$verb" in
  getblockcount) echo 963832 ;;
  getindexinfo) echo '{{"txindex": {{"synced": true, "best_block_height": 963832}}}}' ;;
  getblockchaininfo) echo '{{"chain": "regtest", "mediantime": 1700000000}}' ;;
  gettxout) case "$arg1" in d13d*) echo '{{"value": 3.0, "confirmations": 5}}' ;; *) echo '{{"value": 5.0, "confirmations": 5}}' ;; esac ;;
  getrawtransaction)
    case "$arg1" in
{arms}      *) exit 5 ;;
    esac ;;
  listunspent) echo '[{{"txid":"{p0}","vout":1,"amount":8.0}}]' ;;
  getnewaddress) echo bcrt1q07h88fcj0j86excq5m9k97e26su7j5tdys0686 ;;
  createpsbt|utxoupdatepsbt) echo {fin} ;;
  walletprocesspsbt) echo '{{"psbt":"{fin}","complete":true}}' ;;
  finalizepsbt) echo '{{"psbt":"{fin}","complete":true}}' ;;
  testmempoolaccept) echo '[{{"allowed":true}}]' ;;
  *) exit 1 ;;
esac
"#,
            p0 = parents[0]
        ),
    )
    .unwrap();
    std::fs::set_permissions(&rt, std::fs::Permissions::from_mode(0o700)).unwrap();
    rt
}

/// A temp repo root holding a copy of the live script and `target/debug/mt`.
fn script_root(mt_bin: Option<&str>) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("scripts")).unwrap();
    std::fs::create_dir_all(root.path().join("target/debug")).unwrap();
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/live-smoke-test.sh"
        ),
        root.path().join("scripts/live-smoke-test.sh"),
    )
    .unwrap();
    let mt_path = root.path().join("target/debug/mt");
    match mt_bin {
        None => std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_mt"), &mt_path).unwrap(),
        Some(body) => {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&mt_path, body).unwrap();
            std::fs::set_permissions(&mt_path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
    }
    root
}

/// Run the script as README.md:52 prints it, from a shell whose umask is 022.
fn run_script(root: &std::path::Path, rt: &std::path::Path) -> std::process::Output {
    std::process::Command::new("bash")
        .arg("-c")
        .arg("umask 022; exec scripts/live-smoke-test.sh")
        .current_dir(root)
        .env("MT_RT", rt)
        .output()
        .unwrap()
}

/// Row 4. Under umask 022 the script reaches stage 5 and exits 0.
#[test]
fn i25_live_smoke_test_under_umask_022() {
    let root = script_root(None);
    let rt = scripted_node(root.path());
    let out = run_script(root.path(), &rt);
    let so = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && so.contains("   allowed: true"),
        "status {:?}\nstdout:\n{so}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Row 5. A failing mt step prints its exit status and its stderr, indented,
/// before the EXIT trap deletes the work directory.
#[test]
fn i25_live_smoke_test_failure_shows_diagnostic() {
    let fake = "#!/bin/sh\necho 'mt encode: REFUSED — fake refusal for this test' >&2\nexit 64\n";
    let root = script_root(Some(fake));
    let rt = scripted_node(root.path());
    let out = run_script(root.path(), &rt);
    let so = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "stdout:\n{so}");
    assert!(
        so.lines()
            .any(|l| l == "FAIL: mt encode exited 64; its stderr follows:"),
        "no FAIL line; stdout:\n{so}"
    );
    assert!(
        so.lines()
            .any(|l| l == "   mt encode: REFUSED — fake refusal for this test"),
        "the step's stderr was not shown; stdout:\n{so}"
    );
}
