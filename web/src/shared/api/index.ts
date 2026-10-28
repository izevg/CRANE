export { PRIORITY } from "./codes/finding";
export { FORMAT, REPOSITORY_KIND } from "./codes/package";
export { BLOCK_REASON_KIND, LIFECYCLE_STATE, VERSION_STATE } from "./codes/version";
// Страницы берут `get<Op>QueryOptions` / `get<Op>MutationOptions` и не используют хуки `use<Op>`: один и тот же
// объект идёт в loader маршрута и в useQuery. Сырые request-функции и заглушки наружу не выходят.
export {
  getGetMeQueryOptions,
  getGetStatusQueryOptions,
  getGetStatusSuspenseQueryOptions,
} from "./generated/admin.gen";
export type * from "./generated/schemas";
export { halHref } from "./hal/halHref";
export { ApiProblem } from "./http/problem";
export { problemViewOf, SIGN_IN_AVAILABLE_IN } from "./present/problemView";
export type { ApiOverrides } from "./stubs/override/override";
export { DEMO } from "./stubs/table/demo";
