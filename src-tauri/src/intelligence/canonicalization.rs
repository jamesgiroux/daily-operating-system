//! Canonical hashing for tombstone item identity.
//! Shared by W0 callers and DOS-7 commit_claim/propose_claim.

use sha2::{Digest, Sha256};

/// The kind of intelligence item being hashed; reserved for forward-compat
/// with DOS-7's claim_type registry (ADR-0125). For W0 we only emit `Risk`/`Win`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::manual_non_exhaustive)]
pub enum ItemKind {
    /// Risk item text.
    Risk,
    /// Recent-win item text.
    Win,
    /// Reserved for DOS-7 expansion; not emitted in W0.
    #[doc(hidden)]
    _Reserved,
}

/// Canonical hash for tombstone matching.
///
/// Stable rule (locked for v1.4.0):
/// 1. Trim leading/trailing whitespace.
/// 2. NFC-normalize Unicode.
/// 3. Collapse internal whitespace runs to a single space.
/// 4. SHA-256 hex digest.
///
/// This rule is intentionally not case-folded and not punctuation-stripped.
/// Changing it is a data migration because existing tombstone hashes are
/// persisted and compared byte-for-byte.
pub fn item_hash(_kind: ItemKind, text: &str) -> String {
    let canonical = canonical_text(text);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn canonical_text(text: &str) -> String {
    let trimmed = text.trim();
    let nfc = normalize_nfc(trimmed);
    nfc.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(target_os = "macos")]
fn normalize_nfc(input: &str) -> String {
    use std::ffi::c_void;
    use std::ptr;

    type CFIndex = isize;
    type CFAllocatorRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFMutableStringRef = *mut c_void;
    type CFStringNormalizationForm = CFIndex;

    #[repr(C)]
    struct CFRange {
        location: CFIndex,
        length: CFIndex,
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateMutable(
            alloc: CFAllocatorRef,
            max_length: CFIndex,
        ) -> CFMutableStringRef;
        fn CFStringAppendCharacters(
            the_string: CFMutableStringRef,
            chars: *const u16,
            num_chars: CFIndex,
        );
        fn CFStringNormalize(
            the_string: CFMutableStringRef,
            the_form: CFStringNormalizationForm,
        );
        fn CFStringGetLength(the_string: CFStringRef) -> CFIndex;
        fn CFStringGetCharacters(
            the_string: CFStringRef,
            range: CFRange,
            buffer: *mut u16,
        );
        fn CFRelease(cf: *const c_void);
    }

    if input.is_empty() {
        return String::new();
    }

    let utf16 = input.encode_utf16().collect::<Vec<_>>();

    // SAFETY: CoreFoundation copies the UTF-16 buffer into a mutable CFString,
    // normalizes it in place, then copies the normalized contents into a Rust
    // Vec before the CF object is released.
    unsafe {
        let cf_string = CFStringCreateMutable(ptr::null(), 0);
        if cf_string.is_null() {
            return input.to_string();
        }

        CFStringAppendCharacters(cf_string, utf16.as_ptr(), utf16.len() as CFIndex);
        CFStringNormalize(cf_string, 2);

        let len = CFStringGetLength(cf_string as CFStringRef);
        let mut buffer = vec![0_u16; len as usize];
        CFStringGetCharacters(
            cf_string as CFStringRef,
            CFRange {
                location: 0,
                length: len,
            },
            buffer.as_mut_ptr(),
        );
        CFRelease(cf_string as *const c_void);

        String::from_utf16(&buffer).unwrap_or_else(|_| input.to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn normalize_nfc(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if matches!(chars.peek(), Some('\u{0301}')) {
            if let Some(composed) = compose_acute(ch) {
                let _ = chars.next();
                output.push(composed);
                continue;
            }
        }
        output.push(ch);
    }
    output
}

#[cfg(not(target_os = "macos"))]
fn compose_acute(previous: char) -> Option<char> {
    match previous {
        'a' => Some('á'),
        'A' => Some('Á'),
        'e' => Some('é'),
        'E' => Some('É'),
        'i' => Some('í'),
        'I' => Some('Í'),
        'o' => Some('ó'),
        'O' => Some('Ó'),
        'u' => Some('ú'),
        'U' => Some('Ú'),
        'y' => Some('ý'),
        'Y' => Some('Ý'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{item_hash, ItemKind};

    #[test]
    fn canonicalization_basic_ascii() {
        assert_eq!(
            item_hash(ItemKind::Risk, "ARR at risk"),
            item_hash(ItemKind::Risk, "ARR at risk")
        );
    }

    #[test]
    fn canonicalization_nfc_normalization() {
        assert_eq!(
            item_hash(ItemKind::Risk, "Cafe\u{301} renewal"),
            item_hash(ItemKind::Risk, "Café renewal")
        );
    }

    #[test]
    fn canonicalization_whitespace_collapse() {
        assert_eq!(
            item_hash(ItemKind::Win, "  New\tchampion\nidentified  "),
            item_hash(ItemKind::Win, "New champion identified")
        );
    }

    #[test]
    fn canonicalization_mixed_unicode() {
        assert_eq!(
            item_hash(ItemKind::Risk, "  Cafe\u{301}\t東京  "),
            item_hash(ItemKind::Risk, "Café 東京")
        );
    }

    #[test]
    fn canonicalization_idempotent_same_input() {
        let first = item_hash(ItemKind::Risk, "Procurement risk?");
        let second = item_hash(ItemKind::Risk, "Procurement risk?");
        assert_eq!(first, second);
    }
}
