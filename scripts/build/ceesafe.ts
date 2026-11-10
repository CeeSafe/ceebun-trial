/**
 * CeeBun: the CeeSafe packages under `ceebun/` that replace Rust crates (ADR 0088, stage A).
 *
 * One umbrella root package (`kind = "object"`) depends on every swapped crate, so they share one copy of each
 * package and its statics. `ceesafec build --emit lib` compiles it to one static archive — the generated code and
 * the CeeSafe runtime, one member per C file — which the link takes beside the Rust rlibs. With LTO the members are
 * ThinLTO bitcode, so CeeSafe joins the link's cross-language optimisation. Beside the archive ceesafec writes the
 * host's headers: `ceesafe_exports.h` (the exports) and `ceesafe/embed.h` (`ceesafe_init`, the panic hook).
 *
 * `ceebun/host/ceebun_host.c` is the C side of the embedding: `ceebun_init` (called by `bin_entry::main`) starts the
 * runtime with a panic hook that reports through Bun's crash handler and Rust's allocator (mimalloc, or the C
 * library's under ASAN; ADR 0088 stage B), and the file re-declares each export as the Rust shims declare it, so a
 * shim that drifts from the CeeSafe signature is a compile error.
 *
 * The compiler is `$CEESAFEC`, or `ceesafec` on the PATH.
 */

import { readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { cc } from "./compile.ts";
import type { Config } from "./config.ts";
import { assert } from "./error.ts";
import { computeDepFlags } from "./flags.ts";
import type { Ninja } from "./ninja.ts";
import { quote } from "./shell.ts";
import type { ResolvedDep } from "./source.ts";
import { toolIdentityFile } from "./tools.ts";

/** The umbrella root package: one archive per binary (ADR 0088 §2.1). */
const root = "bun_csrf";

function ceesafec(): string {
  const found = process.env.CEESAFEC ?? Bun.which("ceesafec");
  assert(found !== null && found !== undefined, "ceesafec not found", {
    hint: "Set CEESAFEC to the CeeSafe compiler (ceesafec 0.29.0 or later), or put it on the PATH.",
  });
  return found;
}

/**
 * Every `.cee` file and manifest under `dir`: a change to any of them rebuilds the archive. Listed at configure
 * time, so a file or package added later is seen after the next configure.
 */
function packageInputs(dir: string): string[] {
  const inputs: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "out") inputs.push(...packageInputs(path));
    } else if (entry.name.endsWith(".cee") || entry.name === "ceesafe.toml" || entry.name === "ceesafe.lock") {
      inputs.push(path);
    }
  }
  return inputs;
}

export function registerCeesafeRules(n: Ninja, cfg: Config): void {
  const q = (p: string) => quote(p, false);
  n.rule("ceesafe_lib", {
    command: `cd $pkgdir && ${q(ceesafec())} build --emit lib --offline --cc ${q(cfg.cc)} --ar ${q(cfg.ar)} $cflags -o $libpath`,
    description: "ceesafec $out",
  });
}

/**
 * The archive's edge and the host file's; returns what the link takes. `mimalloc` is the resolved dependency: the host
 * file includes its header to give CeeSafe the allocator Rust's `#[global_allocator]` is.
 */
export function emitCeesafe(n: Ninja, cfg: Config, mimalloc: ResolvedDep): string[] {
  const ceebun = resolve(cfg.cwd, "ceebun");
  const outDir = resolve(cfg.buildDir, "ceebun");
  const archive = join(outDir, `lib${root}.a`);
  const exportsHeader = join(outDir, "ceesafe_exports.h");
  const embedHeader = join(outDir, "ceesafe", "embed.h");

  // ceesafec drives the C compiler with GCC-style flags, runs `$ar rcs`, and the rule starts with `cd`: Windows
  // (clang-cl, llvm-lib, ninja without a shell) is ADR 0088 stage E.
  assert(cfg.unix, "CeeBun builds only for Unix targets", {
    hint: "Windows needs ceesafec's clang-cl and llvm-lib support (ADR 0088 stage E).",
  });

  // Bun's flags for C that is not Bun's own (flags.ts `globalFlags`), the same as the host file's: the target and
  // sysroot, -mcpu, frame pointers for the crash handler's traces, -flto=thin in LTO builds (so the members are
  // bitcode and join Bun's ThinLTO, ADR 0088 §1.11), ASAN in ASAN builds, -O0 and -g in debug builds. Given after
  // ceesafec's own flags, they override them.
  const cflags = computeDepFlags(cfg).cflags;

  n.build({
    outputs: [archive],
    implicitOutputs: [exportsHeader, embedHeader],
    rule: "ceesafe_lib",
    inputs: [],
    // A new ceesafec or a new clang rebuilds the archive, as a new clang recompiles Bun's C (compile.ts).
    implicitInputs: [...packageInputs(ceebun), resolve(ceesafec()), toolIdentityFile(cfg, "cc")],
    vars: {
      pkgdir: quote(join(ceebun, root), false),
      // ninja's `$out` is relative to the build directory, and the command runs in the package's.
      libpath: quote(archive, false),
      cflags: cflags.map(f => `--cflag ${quote(f, false)}`).join(" "),
    },
  });

  // The host side, with the same flags, against the headers written beside the archive. Debug builds read
  // CEESAFE_ALLOC_REPORT and CEESAFE_FAIL_ALLOC_AT; a release `bun` reads no variable outside BUN_*. CEEBUN_MIMALLOC
  // follows `bin_entry`'s `#[global_allocator]`: mimalloc unless `bun_asan`, which is set exactly when cfg.asan
  // (rust.ts). mimalloc's outputs signal that its headers are ready, as for Bun's own C (bun.ts).
  const host = cc(n, cfg, join(ceebun, "host", "ceebun_host.c"), {
    flags: [
      ...cflags,
      "-I",
      outDir,
      ...mimalloc.includes.flatMap(dir => ["-I", dir]),
      ...(cfg.debug ? ["-DCEEBUN_READ_ENVIRONMENT=1"] : []),
      ...(cfg.asan ? [] : ["-DCEEBUN_MIMALLOC=1"]),
    ],
    implicitInputs: [exportsHeader, embedHeader, ...mimalloc.outputs],
  });
  return [host, archive];
}
