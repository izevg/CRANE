import { Outlet } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { DEMO } from "~/shared/api";
import { DemoNotice } from "../demo/DemoNotice";

const MAIN_ID = "main-content";

/** Страница без шелла (архетип K): карточка по центру, ≤ 28rem. Единственный `<main>` документа. */
export function BareFrame({ children }: { children: ReactNode }) {
  return (
    <main
      id={MAIN_ID}
      tabIndex={-1}
      className="mx-auto flex min-h-svh w-full max-w-md flex-col justify-center px-4 py-6 outline-none"
    >
      {DEMO ? <DemoNotice /> : null}
      {children}
    </main>
  );
}

export function BareLayout() {
  return (
    <BareFrame>
      <Outlet />
    </BareFrame>
  );
}
