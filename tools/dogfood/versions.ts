// Разбор вывода `cargo --version` и аргументов запуска.

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

export type Which = "latest" | "previous";

export interface Args {
  which: Which[];
  /** Полный dogfood (публикация CRANE и сборка из реестра) на клиенте latest, после остальных сценариев. */
  full: boolean;
}

/** `[--cargo latest|previous|список] [--full]`; по умолчанию latest без полного сценария. */
export function parseArgs(argv: string[]): Args {
  const full = argv.includes("--full");
  const flag = argv.indexOf("--cargo");
  const which: Which[] = [];
  if (flag === -1) {
    which.push("latest");
  } else {
    const values = (argv[flag + 1] ?? "").split(",");
    const known = values.filter((v): v is Which => v === "latest" || v === "previous");
    if (known.length === 0 || known.length !== values.length) {
      throw new Error("--cargo принимает latest, previous или их список через запятую");
    }
    which.push(...new Set(known));
  }
  if (full && !which.includes("latest")) {
    throw new Error("--full идёт на клиенте latest: добавьте latest в --cargo");
  }
  return { which, full };
}
