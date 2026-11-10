/**
 * CeeBun: the CeeSafe packages under `ceebun/` that replace Rust crates. Each root package (`kind = "object"`) is
 * compiled by `ceesafec build --emit obj` to one relocatable object — its dependency packages and the CeeSafe
 * runtime included — which the link takes beside the Rust rlibs. The Rust crate it replaces keeps its API as a shim
 * over the object's `@export` functions.
 *
 * The compiler is `$CEESAFEC`, or `ceesafec` on the PATH.
 */

import { readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import type { Config } from "./config.ts";
import { assert } from "./error.ts";
import type { Ninja } from "./ninja.ts";
import { quote } from "./shell.ts";

/** The root packages, each one object in the link. */
const roots = ["bun_csrf"];

function ceesafec(): string {
  const found = process.env.CEESAFEC ?? Bun.which("ceesafec");
  assert(found !== null && found !== undefined, "ceesafec not found", {
    hint: "Set CEESAFEC to the CeeSafe compiler (ceesafec 0.27.0 or later), or put it on the PATH.",
  });
  return found;
}

/** Every `.cee` file and manifest under `dir`: a change to any of them rebuilds the objects. */
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
  n.rule("ceesafe_obj", {
    command: `cd $pkgdir && ${q(ceesafec())} build --emit obj --offline --cc ${q(cfg.cc)} $cflags -o $objpath`,
    description: "ceesafec $out",
  });
}

/** Emit one `ceesafec build --emit obj` edge per root package; returns the objects for the link. */
export function emitCeesafe(n: Ninja, cfg: Config): string[] {
  const ceebun = resolve(cfg.cwd, "ceebun");
  const inputs = packageInputs(ceebun);
  const cflags: string[] = [];
  if (cfg.darwin && cfg.osxDeploymentTarget !== undefined) {
    cflags.push(`-mmacosx-version-min=${cfg.osxDeploymentTarget}`);
  }
  if (cfg.debug) cflags.push("-g");
  const objects: string[] = [];
  for (const root of roots) {
    const out = resolve(cfg.buildDir, "ceebun", `${root}.o`);
    n.build({
      outputs: [out],
      rule: "ceesafe_obj",
      inputs: [],
      implicitInputs: inputs,
      vars: {
        pkgdir: quote(join(ceebun, root), false),
        // ninja's `$out` is relative to the build directory, and the command runs in the package's.
        objpath: quote(out, false),
        cflags: cflags.map(f => `--cflag ${quote(f, false)}`).join(" "),
      },
    });
    objects.push(out);
  }
  return objects;
}
