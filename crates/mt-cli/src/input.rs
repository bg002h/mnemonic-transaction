//! Reading what the operator handed us — §8.2e's **ordered** sniffing procedure.
//!
//! The spec calls this an ordered procedure rather than a set of recognisers,
//! and the order is load-bearing at one point in particular: **binary is tested
//! before whitespace removal**, because `0x09`, `0x0a` and `0x20` are ordinary
//! bytes inside a binary PSBT and stripping them corrupts it. Everything after
//! step 2 is text.
//!
//! The steps exist because real user input falls through the obvious ones:
//! line-wrapped base64 is what many wallets and `openssl`-style exports produce,
//! a trailing newline is what every `.psbt` file and terminal paste carries, and
//! uppercase hex with an `0x` prefix is plausible from a person.

use crate::refusal::Refusal;

/// What the operator gave us, after sniffing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// A PSBT, binary or base64-decoded.
    Psbt(Vec<u8>),
    /// A raw signed transaction, from hex.
    ///
    /// **Accepted, loudly warned** (§8.2e). Bitcoin Core's `finalizepsbt`
    /// defaults `extract=true` and returns hex, so refusing this would refuse
    /// the default output of the standard tool at exactly the moment this
    /// workflow starts.
    RawHex(Vec<u8>),
}

/// The PSBT magic, `psbt\xff`.
const PSBT_MAGIC: &[u8] = b"psbt\xff";
/// Base64 PSBTs always begin here — it is `psbt\xff` in base64.
const PSBT_BASE64_PREFIX: &str = "cHNidP8";

/// Sniff, in the order §8.2e rules.
pub fn sniff(raw: &[u8]) -> Result<Input, Refusal> {
    // 1. Trim leading/trailing whitespace including CRLF. NOT interior — see below.
    let trimmed = trim_outer(raw);

    // 2. Binary PSBT, BEFORE any interior whitespace is touched.
    if trimmed.starts_with(PSBT_MAGIC) {
        return Ok(Input::Psbt(trimmed.to_vec()));
    }

    // 3. From here it is text. Remove ALL interior whitespace, so line-wrapped
    //    exports at 64 or 76 columns are handled.
    let text: String = match core::str::from_utf8(trimmed) {
        Ok(s) => s.chars().filter(|c| !c.is_whitespace()).collect(),
        Err(_) => return Err(unrecognised(trimmed)),
    };

    //    (a) base64 PSBT
    if text.starts_with(PSBT_BASE64_PREFIX) {
        return match base64_decode(&text) {
            Some(bytes) if bytes.starts_with(PSBT_MAGIC) => Ok(Input::Psbt(bytes)),
            _ => Err(Refusal::new(
                "encode",
                "§8.2e",
                "input looks like a base64 PSBT but does not decode",
                "The `cHNidP8` prefix marks a base64-encoded PSBT, but decoding it \
                 did not yield the `psbt\\xff` magic. The text is probably truncated \
                 or has had characters altered in transit.",
            )
            .with_remedy("Re-export the PSBT and pass the file with --in.")),
        };
    }

    //    (b) raw hex — optional 0x, any case, even length
    let hex_body = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(&text);
    if !hex_body.is_empty()
        && hex_body.len() % 2 == 0
        && hex_body.chars().all(|c| c.is_ascii_hexdigit())
    {
        let bytes = hex_to_bytes(hex_body);
        // A hex-encoded PSBT is valid hex AND a PSBT. It is the one genuinely
        // ambiguous input, and the refusal must name the REAL problem — telling
        // someone "invalid transaction" sends them to look at the wrong thing.
        if bytes.starts_with(PSBT_MAGIC) {
            return Err(Refusal::new(
                "encode",
                "§8.2e",
                "input is a hex-encoded PSBT, not a raw transaction",
                "These bytes are valid hex, and decoding them yields the `psbt\\xff` \
                 magic — so this is a PSBT that has been hex-encoded rather than a \
                 raw signed transaction. mt reads PSBTs in binary or base64.",
            )
            .with_remedy("Pass the .psbt file directly with --in, or convert it to base64."));
        }
        return Ok(Input::RawHex(bytes));
    }

    // 4. Nothing matched.
    recognised_guard(false, trimmed)?;
    unreachable!("recognised_guard(false, ..) returns Err")
}

/// §8.2e step 4, as a **guard** rather than an inline `return`.
///
/// The shape is deliberate: every entry in `tests/refusals.toml` names a
/// `fn … -> Result<(), Refusal>` so `scripts/mutate-refusals.sh` can locate and
/// neuter exactly one check by name. Neutered, `sniff` reaches the
/// `unreachable!` and panics rather than quietly accepting the input — which is
/// a *stronger* control than silent acceptance, not a weaker one, since a
/// panicking binary cannot print the refusal the test is looking for either.
pub fn recognised_guard(matched: bool, bytes: &[u8]) -> Result<(), Refusal> {
    if matched {
        return Ok(());
    }
    Err(unrecognised(bytes))
}

/// §8.2e step 4: name what was seen. Never a bare "invalid input" — an operator
/// who cannot tell *what* mt thought it received cannot fix it.
fn unrecognised(bytes: &[u8]) -> Refusal {
    let head: String = bytes
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    Refusal::new(
        "encode",
        "§8.2e",
        format!(
            "input is not a PSBT or a raw transaction ({} bytes)",
            bytes.len()
        ),
        format!(
            "mt accepts a binary PSBT (magic `psbt\\xff`), a base64 PSBT (beginning \
             `cHNidP8`), or a raw signed transaction as hex. This input begins {head} \
             and matches none of them."
        ),
    )
    .with_remedy("Check the file is the one you meant, and pass it with --in.")
}

fn trim_outer(raw: &[u8]) -> &[u8] {
    let start = raw
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(raw.len());
    let end = raw
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(start, |i| i + 1);
    &raw[start..end]
}

fn hex_to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks(2)
        .map(|p| {
            let hi = (p[0] as char).to_digit(16).unwrap() as u8;
            let lo = (p[1] as char).to_digit(16).unwrap() as u8;
            (hi << 4) | lo
        })
        .collect()
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        let v = TABLE.iter().position(|&t| t == c)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn psbt_bytes() -> Vec<u8> {
        let mut v = PSBT_MAGIC.to_vec();
        v.extend_from_slice(&[0x01, 0x00, 0x00]);
        v
    }

    #[test]
    fn binary_psbt_is_recognised_before_whitespace_is_touched() {
        // A binary PSBT whose body contains 0x0a and 0x20 — ordinary bytes that
        // stripping would destroy. This is why step 2 precedes step 3.
        let mut v = PSBT_MAGIC.to_vec();
        v.extend_from_slice(&[0x0a, 0x20, 0x09, 0x00]);
        match sniff(&v).unwrap() {
            Input::Psbt(b) => assert_eq!(b, v, "interior whitespace bytes were stripped"),
            other => panic!("binary PSBT sniffed as {other:?}"),
        }
    }

    #[test]
    fn line_wrapped_base64_is_accepted() {
        let b64 = base64_encode(&psbt_bytes());
        let mut wrapped = String::new();
        for c in b64.as_bytes().chunks(4) {
            wrapped.push_str(core::str::from_utf8(c).unwrap());
            wrapped.push('\n');
        }
        assert!(matches!(sniff(wrapped.as_bytes()).unwrap(), Input::Psbt(_)));
    }

    #[test]
    fn hex_is_accepted_in_any_case_with_or_without_0x() {
        for s in ["deadbeef", "DEADBEEF", "0xdeadbeef", "  deadbeef\r\n"] {
            assert!(
                matches!(sniff(s.as_bytes()).unwrap(), Input::RawHex(b) if b == vec![0xde,0xad,0xbe,0xef]),
                "hex form {s:?} not accepted"
            );
        }
    }

    /// The one genuinely ambiguous input. It is valid hex AND a PSBT, and the
    /// refusal must name the real problem.
    #[test]
    fn hex_encoded_psbt_is_refused_by_name() {
        use core::fmt::Write as _;
        let mut hex = String::new();
        for b in psbt_bytes() {
            let _ = write!(hex, "{b:02x}");
        }
        let r = sniff(hex.as_bytes()).unwrap_err();
        assert!(
            r.verdict.contains("hex-encoded PSBT"),
            "refusal does not name the real problem: {}",
            r.verdict
        );
    }

    #[test]
    fn unrecognised_input_names_what_was_seen() {
        let r = sniff(b"\x01\x02\x03not-anything").unwrap_err();
        assert!(
            r.mechanism.contains("01 02 03"),
            "refusal does not show the bytes"
        );
    }

    fn base64_encode(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for c in bytes.chunks(3) {
            let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= c.len() {
                    out.push(TABLE[((n >> (18 - 6 * i)) & 0x3F) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }
}
