import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type ReactNode, useState } from "react";

/**
 * Свежий клиент запросов на каждую историю: данные одной истории не видны другой. `retry: false`, иначе история
 * ошибки ждала бы повторов.
 */
export function ApiStory({ children }: { children: ReactNode }) {
  const [client] = useState(
    () => new QueryClient({ defaultOptions: { queries: { retry: false } } }),
  );
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
