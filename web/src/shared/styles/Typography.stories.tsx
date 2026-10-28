import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor, within } from "storybook/test";

function Typography() {
  return (
    <div className="flex max-w-3xl flex-col gap-4 p-6">
      <h1 className="text-h1">Onest: заголовок страницы, Registry 1.4.2</h1>
      <h2 className="text-h2">Заголовок раздела «Находки», CVE-2026-31337</h2>
      <p className="text-body">
        Реестр артефактов CRANE: пакеты npm, PyPI, Maven, NuGet и Cargo. Съешь ещё этих мягких
        французских булок.
      </p>
      <p className="text-body">
        <span data-testid="tnum-ones" className="tabular-nums">
          1111111111
        </span>
        <br />
        <span data-testid="tnum-eights" className="tabular-nums">
          8888888888
        </span>
      </p>
      <p className="text-caption text-muted-foreground">
        Подпись caption: 12 340 загрузок за 30 дн · 94,2 %
      </p>
    </div>
  );
}

const meta = {
  title: "Основы/Типографика (Onest)",
  component: Typography,
} satisfies Meta<typeof Typography>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Регрессия шрифта кита: Onest применён к тексту и заголовкам, tabular-nums выравнивает цифры. */
export const OnestAndTabularNumbers: Story = {
  name: "Onest и табличные цифры",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await document.fonts.load('400 16px "Onest Variable"', "Аa1");
    const h1 = canvas.getByRole("heading", { level: 1 });
    await expect(getComputedStyle(h1).fontFamily).toContain("Onest Variable");
    await expect(getComputedStyle(canvas.getByRole("heading", { level: 2 })).fontFamily).toContain(
      "Onest Variable",
    );
    await waitFor(() => expect(document.fonts.check('400 16px "Onest Variable"', "Аa")).toBe(true));
    const ones = canvas.getByTestId("tnum-ones").getBoundingClientRect().width;
    const eights = canvas.getByTestId("tnum-eights").getBoundingClientRect().width;
    await expect(Math.abs(ones - eights)).toBeLessThan(0.5);
  },
};
