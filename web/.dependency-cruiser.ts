import type { IConfiguration } from "dependency-cruiser";

/** Feature-Sliced Design: импорты только вниз по слоям, без перекрёстных слайсов, только через публичный API. */
const LAYERS: string[] = ["app", "pages", "widgets", "features", "entities", "shared"];
const below = (i: number): string[] => LAYERS.slice(0, i); // слои, которым нельзя импортироваться снизу

const layerRules = LAYERS.slice(1).map((layer, i) => ({
  name: `fsd-${layer}-no-upward-imports`,
  comment: `Слой ${layer} не импортирует вышележащие слои`,
  severity: "error",
  from: { path: `^src/${layer}/` },
  to: { path: `^src/(${below(i + 1).join("|")})/` },
}));

const config: IConfiguration = {
  forbidden: [
    ...layerRules,
    {
      name: "fsd-no-cross-slice-imports",
      comment: "Слайсы одного слоя не импортируют друг друга (кроме app и shared)",
      severity: "error",
      from: { path: "^src/(pages|widgets|features|entities)/([^/]+)/" },
      to: {
        path: "^src/$1/[^/]+/",
        pathNot: "^src/$1/$2/",
      },
    },
    {
      name: "fsd-public-api-only",
      comment: "Слайс доступен только через index.ts",
      severity: "error",
      from: { path: "^src/", pathNot: "^src/(pages|widgets|features|entities)/([^/]+)/" },
      to: {
        path: "^src/(pages|widgets|features|entities)/[^/]+/.+",
        pathNot: "^src/(pages|widgets|features|entities)/[^/]+/index\\.(ts|tsx)$",
      },
    },
    { name: "no-circular", severity: "error", from: {}, to: { circular: true } },
    {
      name: "not-to-dev-dep",
      severity: "error",
      from: { path: "^src/", pathNot: "\\.test\\.(ts|tsx)$" },
      to: { dependencyTypes: ["npm-dev"] },
    },
    {
      name: "no-orphans",
      severity: "error",
      from: { orphan: true, pathNot: ["\\.test\\.(ts|tsx)$", "^src/app/main\\.tsx$"] },
      to: {},
    },
  ],
  options: {
    doNotFollow: { path: "node_modules" },
    tsPreCompilationDeps: true,
    tsConfig: { fileName: "tsconfig.json" },
    enhancedResolveOptions: {
      exportsFields: ["exports"],
      conditionNames: ["import", "require", "node", "default"],
    },
  },
};

export default config;
