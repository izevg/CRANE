// Чистые помощники выбора версий cargo.

const RELEASE = /^1\.(\d+)\.\d+$/;
const CARGO_OUTPUT = /^cargo (1\.\d+\.\d+)\b/;
const MINOR_LINE = /^(1\.\d+)\.\d+$/;

/** "1.99.0" -> "1.98": предыдущая минорная линия стабильного Rust. */
export function previousMinor(version: string): string {
  const match = RELEASE.exec(version);
  const minor = Number(match?.[1]);
  if (!Number.isInteger(minor) || minor < 1) {
    throw new Error(`не разобрать версию Rust: ${version}`);
  }
  return `1.${minor - 1}`;
}

/** "cargo 1.99.0 (abc123 2026-10-01)" -> "1.99.0". */
export function parseCargoVersion(output: string): string {
  const match = CARGO_OUTPUT.exec(output.trim());
  if (match?.[1] === undefined) {
    throw new Error(`не разобрать вывод cargo --version: ${output.trim()}`);
  }
  return match[1];
}

/** "1.99.0" -> "1.99". */
export function minorOf(version: string): string {
  const match = MINOR_LINE.exec(version);
  if (match?.[1] === undefined) {
    throw new Error(`не разобрать версию Rust: ${version}`);
  }
  return match[1];
}
