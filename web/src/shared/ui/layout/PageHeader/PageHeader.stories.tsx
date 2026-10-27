import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import { AutoRefreshToggle } from "~/shared/ui/actions";
import { PageHeader } from "~/shared/ui/layout";
import {
  Badge,
  Tabs,
  TabsList,
  TabsTrigger,
  ToggleGroup,
  ToggleGroupItem,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";

const meta = {
  title: "Раскладка/PageHeader",
  component: PageHeader,
  args: { title: "@acme/ui" },
} satisfies Meta<typeof PageHeader>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Факт, а не ссылка: «подпись → значение» одной строкой (patterns §1). */
function Chip({ label, value }: { label: string; value: string }) {
  return (
    <span className="inline-flex h-6 items-center gap-1 rounded-full bg-muted px-2.5 text-caption">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-semibold tabular-nums">{value}</span>
    </span>
  );
}

export const PackageCard: Story = {
  name: "Карточка пакета (S2)",
  args: {
    status: <StatusBadge tone="success" label="Активен" />,
    chips: <Badge variant="outline">npm</Badge>,
    caption: "Компоненты интерфейса CRANE и внутренних порталов",
    stats: (
      <>
        <Chip label="Версий" value="48" />
        <Chip label="Последняя" value="1.4.2" />
        <Chip label="Загрузок за 30 дн" value="12 340" />
        <Chip label="Потребителей" value="63" />
        <Chip label="Команда" value="UI-kit" />
      </>
    ),
    tabs: (
      <Tabs defaultValue="overview">
        <TabsList variant="line" aria-label="Разделы пакета">
          <TabsTrigger value="overview">Обзор</TabsTrigger>
          <TabsTrigger value="versions">Версии 48</TabsTrigger>
          <TabsTrigger value="readme">README</TabsTrigger>
          <TabsTrigger value="risks">Риски 1</TabsTrigger>
          <TabsTrigger value="consumers">Потребители</TabsTrigger>
          <TabsTrigger value="settings">Настройки</TabsTrigger>
        </TabsList>
      </Tabs>
    ),
    primary: { label: "Скопировать команду установки", run: { onSelect: fn() } },
    secondary: [{ label: "В избранное", run: { onSelect: fn() } }],
    menu: [
      { label: "Назначить замену…", run: { onSelect: fn() } },
      { label: "Запросить одобрение…", run: { onSelect: fn() } },
      { label: "Настройки", run: { href: "/packages/acme-ui/settings" } },
      { label: "Объявить устаревшим…", run: { onSelect: fn() }, destructive: true },
    ],
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const heading = canvas.getByRole("heading", { level: 1, name: "@acme/ui" });
    await expect(heading).toHaveAttribute("tabindex", "-1");
    await expect(
      canvas.getByRole("button", { name: "Скопировать команду установки" }),
    ).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Другие действия" })).toBeVisible();
  },
};

export const Dashboard: Story = {
  name: "Дашборд со слотом aside, без primary (S6)",
  args: {
    title: "Платформа",
    status: <StatusBadge tone="success" label="Режим: норма" />,
    caption: "Хранилища, кэш, upstream и фоновые задачи",
    aside: (
      <>
        <ToggleGroup aria-label="Окно времени" defaultValue={["24h"]} size="sm" variant="outline">
          <ToggleGroupItem value="1h">1 ч</ToggleGroupItem>
          <ToggleGroupItem value="6h">6 ч</ToggleGroupItem>
          <ToggleGroupItem value="24h">24 ч</ToggleGroupItem>
          <ToggleGroupItem value="7d">7 дн</ToggleGroupItem>
        </ToggleGroup>
        <AutoRefreshToggle paused={false} onPausedChange={fn()} />
      </>
    ),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: "Пауза автообновления" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "24 ч" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  },
};

export const NoActions: Story = {
  name: "Без действий — блока действий нет",
  args: {
    title: "Журнал действий",
    caption: "Неизменяемая запись событий, выгружается в SIEM",
  },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("button")).toBeNull();
    await expect(canvasElement.querySelector("[data-slot=action-bar]")).toBeNull();
  },
};
