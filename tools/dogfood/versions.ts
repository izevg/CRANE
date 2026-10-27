// Разбор вывода `cargo --version`.

const CARGO_OUTPUT = /^cargo (1\.(\d+)\.\d+)\b/;

export interface CargoVersion {
  /** Полная версия 1.M.p: из неё строится user agent `cargo/1.M.p`. */
  version: string;
  /** M: из него строятся тег образа rust:1.M-bookworm и предыдущая линия M-1. */
  minor: number;
}

/** "cargo 1.99.0 (abc 2026-10-01)" -> { version: "1.99.0", minor: 99 }. */
export function parseCargo(output: string): CargoVersion {
  const match = CARGO_OUTPUT.exec(output.trim());
  const version = match?.[1];
  const minor = Number(match?.[2]);
  if (version === undefined || !Number.isInteger(minor)) {
    throw new Error(`не разобрать вывод cargo --version: ${output.trim()}`);
  }
  return { version, minor };
}
