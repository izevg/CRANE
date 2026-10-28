import { msg } from "@lingui/core/macro";
import { parseLoginSearch } from "./search";

export const loginRoute = {
  staticData: { title: msg`Вход` },
  validateSearch: parseLoginSearch,
  load: () => import("../ui/LoginPage").then((module) => module.LoginPage),
};
