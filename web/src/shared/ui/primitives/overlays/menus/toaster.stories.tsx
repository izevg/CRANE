import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { showToast } from "~/shared/lib/state";
import { Button } from "~/shared/ui/primitives";

// Сам Toaster смонтирован декоратором витрины; здесь он показывает тосты из showToast.
const notifySuccess = () => showToast.success("Токен crp_a1b2c3d4 создан");
const notifyInfo = () => showToast.info("Поставлено в очередь: 41");
const notifyError = () =>
  showToast.error({ title: "Не удалось отозвать токен", requestId: "0192f3a4-7b1c-7e21" });

const meta = {
  title: "Примитивы/Toaster",
  parameters: { layout: "centered" },
  render: () => (
    <div className="flex flex-wrap gap-2">
      <Button variant="secondary" onClick={notifySuccess}>
        Успех
      </Button>
      <Button variant="secondary" onClick={notifyInfo}>
        Информация
      </Button>
      <Button variant="secondary" onClick={notifyError}>
        Ошибка с request_id
      </Button>
    </div>
  ),
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const Success: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Успех" }));
    const toast = await within(document.body).findByText("Токен crp_a1b2c3d4 создан");
    await waitFor(() => expect(toast).toBeVisible());
  },
};

export const ErrorWithRequestId: Story = {
  name: "Ошибка остаётся и показывает request_id",
  play: async ({ canvasElement }) => {
    await userEvent.click(
      within(canvasElement).getByRole("button", { name: "Ошибка с request_id" }),
    );
    const id = await within(document.body).findByText("request_id: 0192f3a4-7b1c-7e21");
    await waitFor(() => expect(id).toBeVisible());
    await expect(
      within(document.body).getByRole("button", { name: "Копировать request_id" }),
    ).toBeInTheDocument();
  },
};
