import type { QueryClient } from "@tanstack/react-query";

/** Контекст роутера: страницы получают клиент запросов отсюда, а не импортом из app. */
export type RouterContext = { queryClient: QueryClient };
