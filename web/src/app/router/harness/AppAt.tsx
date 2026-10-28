import { useQueryClient } from "@tanstack/react-query";
import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { useState } from "react";
import { createAppRouter } from "../tree/router";

/** Приложение на памяти по адресу `url`; клиент запросов даёт декоратор витрины (свежий на историю, retry: false). */
export function AppAt({ url }: { url: string }) {
  const queryClient = useQueryClient();
  const [router] = useState(() =>
    createAppRouter({ queryClient, history: createMemoryHistory({ initialEntries: [url] }) }),
  );
  return <RouterProvider router={router} />;
}
