import { execFileSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { join, relative, sep } from "node:path";

export interface Rules {
  scope: string[];
  limits: { files: number; dirs: number };
  ignoredDirs: string[];
  unlimitedDirs: string[];
  serviceFiles: string[];
  testFiles: string[];
  entryFiles: string[];
  rustLayers: Record<string, string[]>;
}

export interface CargoPackage {
  name: string;
  manifest_path: string;
  dependencies: { name: string; kind: string | null }[];
}

export interface CargoMetadata {
  packages: CargoPackage[];
}

type Matcher = (name: string) => boolean;

const globToRegExp = (glob: string): RegExp =>
  new RegExp(`^${glob.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*")}$`);

const matcher = (globs: string[]): Matcher => {
  const regs = globs.map(globToRegExp);
  return (name) => regs.some((r) => r.test(name));
};

interface Listing {
  files: string[];
  dirs: string[];
}

function list(dir: string, rules: Rules): Listing {
  const isService = matcher(rules.serviceFiles);
  const isTest = matcher(rules.testFiles);
  const listing: Listing = { files: [], dirs: [] };
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const { name } = entry;
    if (name.startsWith(".")) continue;
    if (entry.isDirectory()) {
      if (!rules.ignoredDirs.includes(name) && !rules.unlimitedDirs.includes(name)) {
        listing.dirs.push(name);
      }
    } else if (!isService(name) && !isTest(name)) {
      listing.files.push(name);
    }
  }
  return listing;
}

function violationsOf(rel: string, { files, dirs }: Listing, rules: Rules): string[] {
  const isEntry = matcher(rules.entryFiles);
  const mixing = files.filter((f) => !isEntry(f));
  const found: string[] = [];
  if (files.length > rules.limits.files) {
    found.push(`${rel}: ${files.length} файлов при лимите ${rules.limits.files}`);
  }
  if (dirs.length > rules.limits.dirs) {
    found.push(`${rel}: ${dirs.length} подпапок при лимите ${rules.limits.dirs}`);
  }
  if (dirs.length > 0 && mixing.length > 0) {
    found.push(`${rel}: смешанное наполнение (файлы ${mixing.join(", ")} и подпапки)`);
  }
  return found;
}

/** «Правило четырёх»: ≤ files файлов, ≤ dirs подпапок, без смешанного наполнения. */
export function checkStructure(root: string, rules: Rules): string[] {
  const walk = (dir: string): string[] => {
    const listing = list(dir, rules);
    const own = violationsOf(relative(root, dir) || ".", listing, rules);
    return listing.dirs.reduce((acc, d) => acc.concat(walk(join(dir, d))), own);
  };
  return rules.scope
    .map((scope) => join(root, scope))
    .filter((start) => existsSync(start))
    .flatMap(walk);
}

/** Слой крейта по пути: crates/core/<слой>/..., crates/<слой>/... */
export function layerOf(manifestPath: string, root: string): string {
  const parts = relative(join(root, "crates"), manifestPath).split(sep);
  return (parts[0] === "core" ? parts[1] : parts[0]) ?? "";
}

/** Направление зависимостей между крейтами (hexagonal + clean architecture). */
export function checkRustLayers(root: string, rules: Rules, metadata: CargoMetadata): string[] {
  const violations: string[] = [];
  const byName = new Map(metadata.packages.map((p) => [p.name, p]));
  for (const pkg of metadata.packages) {
    const layer = layerOf(pkg.manifest_path, root);
    const allowed = rules.rustLayers[layer];
    if (!allowed) {
      const known = Object.keys(rules.rustLayers).join(", ");
      violations.push(`${pkg.name}: неизвестный слой «${layer}» (допустимы ${known})`);
      continue;
    }
    for (const dep of pkg.dependencies) {
      const target = byName.get(dep.name);
      if (dep.kind === "dev" || !target) continue;
      const depLayer = layerOf(target.manifest_path, root);
      if (!allowed.includes(depLayer)) {
        violations.push(`${pkg.name} (${layer}) не может зависеть от ${dep.name} (${depLayer})`);
      }
    }
  }
  return violations;
}

export function cargoMetadata(root: string): CargoMetadata {
  if (!existsSync(join(root, "Cargo.toml"))) return { packages: [] };
  const out = execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  return JSON.parse(out) as CargoMetadata;
}
