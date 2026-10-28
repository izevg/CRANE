import type { Me } from "../../generated/schemas";

// i18n-fixture: демо-данные заглушки — контент сервера, а не текст интерфейса
export function getMe(): Me {
  return {
    principal: { id: "usr_demo", kind: "human", display: "Иванова А." },
    roles: ["developer"],
    workspaces: [{ workspace: "billing", roles: ["maintainer"] }],
    sod: { mode: "warn", conflict: false },
    _links: {
      self: { href: "/me" },
      // Ссылки без маршрута в роутере сайдбар не показывает; `crane:status` — единственный раздел PR-2.
      "crane:status": { href: "/status" },
      "crane:search": { href: "/search" },
      "crane:my-tokens": { href: "/tokens" },
    },
  };
}
