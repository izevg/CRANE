import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { type NotificationItem, NotificationsCenter } from "~/widgets/notifications-center";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const items: NotificationItem[] = [
  {
    id: "n1",
    category: "security",
    tone: "error",
    title: "P0 без действия: CVE-2026-31337 в @acme/ui 1.4.2",
    time: ago(12),
    caption: "@acme/ui · npm",
    href: "/findings/CVE-2026-31337",
    unread: true,
  },
  {
    id: "n2",
    category: "access",
    tone: "warning",
    title: "Токен crp_9f8e7d6c истекает через 3 дня",
    time: ago(95),
    caption: "ci-deploy",
    href: "/tokens/crp_9f8e7d6c",
    unread: true,
  },
  {
    id: "n3",
    category: "platform",
    tone: "warning",
    title: "Upstream npmjs недоступен",
    time: ago(240),
    caption: "Источник npmjs",
    href: "/platform/upstreams/npmjs",
    unread: true,
  },
  {
    id: "n4",
    category: "security",
    tone: "success",
    title: "lodash 4.17.21 выпущена из карантина",
    time: ago(60 * 26),
    caption: "lodash · npm",
    href: "/packages/lodash",
    unread: false,
  },
  {
    id: "n5",
    category: "platform",
    tone: "info",
    title: "Индексация репозитория npm-proxy завершена",
    time: ago(60 * 30),
    href: "/jobs/42",
    unread: false,
  },
];

const BELL = /^Уведомления/;
const SECURITY_TAB = /Безопасность/;
const ALL_TAB = /Все/;
const CVE_LINK = /Непрочитано.*CVE-2026-31337/;
const LODASH_LINK = /lodash 4.17.21/;
const UNREAD = /Непрочитано/;
const UPSTREAM_LINK = /Upstream npmjs/;

const markRead = fn();
const readAll = fn();

const SECURITY_EN = /Security/;

const meta = {
  title: "Шелл/Уведомления/Центр",
  component: NotificationsCenter,
  parameters: { layout: "centered" },
  args: {
    items,
    unread: 3,
    unreadByCategory: { security: 1, access: 1, platform: 1 },
    onCategoryChange: fn(),
    onMarkRead: markRead,
  },
} satisfies Meta<typeof NotificationsCenter>;

export default meta;
type Story = StoryObj<typeof meta>;

async function openCenter(canvasElement: HTMLElement) {
  await userEvent.click(within(canvasElement).getByRole("button", { name: BELL }));
  const body = within(document.body);
  // Ждём, пока Base UI перенесёт фокус в окно: до этого его служебные фокус-ловушки ещё в переходном состоянии, и axe видит их нестабильно.
  const popup = await body.findByRole("dialog", { name: "Уведомления" });
  await waitFor(() => expect(popup).toContainElement(document.activeElement as HTMLElement));
  return body;
}

/** Остаётся открытой в момент axe-проверки: четыре тона, прочитанные и непрочитанные строки. */
export const Open: Story = {
  name: "Открыт",
  args: { onReadAll: readAll },
  play: async ({ canvasElement }) => {
    const body = await openCenter(canvasElement);
    await expect(await body.findByRole("tab", { name: SECURITY_TAB })).toBeInTheDocument();
    await expect(body.getByRole("link", { name: CVE_LINK })).toHaveAttribute(
      "href",
      "/findings/CVE-2026-31337",
    );
    await expect(body.getByRole("link", { name: LODASH_LINK })).not.toHaveAccessibleName(UNREAD);
  },
};

export const MarkRead: Story = {
  name: "Переход по строке отмечает прочитанной",
  play: async ({ canvasElement }) => {
    const body = await openCenter(canvasElement);
    const row = await body.findByRole("link", { name: UPSTREAM_LINK });
    row.addEventListener("click", (event) => event.preventDefault());
    await userEvent.click(row);
    await expect(markRead).toHaveBeenCalledWith("n3");
  },
};

export const ReadAllInCategory: Story = {
  name: "«Прочитать все» с текущей категорией",
  args: { category: "security", onReadAll: readAll },
  play: async ({ canvasElement }) => {
    const body = await openCenter(canvasElement);
    await userEvent.click(await body.findByRole("button", { name: "Прочитать все" }));
    await waitFor(() => expect(readAll).toHaveBeenCalledWith("security"));
  },
};

export const WithoutReadAll: Story = {
  name: "Без «Прочитать все»",
  play: async ({ canvasElement }) => {
    const body = await openCenter(canvasElement);
    await body.findByRole("tab", { name: ALL_TAB });
    await expect(body.queryByRole("button", { name: "Прочитать все" })).not.toBeInTheDocument();
  },
};

export const LoadMore: Story = {
  name: "Загрузить ещё",
  args: { onReadAll: readAll, loadMore: { label: "Загрузить ещё", run: { onSelect: fn() } } },
  play: async ({ canvasElement }) => {
    const body = await openCenter(canvasElement);
    await expect(await body.findByRole("button", { name: "Загрузить ещё" })).toBeInTheDocument();
  },
};

export const Loading: Story = {
  name: "Загрузка",
  args: { loading: true },
  play: async ({ canvasElement }) => {
    const body = await openCenter(canvasElement);
    await body.findByRole("tab", { name: ALL_TAB });
    await expect(body.queryAllByRole("link")).toHaveLength(0);
  },
};

export const Empty: Story = {
  name: "Новых уведомлений нет",
  args: {
    items: [],
    unread: 0,
    unreadByCategory: { security: 0, access: 0, platform: 0 },
  },
  play: async ({ canvasElement }) => {
    const body = await openCenter(canvasElement);
    await expect(await body.findByText("Новых уведомлений нет")).toBeInTheDocument();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  args: { onReadAll: readAll },
  globals: { theme: "dark" },
  play: async ({ canvasElement }) => {
    const body = await openCenter(canvasElement);
    await body.findByRole("tab", { name: SECURITY_TAB });
  },
};

export const English: Story = {
  name: "EN: подписи на английском",
  args: { onReadAll: readAll },
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    await userEvent.click(
      within(canvasElement).getByRole("button", { name: "Notifications, 3 unread" }),
    );
    const body = within(document.body);
    const popup = await body.findByRole("dialog", { name: "Notifications" });
    await waitFor(() => expect(popup).toContainElement(document.activeElement as HTMLElement));
    await expect(body.getByRole("tab", { name: SECURITY_EN })).toBeInTheDocument();
    await expect(body.getByRole("button", { name: "Mark all as read" })).toBeInTheDocument();
  },
};
