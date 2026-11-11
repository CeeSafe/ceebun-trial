#![warn(unused_must_use)]
use core::ffi::{c_char, c_void};

use bun_core::ZStr;

// ─── FFI bindings ─────────────────────────────────────────────────────────
// Externs stay in this crate per PORTING.md §FFI: "If your file has externs
// and isn't already *_sys, leave them in place".
#[allow(non_camel_case_types, non_upper_case_globals)]
pub mod c {
    use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

    // `ZSTD_DStream` — opaque streaming-decompression context (Nomicon FFI pattern).
    //
    // `UnsafeCell` makes the type `!Freeze` so a `&ZSTD_DStream` does not assert
    // immutability of the C-owned state (zstd mutates internally on every call).
    bun_opaque::opaque_ffi! {
        pub struct ZSTD_DStream;
        /// `ZSTD_CCtx` — opaque streaming-compression context.
        pub struct ZSTD_CCtx;
    }

    /// `typedef ZSTD_DCtx ZSTD_DStream;` — same opaque object.
    pub(crate) type ZSTD_DCtx = ZSTD_DStream;

    // C enums passed by value across FFI — model as `c_uint` (their declared
    // underlying type) so callers can pass raw values without transmute.
    pub(crate) type ZSTD_ErrorCode = c_uint;
    pub(crate) type ZSTD_EndDirective = c_uint;
    pub(crate) type ZSTD_ResetDirective = c_uint;
    pub(crate) type ZSTD_cParameter = c_uint;
    pub(crate) type ZSTD_dParameter = c_uint;

    // ZSTD_EndDirective
    pub const ZSTD_e_continue: ZSTD_EndDirective = 0;

    // ZSTD_cParameter
    pub const ZSTD_c_compressionLevel: ZSTD_cParameter = 100;

    pub const ZSTD_reset_session_and_parameters: ZSTD_ResetDirective = 3;

    // ZSTD_ErrorCode (zstd_errors.h) — only the public stable subset.
    pub const ZSTD_error_no_error: ZSTD_ErrorCode = 0;
    pub const ZSTD_error_GENERIC: ZSTD_ErrorCode = 1;
    pub const ZSTD_error_prefix_unknown: ZSTD_ErrorCode = 10;
    pub const ZSTD_error_version_unsupported: ZSTD_ErrorCode = 12;
    pub const ZSTD_error_frameParameter_unsupported: ZSTD_ErrorCode = 14;
    pub const ZSTD_error_frameParameter_windowTooLarge: ZSTD_ErrorCode = 16;
    pub const ZSTD_error_corruption_detected: ZSTD_ErrorCode = 20;
    pub const ZSTD_error_checksum_wrong: ZSTD_ErrorCode = 22;
    pub const ZSTD_error_literals_headerWrong: ZSTD_ErrorCode = 24;
    pub const ZSTD_error_dictionary_corrupted: ZSTD_ErrorCode = 30;
    pub const ZSTD_error_dictionary_wrong: ZSTD_ErrorCode = 32;
    pub const ZSTD_error_dictionaryCreation_failed: ZSTD_ErrorCode = 34;
    pub const ZSTD_error_parameter_unsupported: ZSTD_ErrorCode = 40;
    pub const ZSTD_error_parameter_combination_unsupported: ZSTD_ErrorCode = 41;
    pub const ZSTD_error_parameter_outOfBound: ZSTD_ErrorCode = 42;
    pub const ZSTD_error_tableLog_tooLarge: ZSTD_ErrorCode = 44;
    pub const ZSTD_error_maxSymbolValue_tooLarge: ZSTD_ErrorCode = 46;
    pub const ZSTD_error_maxSymbolValue_tooSmall: ZSTD_ErrorCode = 48;
    pub const ZSTD_error_stabilityCondition_notRespected: ZSTD_ErrorCode = 50;
    pub const ZSTD_error_stage_wrong: ZSTD_ErrorCode = 60;
    pub const ZSTD_error_init_missing: ZSTD_ErrorCode = 62;
    pub const ZSTD_error_memory_allocation: ZSTD_ErrorCode = 64;
    pub const ZSTD_error_workSpace_tooSmall: ZSTD_ErrorCode = 66;
    pub const ZSTD_error_dstSize_tooSmall: ZSTD_ErrorCode = 70;
    pub const ZSTD_error_srcSize_wrong: ZSTD_ErrorCode = 72;
    pub const ZSTD_error_dstBuffer_null: ZSTD_ErrorCode = 74;
    pub const ZSTD_error_noForwardProgress_destFull: ZSTD_ErrorCode = 80;
    pub const ZSTD_error_noForwardProgress_inputEmpty: ZSTD_ErrorCode = 82;

    #[repr(C)]
    pub struct ZSTD_inBuffer {
        pub src: *const c_void,
        pub size: usize,
        pub pos: usize,
    }

    #[repr(C)]
    pub struct ZSTD_outBuffer {
        pub dst: *mut c_void,
        pub size: usize,
        pub pos: usize,
    }

    unsafe extern "C" {
        // Pure scalar fns — no preconditions.
        pub safe fn ZSTD_isError(code: usize) -> c_uint;
        pub fn ZSTD_decompressStream(
            zds: *mut ZSTD_DStream,
            output: *mut ZSTD_outBuffer,
            input: *mut ZSTD_inBuffer,
        ) -> usize;

        // ── streaming-compress / advanced API (used by NativeZstd) ───────
        pub safe fn ZSTD_createCCtx() -> *mut ZSTD_CCtx;
        pub fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> usize;
        pub safe fn ZSTD_createDCtx() -> *mut ZSTD_DCtx;
        pub fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> usize;
        pub fn ZSTD_CCtx_setPledgedSrcSize(
            cctx: *mut ZSTD_CCtx,
            pledged_src_size: c_ulonglong,
        ) -> usize;
        /// Copies the dictionary into the context, so `dict` need not outlive
        /// the call (unlike the `_byReference` variant).
        pub fn ZSTD_CCtx_loadDictionary(
            cctx: *mut ZSTD_CCtx,
            dict: *const c_void,
            dict_size: usize,
        ) -> usize;
        /// Copies the dictionary into the context; see `ZSTD_CCtx_loadDictionary`.
        pub fn ZSTD_DCtx_loadDictionary(
            dctx: *mut ZSTD_DCtx,
            dict: *const c_void,
            dict_size: usize,
        ) -> usize;
        pub fn ZSTD_CCtx_setParameter(
            cctx: *mut ZSTD_CCtx,
            param: ZSTD_cParameter,
            value: c_int,
        ) -> usize;
        pub fn ZSTD_DCtx_setParameter(
            dctx: *mut ZSTD_DCtx,
            param: ZSTD_dParameter,
            value: c_int,
        ) -> usize;
        pub fn ZSTD_DCtx_reset(dctx: *mut ZSTD_DCtx, reset: ZSTD_ResetDirective) -> usize;
        pub fn ZSTD_compressStream2(
            cctx: *mut ZSTD_CCtx,
            output: *mut ZSTD_outBuffer,
            input: *mut ZSTD_inBuffer,
            end_op: ZSTD_EndDirective,
        ) -> usize;
        pub safe fn ZSTD_getErrorCode(function_result: usize) -> ZSTD_ErrorCode;
        pub safe fn ZSTD_getErrorString(code: ZSTD_ErrorCode) -> *const c_char;
    }
}

// -----------------------------------
// The implementation is CeeSafe (`ceebun/bun_zstd`), linked into Bun in CeeBun's one archive; this crate keeps the
// public API and calls its C exports. Slices are lent for the call. A `Vec<u8>` changes sides (CeeSafe ADR 0090):
// CeeSafe adopts the caller's `Vec`, appends to it and gives it back, or gives a new one. Both sides allocate from
// the `#[global_allocator]` (`ceebun_init` gives it to CeeSafe), which frees a block by its address alone and
// reallocates one whatever its layout was, so either side may free or grow the other's block.

unsafe extern "C" {
    fn bun_zstd__compress(
        dest: *mut u8,
        dest_len: usize,
        src: *const u8,
        src_len: usize,
        has_level: bool,
        level: i32,
        error: *mut *const c_char,
    ) -> usize;
    safe fn bun_zstd__compress_bound(src_size: usize) -> usize;
    fn bun_zstd__compress_append(
        pointer: *mut *mut u8,
        length: *mut usize,
        capacity: *mut usize,
        src: *const u8,
        src_len: usize,
        has_level: bool,
        level: i32,
        error: *mut *const c_char,
    ) -> usize;
    safe fn bun_zstd__is_error(code: usize) -> bool;
    fn bun_zstd__get_decompressed_size(src: *const u8, src_len: usize) -> usize;
    fn bun_zstd__decompress_alloc(
        src: *const u8,
        src_len: usize,
        pointer: *mut *mut u8,
        length: *mut usize,
        capacity: *mut usize,
    ) -> u8;
    fn bun_zstd__decompress_alloc_with_prefix(
        src: *const u8,
        src_len: usize,
        prefix: *const u8,
        prefix_len: usize,
        pointer: *mut *mut u8,
        length: *mut usize,
        capacity: *mut usize,
    ) -> u8;
    fn bun_zstd__inflate_embedded(
        compressed: *const u8,
        compressed_len: usize,
        nul: bool,
        pointer: *mut *mut u8,
        length: *mut usize,
        capacity: *mut usize,
    );
    fn bun_zstd__StreamingDecoder__new(error: *mut u8) -> *mut c_void;
    fn bun_zstd__StreamingDecoder__is_inflating(decoder: *const c_void) -> bool;
    fn bun_zstd__StreamingDecoder__decompress(
        decoder: *mut c_void,
        input: *const u8,
        input_len: usize,
        pointer: *mut *mut u8,
        length: *mut usize,
        capacity: *mut usize,
        max_output: usize,
        is_done: bool,
        consumed: *mut usize,
    ) -> u8;
    fn bun_zstd__StreamingDecoder__drop(decoder: *mut c_void);
}

/// The parts of a `Vec<u8>` on their way to CeeSafe or back: `(pointer, length, capacity)`.
type Parts = (*mut u8, usize, usize);

/// No vector: what CeeSafe's `Vector.fromForeign` takes for one that never allocated.
const NO_PARTS: Parts = (core::ptr::null_mut(), 0, 0);

/// Takes `out` apart for CeeSafe to adopt, leaving it empty. A `Vec` that never allocated has a dangling pointer
/// and capacity 0, which CeeSafe reads as no block.
fn give_up(out: &mut Vec<u8>) -> Parts {
    let mut taken = core::mem::ManuallyDrop::new(core::mem::take(out));
    (taken.as_mut_ptr(), taken.len(), taken.capacity())
}

/// The `Vec<u8>` CeeSafe gave with `Vector.intoForeign`.
///
/// # Safety
/// `parts` are what CeeSafe wrote: capacity 0, or a block of the `#[global_allocator]` of `capacity` bytes whose
/// first `length` are initialised, which this side now owns.
unsafe fn adopt(parts: Parts) -> Vec<u8> {
    let (pointer, length, capacity) = parts;
    if capacity == 0 {
        return Vec::new();
    }
    // SAFETY: the caller's contract. The block is CeeSafe's, not one Rust allocated with this layout: the allocator
    // (mimalloc, or the C library's under ASAN) frees by address and reallocates for any layout aligned to 16 or less.
    unsafe { Vec::from_raw_parts(pointer, length, capacity) }
}

pub enum Result {
    Success(usize),
    // Always assigned from ZSTD_getErrorName (static C string).
    Err(&'static ZStr),
}

impl Result {
    /// What the exports return: the size with `error` null, or the error's name.
    ///
    /// # Safety
    /// `error` is null or the static NUL-terminated string `ZSTD_getErrorName` returned.
    unsafe fn from_export(size: usize, error: *const c_char) -> Result {
        if error.is_null() {
            return Result::Success(size);
        }
        // SAFETY: the caller's contract.
        Result::Err(unsafe { ZStr::from_c_ptr(error) })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum ZstdError {
    /// The output, or the decoder state the frame's window size dictates, could not be allocated.
    OutOfMemory,
    InvalidZstdData,
    DecompressionFailed,
    ZstdFailedToCreateInstance,
    ZstdDecompressionError,
    ShortRead,
}

bun_core::impl_tag_error!(ZstdError);

impl ZstdError {
    /// The error for a non-zero code from an export (`errorCode` in ceebun/bun_zstd/src/exports.cee).
    fn from_code(code: u8) -> ZstdError {
        match code {
            1 => ZstdError::OutOfMemory,
            2 => ZstdError::InvalidZstdData,
            3 => ZstdError::DecompressionFailed,
            4 => ZstdError::ZstdFailedToCreateInstance,
            5 => ZstdError::ZstdDecompressionError,
            6 => ZstdError::ShortRead,
            _ => unreachable!("invalid ZstdError from CeeSafe"),
        }
    }
}

/// ZSTD_compress() :
///  Compresses `src` content as a single zstd compressed frame into already allocated `dst`.
///  NOTE: Providing `dstCapacity >= ZSTD_compressBound(srcSize)` guarantees that zstd will have
///        enough space to successfully compress the data.
///  @return : compressed size written into `dst` (<= `dstCapacity),
///            or an error code if it fails (which can be tested using ZSTD_isError()). */
pub fn compress(dest: &mut [u8], src: &[u8], level: Option<i32>) -> Result {
    let mut error: *const c_char = core::ptr::null();
    // SAFETY: the slices are live for the call and the export keeps neither; it writes `error` once.
    unsafe {
        let size = bun_zstd__compress(
            dest.as_mut_ptr(),
            dest.len(),
            src.as_ptr(),
            src.len(),
            level.is_some(),
            level.unwrap_or(0),
            &raw mut error,
        );
        Result::from_export(size, error)
    }
}

pub fn compress_bound(src_size: usize) -> usize {
    bun_zstd__compress_bound(src_size)
}

/// [`compress`] into `out`'s spare capacity (append mode). On success
/// `out.len()` is advanced by the compressed size. Callers must ensure
/// `out.spare_capacity_mut().len() >= compress_bound(src.len())` to
/// guarantee success.
pub fn compress_append(out: &mut Vec<u8>, src: &[u8], level: Option<i32>) -> Result {
    let (mut pointer, mut length, mut capacity) = give_up(out);
    let mut error: *const c_char = core::ptr::null();
    // SAFETY: CeeSafe takes `out`'s block and gives it back; `src` is live for the call; `error` is written once.
    unsafe {
        let size = bun_zstd__compress_append(
            &raw mut pointer,
            &raw mut length,
            &raw mut capacity,
            src.as_ptr(),
            src.len(),
            level.is_some(),
            level.unwrap_or(0),
            &raw mut error,
        );
        *out = adopt((pointer, length, capacity));
        Result::from_export(size, error)
    }
}

/// `ZSTD_isError` — true when a `size_t` return value (including from
/// [`compress_bound`], see zstd.h: "ZSTD_compressBound() itself can fail, if
/// `srcSize >= ZSTD_MAX_INPUT_SIZE`") is an error code rather than a size.
/// Error codes are near `usize::MAX`, so treating one as a size requests an
/// absurd allocation.
pub fn is_error(code: usize) -> bool {
    bun_zstd__is_error(code)
}

/// Decompress data, automatically allocating the output buffer.
/// Returns owned slice that must be freed by the caller.
/// Handles both frames with known and unknown content sizes.
/// For safety, if the reported decompressed size exceeds 16MB, streaming decompression is used instead.
/// Output allocations fail with [`ZstdError::OutOfMemory`] instead of aborting.
pub fn decompress_alloc(src: &[u8]) -> core::result::Result<Vec<u8>, ZstdError> {
    let (mut pointer, mut length, mut capacity) = NO_PARTS;
    // SAFETY: `src` is live for the call; on 0 the parts are the output CeeSafe gave.
    unsafe {
        match bun_zstd__decompress_alloc(
            src.as_ptr(),
            src.len(),
            &raw mut pointer,
            &raw mut length,
            &raw mut capacity,
        ) {
            0 => Ok(adopt((pointer, length, capacity))),
            code => Err(ZstdError::from_code(code)),
        }
    }
}

/// Decompresses one frame that was compressed with `prefix` as its raw content dictionary, which is
/// referenced and not copied. The size has to be in the header of the frame, and at most 16 MB.
pub fn decompress_alloc_with_prefix(
    src: &[u8],
    prefix: &[u8],
) -> core::result::Result<Vec<u8>, ZstdError> {
    let (mut pointer, mut length, mut capacity) = NO_PARTS;
    // SAFETY: `src` and `prefix` are live for the call; on 0 the parts are the output CeeSafe gave.
    unsafe {
        match bun_zstd__decompress_alloc_with_prefix(
            src.as_ptr(),
            src.len(),
            prefix.as_ptr(),
            prefix.len(),
            &raw mut pointer,
            &raw mut length,
            &raw mut capacity,
        ) {
            0 => Ok(adopt((pointer, length, capacity))),
            code => Err(ZstdError::from_code(code)),
        }
    }
}

pub fn get_decompressed_size(src: &[u8]) -> usize {
    // SAFETY: src is valid for src.len() bytes.
    unsafe { bun_zstd__get_decompressed_size(src.as_ptr(), src.len()) }
}

pub use bun_core::compress::State;

// ──────────────────────────────────────────────────────────────────────────
// StreamingDecoder
// ──────────────────────────────────────────────────────────────────────────

/// Streaming zstd decoder that owns only the `ZSTD_DStream`: input and output
/// are passed to [`decompress`](Self::decompress) per call, so callers can
/// hold the decoder across multiple body chunks without lifetime erasure.
///
/// The decoder is CeeSafe's, in a box this holds by its address.
pub struct StreamingDecoder {
    decoder: core::ptr::NonNull<c_void>,
}

impl StreamingDecoder {
    pub fn new() -> core::result::Result<Self, ZstdError> {
        let mut error: u8 = 0;
        // SAFETY: `error` is a live local, written when the result is null.
        let decoder = unsafe { bun_zstd__StreamingDecoder__new(&raw mut error) };
        match core::ptr::NonNull::new(decoder) {
            Some(decoder) => Ok(Self { decoder }),
            None => Err(ZstdError::from_code(error)),
        }
    }

    #[inline]
    pub fn is_inflating(&self) -> bool {
        // SAFETY: the decoder is live until `drop`.
        unsafe { bun_zstd__StreamingDecoder__is_inflating(self.decoder.as_ptr()) }
    }

    /// Append decompressed bytes to `out` (growing in 4096-byte steps) until `input` is
    /// consumed or `out.len()` reaches `max_output`. Returns the input bytes consumed.
    /// Returns `ShortRead` when more input is required and `is_done` is false.
    pub fn decompress(
        &mut self,
        input: &[u8],
        out: &mut Vec<u8>,
        max_output: usize,
        is_done: bool,
    ) -> core::result::Result<usize, ZstdError> {
        let (mut pointer, mut length, mut capacity) = give_up(out);
        let mut consumed = 0usize;
        // SAFETY: the decoder is live and `&mut self` is exclusive; `input` is live for the call; CeeSafe takes
        // `out`'s block and gives one back on every path; `consumed` is written on 0.
        unsafe {
            let code = bun_zstd__StreamingDecoder__decompress(
                self.decoder.as_ptr(),
                input.as_ptr(),
                input.len(),
                &raw mut pointer,
                &raw mut length,
                &raw mut capacity,
                max_output,
                is_done,
                &raw mut consumed,
            );
            *out = adopt((pointer, length, capacity));
            match code {
                0 => Ok(consumed),
                code => Err(ZstdError::from_code(code)),
            }
        }
    }
}

impl Drop for StreamingDecoder {
    fn drop(&mut self) {
        // SAFETY: the decoder came from `bun_zstd__StreamingDecoder__new`; dropped once here.
        unsafe { bun_zstd__StreamingDecoder__drop(self.decoder.as_ptr()) };
    }
}

// ── compressed embedded assets ────────────────────────────────────────────

/// The inflated bytes CeeSafe gave, boxed; `nul` adds a trailing NUL byte.
fn inflate(compressed: &'static [u8], nul: bool) -> Box<[u8]> {
    let (mut pointer, mut length, mut capacity) = NO_PARTS;
    // SAFETY: `compressed` is live for the call; the parts are the vector CeeSafe gave (it panics on a bad frame).
    unsafe {
        bun_zstd__inflate_embedded(
            compressed.as_ptr(),
            compressed.len(),
            nul,
            &raw mut pointer,
            &raw mut length,
            &raw mut capacity,
        );
        adopt((pointer, length, capacity)).into_boxed_slice()
    }
}

/// Cold, shared body of [`embed_compressed!`]'s release arm.
#[cold]
#[inline(never)]
pub fn inflate_embedded(compressed: &'static [u8]) -> Box<[u8]> {
    inflate(compressed, false)
}

/// [`inflate_embedded`] plus a trailing NUL byte.
#[cold]
#[inline(never)]
pub fn inflate_embedded_nul(compressed: &'static [u8]) -> Box<[u8]> {
    inflate(compressed, true)
}

/// Embed an asset zstd-compressed and inflate it on first use. Only for bytes
/// Bun never executes or parses itself: the shell completion scripts and the
/// JS/CSS bundles that are shipped to a browser (dev-server client runtime,
/// error overlay/page). Anything that runs inside Bun stays uncompressed.
///
/// The exception is text that only `bun lint`, `bun fmt` and `bun check`
/// read. It may be compressed because it isn't on the path of running a program.
///
/// Release (`bun_codegen_embed`) builds include the `.zst` twin that
/// `scripts/build/codegen.ts` (`emitCompressedEmbeds`) writes to
/// `<codegen>/compressed/<name>.zst`; other builds evaluate `$fallback`.
#[macro_export]
macro_rules! embed_compressed {
    // A codegen output (`<codegen>/$sub`); debug builds read it at runtime.
    (codegen $sub:literal) => {
        $crate::embed_compressed!(
            ("codegen/", $sub),
            ::bun_core::runtime_embed_file!(Codegen, $sub).as_bytes()
        )
    };
    // Same, with a trailing NUL included in the slice (for C-string consumers).
    (codegen_nul $sub:literal) => {{
        #[allow(unexpected_cfgs)]
        let __bytes: &'static [u8] = {
            #[cfg(bun_codegen_embed)]
            {
                static __INFLATED: ::bun_core::Once<::std::boxed::Box<[u8]>> = ::bun_core::Once::new();
                __INFLATED
                    .get_or_init(|| {
                        $crate::inflate_embedded_nul(::core::include_bytes!(::core::concat!(
                            ::core::env!("BUN_CODEGEN_DIR"),
                            "/compressed/codegen/",
                            $sub,
                            ".zst"
                        )))
                    })
            }
            #[cfg(not(bun_codegen_embed))]
            {
                static __COPY: ::std::sync::OnceLock<::std::boxed::Box<[u8]>> = ::std::sync::OnceLock::new();
                &__COPY.get_or_init(|| {
                    let text = ::bun_core::runtime_embed_file!(Codegen, $sub).as_bytes();
                    let mut copy = ::std::vec::Vec::with_capacity(text.len() + 1);
                    copy.extend_from_slice(text);
                    copy.push(0);
                    copy.into_boxed_slice()
                })[..]
            }
        };
        __bytes
    }};
    // A file under `src/`; debug builds read it from the source tree at runtime.
    (src $sub:literal) => {
        $crate::embed_compressed!(("src/", $sub), ::bun_core::runtime_embed_file!(Src, $sub).as_bytes())
    };
    ($name:literal, $fallback:expr) => {
        $crate::embed_compressed!(($name), $fallback)
    };
    (($($name:literal),+), $fallback:expr) => {{
        #[allow(unexpected_cfgs)]
        let __bytes: &'static [u8] = {
            #[cfg(bun_codegen_embed)]
            {
                static __INFLATED: ::bun_core::Once<::std::boxed::Box<[u8]>> = ::bun_core::Once::new();
                __INFLATED
                    .get_or_init(|| {
                        $crate::inflate_embedded(::core::include_bytes!(::core::concat!(
                            ::core::env!("BUN_CODEGEN_DIR"),
                            "/compressed/",
                            $($name,)+
                            ".zst"
                        )))
                    })
            }
            #[cfg(not(bun_codegen_embed))]
            {
                $fallback
            }
        };
        __bytes
    }};
}
