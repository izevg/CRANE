import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ComponentProps, useCallback, useState } from "react";
import { expect, fn, spyOn, userEvent, within } from "storybook/test";
import { InstallBlock } from "../index";

// Ссылка на переменную окружения собирается из частей, чтобы линтер не принял её за шаблонную строку.
const COPY = /^Копировать/;
const TOKEN_REF = ["$", "{CRANE_TOKEN}"].join("");
const onClientChange = fn();

const VIEWPORTS = {
  mobile: { name: "Телефон 390", styles: { width: "390px", height: "844px" } },
};

const NPM: NonNullable<ComponentProps<typeof InstallBlock>["instructions"]> = {
  client: "npm",
  available_clients: ["npm", "pnpm", "yarn"],
  setup_steps: [{ title: "Вход и настройка npm", code: "cctl login\ncctl setup npm" }],
  manual_config: {
    path: ".npmrc",
    code: `@acme:registry=https://crane.example/npm/\n//crane.example/npm/:_authToken=${TOKEN_REF}`,
  },
  install: { code: "npm i @acme/ui@1.4.2" },
};

/** Страница держит выбранного клиента; в истории это локальное состояние, а выбор ещё и уходит в `onClientChange`. */
function Controlled(props: ComponentProps<typeof InstallBlock>) {
  const [client, setClient] = useState(props.client);
  const { onClientChange: notify, instructions } = props;
  const change = useCallback(
    (next: NonNullable<ComponentProps<typeof InstallBlock>["client"]>) => {
      setClient(next);
      notify(next);
    },
    [notify],
  );
  // «Сервер» отвечает сразу для выбранного клиента; story загрузки задают инструкции сами
  const answered =
    instructions === undefined || client === undefined ? instructions : { ...instructions, client };
  return (
    <InstallBlock {...props} client={client} instructions={answered} onClientChange={change} />
  );
}

const meta = {
  title: "Возможности/Установка",
  component: InstallBlock,
  render: (args) => <Controlled {...args} />,
  parameters: { viewport: { options: VIEWPORTS } },
  // Буфер Chromium в headless недоступен без разрешений; выбор клиента не должен течь между историями.
  beforeEach: () => {
    localStorage.clear();
    spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  },
  args: {
    client: "npm",
    instructions: NPM,
    onClientChange,
    envHref: "/help/environment",
  },
} satisfies Meta<typeof InstallBlock>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Npm: Story = {
  name: "npm: три клиента",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: "npm" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await userEvent.click(canvas.getByRole("button", { name: "pnpm" }));
    await expect(onClientChange).toHaveBeenCalledWith("pnpm");
    await expect(canvas.getByRole("button", { name: "pnpm" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  },
};

export const Pypi: Story = {
  name: "PyPI: pip, uv, poetry",
  args: {
    client: "pip",
    instructions: {
      client: "pip",
      available_clients: ["pip", "uv", "poetry"],
      setup_steps: [{ title: "Вход и настройка pip", code: "cctl login\ncctl setup pypi" }],
      manual_config: {
        path: "pip.conf",
        code: `[global]\nindex-url = https://__token__:${TOKEN_REF}@crane.example/pypi/simple/`,
      },
      install: { code: "pip install acme-billing-client==3.1.0" },
    },
  },
};

export const Maven: Story = {
  name: "Maven: Maven и Gradle",
  args: {
    client: "maven",
    instructions: {
      client: "maven",
      available_clients: ["maven", "gradle"],
      setup_steps: [{ title: "Вход и настройка Maven", code: "cctl login\ncctl setup maven" }],
      manual_config: {
        path: "settings.xml",
        code: `<server>\n  <id>crane</id>\n  <password>${TOKEN_REF}</password>\n</server>`,
      },
      install: {
        code: "<dependency>\n  <groupId>org.acme</groupId>\n  <artifactId>payments-core</artifactId>\n  <version>7.2.1</version>\n</dependency>",
      },
    },
  },
};

export const Cargo: Story = {
  name: "Cargo: один клиент",
  args: {
    client: "cargo",
    instructions: {
      client: "cargo",
      available_clients: ["cargo"],
      setup_steps: [{ title: "Вход и настройка cargo", code: "cctl login\ncctl setup cargo" }],
      manual_config: {
        path: ".cargo/config.toml",
        code: `[registries.crane]\nindex = "sparse+https://crane.example/cargo/"\ntoken = "${TOKEN_REF}"`,
      },
      install: { code: "cargo add serde@1.0.217 --registry crane" },
    },
  },
};

export const Loading: Story = {
  name: "До первого ответа: заглушки, копировать нечего",
  args: { client: undefined, instructions: undefined },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("button", { name: COPY })).toBeNull();
    await expect(canvasElement.querySelectorAll("[aria-busy=true]")).toHaveLength(2);
  },
};

export const Refetching: Story = {
  name: "Клиент сменили, инструкции ещё старые: заглушки вместо чужих команд",
  args: { client: "pnpm" },
  render: (args) => <InstallBlock {...args} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: "pnpm" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(canvas.queryByRole("button", { name: COPY })).toBeNull();
  },
};

export const ManualCollapsed: Story = {
  name: "Вручную: свёрнуто",
  play: async ({ canvasElement }) => {
    const toggle = within(canvasElement).getByRole("button", {
      name: "Настроить вручную (.npmrc)",
    });
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
  },
};

export const ManualExpanded: Story = {
  name: "Вручную: развёрнуто, токен — только переменная окружения",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Настроить вручную (.npmrc)" }));
    await expect(
      canvas.getByRole("button", { name: "Настроить вручную (.npmrc)" }),
    ).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByRole("region", { name: "Файл .npmrc" })).toHaveTextContent(TOKEN_REF);
    await expect(canvas.getByRole("link", { name: "Переменные окружения" })).toHaveAttribute(
      "href",
      "/help/environment",
    );
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};

export const Mobile: Story = {
  name: "Телефон 390: длинные команды прокручиваются",
  globals: { viewport: { value: "mobile", isRotated: false } },
  args: {
    instructions: {
      ...NPM,
      install: {
        code: "npm install @acme/ui@1.4.2 --registry https://crane.example/npm/ --userconfig ~/.config/crane/npmrc",
      },
    },
  },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.scrollWidth).toBeLessThanOrEqual(canvasElement.clientWidth + 1);
  },
};

export const English: Story = {
  name: "EN: подписи",
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole("heading", { name: "Install" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Configure manually (.npmrc)" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Copy install command" })).toBeVisible();
  },
};
