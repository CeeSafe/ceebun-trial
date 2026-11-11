// CeeBun: the C side of embedding CeeSafe in Bun (ADR 0088, stages A and B).
//
// - `ceebun_init` starts the CeeSafe runtime once, from `bin_entry::main` before any thread can call CeeSafe, with
//   Bun's arguments (the runtime copies them) and a panic hook that reports through Bun's crash handler, with
//   CeeSafe's reason, message, file, line and column.
// - The allocator CeeSafe uses is the one Rust's `#[global_allocator]` is (`src/runtime/bin_entry/mod.rs`): mimalloc,
//   or the C library's under ASAN (CEEBUN_MIMALLOC, from scripts/build/ceesafe.ts, is set exactly when `bun_asan` is
//   not). The two sides then share one heap. Their blocks are still not interchangeable in general (ADR 0088 §2.5):
//   each side frees what it allocated through its own code. The exception is a `Vec<u8>`'s buffer (CeeSafe ADR 0090):
//   `bun_zstd` takes Rust's with `Vector.fromForeign` and gives one with `Vector.intoForeign`, so Rust frees and
//   grows blocks CeeSafe allocated, and CeeSafe Rust's. That holds because both allocators free by address alone
//   and reallocate a block whatever layout it was allocated with, and because `ceebun_realloc` returns a 16-aligned
//   block for the less-aligned one a small `Vec<u8>` has.
// - Debug builds (CEEBUN_READ_ENVIRONMENT, from scripts/build/ceesafe.ts) also read CEESAFE_ALLOC_REPORT and
//   CEESAFE_FAIL_ALLOC_AT, for allocation reports and failure sweeps inside Bun. A release `bun` does not: every
//   child process inherits the environment, and a stray CEESAFE_FAIL_ALLOC_AT would fail allocations in every `bun`
//   a test or a script spawns.
// - The exports are re-declared below as the Rust shims declare them (`src/csrf/lib.rs`, `src/zstd/lib.rs`). Next to
//   the declarations ceesafec generates (`ceesafe_exports.h`), a CeeSafe export whose C type changes from this copy (parameter count,
//   return type, constness, `bool`, an integer type distinct on this target) is a compile error. It does not see a
//   change made to the Rust shim alone, a reordering of parameters of one type, or enum values: the Rust shim and
//   this copy are kept in step by hand, until `--emit rust` (ADR 0088 stage C).

#include "ceesafe/embed.h"
#include "ceesafe_exports.h"

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(CEEBUN_MIMALLOC)
#include <mimalloc.h>
#endif

// ─── The exports as the Rust shims declare them ───

// src/csrf/lib.rs: `algorithm` is `Algorithm as u8`, `encoding` is `TokenFormat` (`#[repr(u8)]`).
size_t bun_csrf__generate(const uint8_t *secret, size_t secret_len, const uint8_t *session_id, size_t session_id_len,
                          uint64_t expires_in_ms, uint8_t algorithm, uint8_t *out_buffer);
bool bun_csrf__verify(const uint8_t *token, size_t token_len, const uint8_t *secret, size_t secret_len,
                      const uint8_t *session_id, size_t session_id_len, uint64_t max_age_ms, uint8_t encoding,
                      uint8_t algorithm);

// src/zstd/lib.rs: `has_level` and `level` are an `Option<i32>`; `pointer`, `length` and `capacity` are the parts of
// a `Vec<u8>` that changes sides; a `uint8_t` result is 0 or a `ZstdError`.
size_t bun_zstd__compress(uint8_t *dest, size_t dest_len, const uint8_t *src, size_t src_len, bool has_level,
                          int32_t level, const char **error);
size_t bun_zstd__compress_bound(size_t src_size);
size_t bun_zstd__compress_append(uint8_t **pointer, size_t *length, size_t *capacity, const uint8_t *src,
                                 size_t src_len, bool has_level, int32_t level, const char **error);
bool bun_zstd__is_error(size_t code);
size_t bun_zstd__get_decompressed_size(const uint8_t *src, size_t src_len);
uint8_t bun_zstd__decompress_alloc(const uint8_t *src, size_t src_len, uint8_t **pointer, size_t *length,
                                   size_t *capacity);
uint8_t bun_zstd__decompress_alloc_with_prefix(const uint8_t *src, size_t src_len, const uint8_t *prefix,
                                               size_t prefix_len, uint8_t **pointer, size_t *length,
                                               size_t *capacity);
void bun_zstd__inflate_embedded(const uint8_t *compressed, size_t compressed_len, bool nul, uint8_t **pointer,
                                size_t *length, size_t *capacity);
void *bun_zstd__StreamingDecoder__new(uint8_t *error);
bool bun_zstd__StreamingDecoder__is_inflating(const void *decoder);
uint8_t bun_zstd__StreamingDecoder__decompress(void *decoder, const uint8_t *input, size_t input_len,
                                               uint8_t **pointer, size_t *length, size_t *capacity,
                                               size_t max_output, bool is_done, size_t *consumed);
void bun_zstd__StreamingDecoder__drop(void *decoder);

// ─── The panic hook ───

// src/crash_handler/lib.rs: reports `CrashReason::Panic(message)` with a stack trace and does not return
// (`extern "C" fn … -> !`). It serialises threads that crash at once itself (PANIC_STAGE, PANIC_MUTEX), as embed.h
// asks of a hook.
_Noreturn void Bun__crashHandler(const uint8_t *message_ptr, size_t message_len);

// The report is built on the panicking thread's stack: the hook must not call CeeSafe, and need not allocate.
typedef struct report {
    char text[1024];
    size_t length;
} report;

static void report_bytes(report *r, const char *bytes, size_t length) {
    size_t room = sizeof r->text - r->length;
    if (length > room) length = room;
    memcpy(r->text + r->length, bytes, length);
    r->length += length;
}

static void report_text(report *r, const char *text) {
    report_bytes(r, text, strlen(text));
}

static void report_number(report *r, uint32_t value) {
    char digits[16];
    int length = snprintf(digits, sizeof digits, "%u", (unsigned)value);
    if (length > 0) report_bytes(r, digits, (size_t)length);
}

static void ceebun_panic_hook(void *context, const ceesafe_panic_info *info) {
    (void)context;
    report r;
    r.length = 0;
    // Bun prints "panic(<thread>): " before it.
    report_text(&r, "CeeSafe: ");
    report_text(&r, info->reason != NULL ? info->reason : "panic");
    if (info->message != NULL) {
        report_text(&r, ": ");
        // As bytes: a CeeSafe message is not zero-terminated and may hold a zero.
        report_bytes(&r, (const char *)info->message, info->message_length);
    }
    if (info->file != NULL) {
        // The path is the package's (src/…): ceesafec runs in the package directory (scripts/build/ceesafe.ts).
        report_text(&r, " at ");
        report_text(&r, info->file);
        report_text(&r, ":");
        report_number(&r, info->line);
        report_text(&r, ":");
        report_number(&r, info->column);
    } else {
        report_text(&r, " (in the CeeSafe runtime)");
    }
    Bun__crashHandler((const uint8_t *)r.text, r.length);
}

// ─── The allocator ───

#if defined(CEEBUN_MIMALLOC)
// CeeSafe asks for blocks aligned to max(16, alignof(max_align_t)) (embed.h); mimalloc's plain `mi_malloc` gives a
// small block only its size's alignment (8 for an 8-byte block), so the aligned forms, as `mi_malloc_auto_align`
// uses for Rust above 16. `mi_free` frees an aligned block as it frees any other.
#define CEEBUN_HEAP_ALIGN 16u
_Static_assert(_Alignof(max_align_t) <= CEEBUN_HEAP_ALIGN, "CeeSafe's heap alignment is max_align_t's");

// mimalloc's functions are thread-safe and keep a heap per thread: the context is unused.
static void *ceebun_alloc(void *context, size_t size) {
    (void)context;
    return mi_malloc_aligned(size, CEEBUN_HEAP_ALIGN);
}

static void *ceebun_alloc_zeroed(void *context, size_t size) {
    (void)context;
    return mi_zalloc_aligned(size, CEEBUN_HEAP_ALIGN);
}

// On failure the block is kept, as realloc's contract asks.
static void *ceebun_realloc(void *context, void *block, size_t size) {
    (void)context;
    return mi_realloc_aligned(block, size, CEEBUN_HEAP_ALIGN);
}

static void ceebun_free(void *context, void *block) {
    (void)context;
    mi_free(block);
}

static const ceesafe_allocator ceebun_allocator = {ceebun_alloc, ceebun_alloc_zeroed, ceebun_realloc, ceebun_free,
                                                   NULL};
#endif

// ─── Start and shutdown ───

#if defined(CEEBUN_READ_ENVIRONMENT)
static void ceebun_shutdown(void) {
    // Prints the allocation report when CEESAFE_ALLOC_REPORT asked for one; frees nothing.
    (void)ceesafe_shutdown();
}
#endif

int32_t ceebun_init(int argc, const char *const *argv) {
    ceesafe_config config = {0};
    config.size = sizeof config;
#if defined(CEEBUN_READ_ENVIRONMENT)
    config.flags = CEESAFE_INIT_ENVIRONMENT;
#endif
    config.argc = argc;
    config.argv = argv;
    config.panic_hook = ceebun_panic_hook;
#if defined(CEEBUN_MIMALLOC)
    // Installed before CeeSafe allocates anything: argv's copy is the first block. Under ASAN no allocator is given,
    // so CeeSafe uses malloc and ASAN sees every block, as it sees Rust's.
    config.allocator = &ceebun_allocator;
#endif
    int32_t status = ceesafe_init(&config);
#if defined(CEEBUN_READ_ENVIRONMENT)
    if (status == 0) {
        // The report, on the exits that run these handlers: Bun's `Global::exit` is `exit` on macOS and in ASAN
        // builds, `quick_exit` on Linux and FreeBSD. Not after `_exit`, a signal or a crash.
        atexit(ceebun_shutdown);
        at_quick_exit(ceebun_shutdown);
    }
#endif
    return status;
}
