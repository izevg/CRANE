// i18n: не мигрирован
import { Link } from "~/shared/ui/primitives";
import logoLight from "~brand/crane-logo.svg?url";
import logoDark from "~brand/crane-logo-dark.svg?url";

const LABEL = "CRANE, Реестр артефактов: на главную";
const SUBTITLE = "Реестр артефактов";

type Props = { href: string; collapsed: boolean };

/**
 * Единственный логотип на странице. Файл логотипа широкий (знак + слово): в свёрнутом сайдбаре
 * показывается только знак (левая четверть), остальное обрезается. Темы переключаются CSS-ом без JS.
 */
export function Logo({ href, collapsed }: Props) {
  return (
    <Link
      href={href}
      aria-label={LABEL}
      className="flex min-w-0 items-center gap-3 rounded-md text-sidebar-foreground"
    >
      <span className={collapsed ? "block size-8 overflow-hidden" : "block h-7 shrink-0"}>
        <img src={logoLight} alt="" className="block h-full max-w-none dark:hidden" />
        <img src={logoDark} alt="" className="hidden h-full max-w-none dark:block" />
      </span>
      {collapsed ? null : (
        <span className="truncate text-caption text-muted-foreground">{SUBTITLE}</span>
      )}
    </Link>
  );
}
