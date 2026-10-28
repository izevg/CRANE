import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import {
  Button,
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverTitle,
  PopoverTrigger,
} from "~/shared/ui/primitives";

function StatusPopover({ defaultOpen = false }: { defaultOpen?: boolean }) {
  return (
    <Popover defaultOpen={defaultOpen}>
      <PopoverTrigger render={<Button variant="secondary" />}>Состояние</PopoverTrigger>
      <PopoverContent>
        <PopoverTitle>Режим: норма</PopoverTitle>
        <PopoverDescription>Все компоненты работают, последняя проверка 14:05.</PopoverDescription>
      </PopoverContent>
    </Popover>
  );
}

const meta = {
  title: "Примитивы/Popover",
  component: StatusPopover,
} satisfies Meta<typeof StatusPopover>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Состояние" }));
    const text = await within(document.body).findByText("Режим: норма");
    await waitFor(() => expect(text).toBeVisible());
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByText("Режим: норма")).toBeNull());
  },
};

/** Остаётся открытым в момент axe-проверки. */
export const Open: Story = {
  args: { defaultOpen: true },
  play: async () => {
    await waitFor(async () =>
      expect(await within(document.body).findByText("Режим: норма")).toBeVisible(),
    );
  },
};

const PROBE_SCALE = "matrix(0.95, 0, 0, 0.95, 0, 0)";

/** Стартовое состояние входа: элемент с классом анимации и атрибутом Base UI, без самого оверлея. */
function startingTransform(): string {
  const probe = document.createElement("div");
  probe.className = "anim-popup";
  probe.setAttribute("data-starting-style", "");
  document.body.append(probe);
  const { transform } = getComputedStyle(probe);
  probe.remove();
  return transform;
}

/** Остаётся открытым в момент axe-проверки. Движение — только явные свойства, origin от триггера (spec 2). */
export const MotionContract: Story = {
  name: "Движение: transform и opacity, origin от триггера",
  args: { defaultOpen: true },
  play: async () => {
    const popup = await waitFor(() => {
      const element = document.querySelector("[data-slot=popover-content]");
      if (!(element instanceof HTMLElement)) {
        throw new Error("поповер не открыт");
      }
      return element;
    });
    const style = getComputedStyle(popup);
    await expect(style.transitionProperty).not.toContain("all");
    await expect(style.transitionProperty).toContain("opacity");
    await expect(style.transitionProperty).toContain("transform");
    // origin задаёт Base UI (--transform-origin) по триггеру; в центре самого окна он быть не должен
    const [x, y] = style.transformOrigin.split(" ").map(parseFloat);
    const centerX = popup.offsetWidth / 2;
    const centerY = popup.offsetHeight / 2;
    await expect(
      Math.abs((x ?? centerX) - centerX) + Math.abs((y ?? centerY) - centerY),
    ).toBeGreaterThan(1);
    await expect(startingTransform()).toBe(PROBE_SCALE);
  },
};

export const ReducedMotionNoTransform: Story = {
  name: "Reduced motion: вход без transform",
  globals: { motion: "reduce" },
  play: async () => {
    await waitFor(() => expect(document.documentElement).toHaveAttribute("data-motion", "reduce"));
    await expect(startingTransform()).toBe("none");
    await expect(
      getComputedStyle(document.documentElement).getPropertyValue("--dur-base").trim(),
    ).toBe("150ms");
  },
};
