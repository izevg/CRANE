// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые счётчики
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  type NotificationCategory,
  type NotificationItem,
  NotificationsCenter,
} from "~/widgets/notifications-center";

const BELL = /^Уведомления/;
const UNREAD_LINK = /Непрочитано/;
const TOKEN_LINK = /Токен отозван/;
const SECURITY_TAB = /Безопасность/;

const NO_UNREAD: Record<NotificationCategory, number> = { security: 0, access: 0, platform: 0 };

const unreadItem: NotificationItem = {
  id: "n1",
  category: "security",
  tone: "error",
  title: "Токен отозван администратором",
  time: "2026-10-27T11:30:00Z",
  caption: "ci-deploy",
  href: "/tokens/ci-deploy",
  unread: true,
};

const readItem: NotificationItem = {
  id: "n2",
  category: "platform",
  tone: "info",
  title: "Индексация завершена",
  time: "2026-10-27T10:00:00Z",
  href: "/jobs/42",
  unread: false,
};

type Props = React.ComponentProps<typeof NotificationsCenter>;

async function openCenter(overrides: Partial<Props> = {}) {
  const user = userEvent.setup();
  const props: Props = {
    items: [unreadItem, readItem],
    unread: 3,
    unreadByCategory: { security: 2, access: 0, platform: 1 },
    onCategoryChange: vi.fn(),
    ...overrides,
  };
  render(<NotificationsCenter {...props} />);
  await user.click(screen.getByRole("button", { name: BELL }));
  return { user, props };
}

describe("NotificationsCenter", () => {
  it("колокольчик открывает список со вкладками", async () => {
    // Arrange
    const { user } = await openCenter();

    // Act
    const tabs = screen.getAllByRole("tab");

    // Assert
    expect(tabs.map((tab) => tab.textContent)).toEqual([
      "Все3, 3 непрочитанных",
      "Безопасность2, 2 непрочитанных",
      "Доступ",
      "Платформа1, 1 непрочитанное",
    ]);
    expect(user).toBeDefined();
  });

  it("вкладка с непрочитанными показывает счётчик в имени", async () => {
    // Arrange
    await openCenter();

    // Act
    const tab = screen.getByRole("tab", { name: SECURITY_TAB });

    // Assert
    expect(tab).toHaveAccessibleName("Безопасность, 2 непрочитанных");
  });

  it("смена вкладки сообщает категорию, «Все» — undefined", async () => {
    // Arrange
    const { user, props } = await openCenter();

    // Act
    await user.click(screen.getByRole("tab", { name: SECURITY_TAB }));

    // Assert
    expect(props.onCategoryChange).toHaveBeenCalledWith("security");
  });

  it("возврат на «Все» сообщает undefined", async () => {
    // Arrange
    const { user, props } = await openCenter({ category: "access" });

    // Act
    await user.click(screen.getByRole("tab", { name: "Все, 3 непрочитанных" }));

    // Assert
    expect(props.onCategoryChange).toHaveBeenCalledWith(undefined);
  });

  it("строка — ссылка на объект", async () => {
    // Arrange
    await openCenter();

    // Act
    const link = screen.getByRole("link", { name: TOKEN_LINK });

    // Assert
    expect(link).toHaveAttribute("href", "/tokens/ci-deploy");
    expect(link).toHaveTextContent("ci-deploy");
  });

  it("непрочитанная строка имеет текст для скринридера, прочитанная нет", async () => {
    // Arrange
    await openCenter();

    // Act
    const links = screen.getAllByRole("link");

    // Assert
    expect(links[0]).toHaveAccessibleName(UNREAD_LINK);
    expect(links[1]).not.toHaveAccessibleName(UNREAD_LINK);
  });

  it("переход по строке отмечает её прочитанной", async () => {
    // Arrange
    const onMarkRead = vi.fn();
    const { user } = await openCenter({ onMarkRead });
    const link = screen.getByRole("link", { name: TOKEN_LINK });
    link.addEventListener("click", (event) => event.preventDefault());

    // Act
    await user.click(link);

    // Assert
    expect(onMarkRead).toHaveBeenCalledWith("n1");
  });

  it("без onMarkRead переход по строке не падает", async () => {
    // Arrange
    const { user } = await openCenter();
    const link = screen.getByRole("link", { name: TOKEN_LINK });
    link.addEventListener("click", (event) => event.preventDefault());

    // Act
    await user.click(link);

    // Assert
    expect(screen.queryByRole("link", { name: TOKEN_LINK })).not.toBeInTheDocument();
  });

  it("«Прочитать все» вызывается с текущей категорией", async () => {
    // Arrange
    const onReadAll = vi.fn();
    const { user } = await openCenter({ onReadAll, category: "security" });

    // Act
    await user.click(screen.getByRole("button", { name: "Прочитать все" }));

    // Assert
    expect(onReadAll).toHaveBeenCalledWith("security");
  });

  it("на вкладке «Все» «Прочитать все» вызывается без категории", async () => {
    // Arrange
    const onReadAll = vi.fn();
    const { user } = await openCenter({ onReadAll });

    // Act
    await user.click(screen.getByRole("button", { name: "Прочитать все" }));

    // Assert
    expect(onReadAll).toHaveBeenCalledWith(undefined);
  });

  it("без onReadAll кнопки «Прочитать все» нет", async () => {
    // Arrange
    await openCenter();

    // Act
    const button = screen.queryByRole("button", { name: "Прочитать все" });

    // Assert
    expect(button).not.toBeInTheDocument();
  });

  it("пустой список показывает «Новых уведомлений нет»", async () => {
    // Arrange
    await openCenter({
      items: [],
      unread: 0,
      unreadByCategory: NO_UNREAD,
    });

    // Act
    const empty = screen.getByText("Новых уведомлений нет");

    // Assert
    expect(empty).toBeInTheDocument();
  });

  it("«Загрузить ещё» есть только при loadMore и вызывает его", async () => {
    // Arrange
    const onSelect = vi.fn();
    const { user } = await openCenter({ loadMore: { label: "Загрузить ещё", run: { onSelect } } });

    // Act
    await user.click(screen.getByRole("button", { name: "Загрузить ещё" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("без loadMore кнопки «Загрузить ещё» нет", async () => {
    // Arrange
    await openCenter();

    // Act
    const button = screen.queryByRole("button", { name: "Загрузить ещё" });

    // Assert
    expect(button).not.toBeInTheDocument();
  });

  it("при загрузке вместо строк скелетоны", async () => {
    // Arrange
    await openCenter({ loading: true });

    // Act
    const links = screen.queryAllByRole("link");

    // Assert
    expect(links).toHaveLength(0);
    expect(screen.queryByText("Новых уведомлений нет")).not.toBeInTheDocument();
  });
});
