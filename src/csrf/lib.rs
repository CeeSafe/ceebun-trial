//! CSRF Token implementation for Bun
//! It provides protection against Cross-Site Request Forgery attacks
//! by generating and validating tokens using HMAC signatures
//!
//! The implementation is CeeSafe (`ceebun/bun_csrf`), linked into Bun as one object; this crate keeps the public API
//! and calls its two C exports. Every buffer is the caller's: the slices are lent for the call, nothing is allocated
//! on one side and freed on the other.

#![warn(unused_must_use)]
use bun_sha_hmac::evp::Algorithm;

/// Default HMAC algorithm used for token signing
pub const DEFAULT_ALGORITHM: Algorithm = Algorithm::Sha256;

/// Error types for CSRF operations
#[derive(strum::IntoStaticStr, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    TokenCreationFailed,
}
bun_core::impl_tag_error!(Error);

/// Options for generating CSRF tokens.
pub struct GenerateOptions<'a> {
    /// Secret key to use for signing
    pub secret: &'a [u8],
    /// Per-principal associated data mixed into the HMAC; an empty slice
    /// means the token is not bound to any principal
    pub session_id: &'a [u8],
    /// How long the token should be valid (in milliseconds)
    pub expires_in_ms: u64,
    /// Algorithm to use for signing
    pub algorithm: Algorithm, // = DEFAULT_ALGORITHM
}

/// Options for validating CSRF tokens.
pub struct VerifyOptions<'a> {
    /// The token to verify
    pub token: &'a [u8],
    /// Secret key used to sign the token
    pub secret: &'a [u8],
    /// Per-principal associated data mixed into the HMAC; an empty slice
    /// means the token is not bound to any principal
    pub session_id: &'a [u8],
    /// Maximum age of the token in milliseconds
    pub max_age_ms: u64,
    /// Encoding to use for the token
    pub encoding: TokenFormat, // = .base64url
    /// Algorithm to use for signing
    pub algorithm: Algorithm, // = DEFAULT_ALGORITHM
}

/// Token encoding format
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenFormat {
    Base64,
    Base64Url,
    Hex,
}

// The CeeSafe side maps `Algorithm` from its declaration-order discriminant.
const _: () = assert!(
    Algorithm::Blake2b256 as u8 == 0
        && Algorithm::Sha256 as u8 == 8
        && Algorithm::Shake256 as u8 == 18
);

unsafe extern "C" {
    fn bun_csrf__generate(
        secret: *const u8,
        secret_len: usize,
        session_id: *const u8,
        session_id_len: usize,
        expires_in_ms: u64,
        algorithm: u8,
        out_buffer: *mut u8,
    ) -> usize;
    fn bun_csrf__verify(
        token: *const u8,
        token_len: usize,
        secret: *const u8,
        secret_len: usize,
        session_id: *const u8,
        session_id_len: usize,
        max_age_ms: u64,
        encoding: TokenFormat,
        algorithm: u8,
    ) -> bool;
}

/// Generate a new CSRF token
///
/// Parameters:
/// - options: Configuration for token generation
/// - out_buffer: caller-provided buffer for the raw token bytes
///
/// Returns: A slice into `out_buffer` containing the raw token
pub fn generate<'a>(
    options: &GenerateOptions<'_>,
    out_buffer: &'a mut [u8; 512],
) -> Result<&'a mut [u8], Error> {
    // SAFETY: the slices are live for the call and the export keeps none of them; `out_buffer` is 512 writable bytes.
    let len = unsafe {
        bun_csrf__generate(
            options.secret.as_ptr(),
            options.secret.len(),
            options.session_id.as_ptr(),
            options.session_id.len(),
            options.expires_in_ms,
            options.algorithm as u8,
            out_buffer.as_mut_ptr(),
        )
    };
    if len == 0 {
        return Err(Error::TokenCreationFailed);
    }
    Ok(&mut out_buffer[0..len])
}

/// Validate a CSRF token
///
/// Parameters:
/// - options: Configuration for token validation
///
/// Returns: true if valid, false if invalid
pub fn verify(options: &VerifyOptions<'_>) -> bool {
    // SAFETY: the slices are live for the call, nothing writes them meanwhile, and the export keeps none of them.
    unsafe {
        bun_csrf__verify(
            options.token.as_ptr(),
            options.token.len(),
            options.secret.as_ptr(),
            options.secret.len(),
            options.session_id.as_ptr(),
            options.session_id.len(),
            options.max_age_ms,
            options.encoding,
            options.algorithm as u8,
        )
    }
}

// NOTE: JS bindings live in the *_jsc crate as extension methods.
