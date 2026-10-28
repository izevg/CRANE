import { msg } from "@lingui/core/macro";
import { lazyPage } from "~/shared/lib/route";
import { parseLoginSearch } from "./search";

export const loginRoute = {
  staticData: { title: msg`Вход` },
  validateSearch: parseLoginSearch,
  component: lazyPage(() => import("../ui/LoginPage"), "LoginPage"),
};
