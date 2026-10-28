import type { ErrorComponentProps } from "@tanstack/react-router";
import { BareFrame } from "../layout/BareLayout";
import { RouteError } from "./RouteError";

/** Сбой до появления шелла (GET /me, чанк шелла): полностраничная карточка по центру, без сайдбара. */
export function ShellError(props: ErrorComponentProps) {
  return (
    <BareFrame>
      <div className="rounded-xl border border-border bg-card shadow-card">
        <RouteError {...props} scope="console" />
      </div>
    </BareFrame>
  );
}
