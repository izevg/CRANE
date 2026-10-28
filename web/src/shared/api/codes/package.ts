import type { Format, RepositoryKind } from "../generated/schemas";

// i18n-fixture: названия форматов — имена собственные и не переводятся
export const FORMAT = {
  npm: "npm",
  pypi: "PyPI",
  maven: "Maven",
  nuget: "NuGet",
  cargo: "Cargo",
} as const satisfies Record<Format, string>;

// i18n-fixture: «hosted» и «proxy» — термины продукта, одинаковые в обоих языках
export const REPOSITORY_KIND = {
  hosted: "hosted",
  proxy: "proxy",
} as const satisfies Record<RepositoryKind, string>;
