import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { CopyableId } from "~/shared/ui/actions";
import { CodeBlock } from "~/shared/ui/data-display";
import {
  Callout,
  MachineValue,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "~/shared/ui/primitives";
import { contrastRatio, effectiveBackground } from "~test/contrast";

const SHA256 = "9b3f1c7ad04e5b82c6f01d3a7e48b9250c1f6d8e3a47b05c9d2e18f4a6b7c3d1";
const TOKEN_PREFIX = "crp_a1b2c3d4";
const UUID = "0192f3a4-7b1c-7e21-9d21-5be0c1d46973";
const MIN_CONTRAST = 4.5;

/** Цвет токена, как его видит браузер: probe-элемент с `color: var(--token)`. */
function tokenColor(token: string, host: Element = document.body): string {
  const probe = document.createElement("span");
  probe.style.color = `var(${token})`;
  host.append(probe);
  const { color } = getComputedStyle(probe);
  probe.remove();
  return color;
}

function seriesOf(root: HTMLElement, text: string): HTMLElement {
  const found = [...root.querySelectorAll<HTMLElement>("[data-slot=machine-value] > span")].find(
    (span) => span.textContent === text,
  );
  if (found === undefined) {
    throw new Error(`нет серии «${text}»`);
  }
  return found;
}

const meta = {
  title: "Примитивы/MachineValue",
  component: MachineValue,
  args: { value: SHA256 },
  decorators: [
    (Story) => (
      <div className="rounded-xl bg-card p-4 text-body">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof MachineValue>;

export default meta;
type Story = StoryObj<typeof meta>;

async function expectDigit(canvasElement: HTMLElement, text: string) {
  await expect(getComputedStyle(seriesOf(canvasElement, text)).color).toBe(
    tokenColor("--char-digit", canvasElement),
  );
}

async function expectSymbol(canvasElement: HTMLElement, text: string) {
  await expect(getComputedStyle(seriesOf(canvasElement, text)).color).toBe(
    tokenColor("--char-symbol", canvasElement),
  );
}

export const Sha256: Story = {
  name: "SHA-256: цифры и буквы, выделение даёт точную строку",
  play: async ({ canvasElement }) => {
    const layer = canvasElement.querySelector("[data-slot=machine-value]");
    if (!(layer instanceof HTMLElement)) {
      throw new Error("нет размеченного слоя");
    }
    await expectDigit(canvasElement, "9");
    await expect(layer.textContent).toBe(SHA256);
    await expect(layer).toHaveAttribute("aria-hidden", "true");
    // доступное значение — одним куском
    await expect(
      within(canvasElement).getByText(SHA256, { selector: ".sr-only" }),
    ).toBeInTheDocument();
    const range = document.createRange();
    range.selectNodeContents(layer);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    await expect(selection?.toString()).toBe(SHA256);
    selection?.removeAllRanges();
    await expect(
      contrastRatio(
        getComputedStyle(seriesOf(canvasElement, "9")).color,
        effectiveBackground(layer),
      ),
    ).toBeGreaterThanOrEqual(MIN_CONTRAST);
  },
};

export const TokenPrefix: Story = {
  name: "Префикс токена crp_a1b2c3d4: знак подчёркивания — symbol",
  args: { value: TOKEN_PREFIX },
  play: async ({ canvasElement }) => {
    await expectDigit(canvasElement, "1");
    await expectSymbol(canvasElement, "_");
  },
};

export const Uuid: Story = {
  name: "UUID: дефисы — symbol",
  args: { value: UUID },
  play: async ({ canvasElement }) => {
    await expectDigit(canvasElement, "0192");
    await expectSymbol(canvasElement, "-");
  },
};

export const SrOnlyFullValue: Story = {
  name: "Усечённое значение: скринридер получает полное",
  args: { value: "9b3f…c3d1", spoken: SHA256 },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByText(SHA256, { selector: ".sr-only" }),
    ).toBeInTheDocument();
    await expectDigit(canvasElement, "3");
    await expectSymbol(canvasElement, "…");
  },
};

export const TooLong: Story = {
  name: "Длиннее 64 знаков — без разметки",
  args: { value: `${SHA256}${SHA256}` },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelector("[data-slot=machine-value]")).toBeNull();
    await expect(canvasElement).toHaveTextContent(`${SHA256}${SHA256}`);
  },
};

export const DarkTheme: Story = {
  name: "Тёмная тема: контраст на карточке",
  globals: { theme: "dark" },
  play: async ({ canvasElement }) => {
    const digit = seriesOf(canvasElement, "9");
    const background = effectiveBackground(digit);
    await expect(contrastRatio(getComputedStyle(digit).color, background)).toBeGreaterThanOrEqual(
      MIN_CONTRAST,
    );
    const symbols = seriesOf(canvasElement, "3");
    await expect(symbols).toBeTruthy();
  },
};

export const PlainOnStatusBackground: Story = {
  name: "На статусном фоне — marked={false}, без разметки",
  render: () => (
    <Callout tone="warning">
      Хэш не совпал: <MachineValue value={SHA256} marked={false} />
    </Callout>
  ),
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelector("[data-slot=machine-value]")).toBeNull();
    await expect(canvasElement).toHaveTextContent(SHA256);
  },
};

/** Остаётся открытым в момент axe-проверки. В тултипе токены инвертируются, буквы — цвет текста тултипа. */
export const InTooltip: Story = {
  name: "В Tooltip: инвертированные токены",
  render: () => (
    <Tooltip defaultOpen>
      <TooltipTrigger render={<button type="button" />}>request_id</TooltipTrigger>
      <TooltipContent>
        <MachineValue value={UUID} />
      </TooltipContent>
    </Tooltip>
  ),
  play: async () => {
    const popup = await within(document.body).findByText("0192", {
      selector: "[data-slot=machine-value] > span",
    });
    await expect(getComputedStyle(popup).color).toBe(tokenColor("--char-digit-inverse"));
  },
};

export const InCopyableId: Story = {
  name: "CopyableId: видимые части размечены, «…» — symbol",
  render: () => <CopyableId value={SHA256} label="Скопировать SHA-256" />,
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelector("[data-slot=machine-value]")?.textContent).toBe(
      "9b3f…c3d1",
    );
    await expectDigit(canvasElement, "9");
    await expectSymbol(canvasElement, "…");
    await expect(within(canvasElement).getByRole("button")).toHaveAccessibleName(
      `Скопировать SHA-256: ${SHA256}`,
    );
  },
};

export const InCodeBlock: Story = {
  name: "CodeBlock marked: секрет на фоне code-bg",
  render: () => <CodeBlock code="crp_a1B2-c3D4_e5F6" copyLabel="Копировать секрет" marked />,
  play: async ({ canvasElement }) => {
    await expectDigit(canvasElement, "1");
    await expectSymbol(canvasElement, "_");
    const digit = seriesOf(canvasElement, "1");
    await expect(
      contrastRatio(getComputedStyle(digit).color, effectiveBackground(digit)),
    ).toBeGreaterThanOrEqual(MIN_CONTRAST);
  },
};

/** Размеченный слой сам ставит моноширинный шрифт: в контексте с гарнитурой текста (Tooltip, абзац) он не Onest. */
export const OwnMonoFont: Story = {
  name: "Слой разметки сам моноширинный",
  render: () => (
    <p className="font-sans">
      <MachineValue value={SHA256} />
    </p>
  ),
  play: async ({ canvasElement }) => {
    const layer = canvasElement.querySelector("[data-slot=machine-value]");
    if (!(layer instanceof HTMLElement)) {
      throw new Error("нет размеченного слоя");
    }
    await expect(getComputedStyle(layer).fontFamily).toContain("JetBrains Mono");
  },
};
