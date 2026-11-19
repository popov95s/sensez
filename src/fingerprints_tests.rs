use super::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Ns {
    Source,
}

#[test]
fn identity_defaults_content_to_hash() {
    let print = Fingerprint::identity(42, Ns::Source, "src/lib.rs", Ns::Source);
    assert_eq!(print.content_hash, 42);
    assert_eq!(print.key(), "2a");
}

#[test]
fn hash_parts_is_deterministic_and_order_sensitive() {
    let first = hash_parts(&["a", "b"]);
    assert_eq!(first, hash_parts(&["a", "b"]));
    assert_ne!(first, hash_parts(&["b", "a"]));
    assert_ne!(first, hash_parts(&["ab"]));
    assert_ne!(hash_bytes(b"x = 1\n"), hash_bytes(b"x = 2\n"));
}

#[test]
fn persisted_hashes_match_pinned_values() {
    assert_eq!(
        hash_parts(&["app", "orphan", "function"]),
        0x5fad_aa35_75b4_44cc
    );
    assert_eq!(hash_bytes(b"x = 1\n"), 0x5e2c_7e9d_0099_3a31);
    assert_eq!(hash_bytes(b""), 0xd1fb_a762_150c_532c);
    assert_eq!(
        hash_bytes(b"a much longer input that spans several sip words"),
        0x6cf6_e552_073f_95e1
    );
}
