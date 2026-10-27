import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
  Input,
  Textarea,
} from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Field",
  component: Field,
} satisfies Meta<typeof Field>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WithHint: Story = {
  render: () => (
    <Field name="token-name" className="max-w-sm">
      <FieldLabel>Название токена</FieldLabel>
      <Input defaultValue="ci-platform" />
      <FieldDescription>До 64 символов, видно только вам</FieldDescription>
    </Field>
  ),
  play: async ({ canvasElement }) => {
    const input = within(canvasElement).getByRole("textbox", { name: "Название токена" });
    await expect(input).toHaveAccessibleDescription("До 64 символов, видно только вам");
  },
};

export const WithError: Story = {
  render: () => (
    <Field invalid name="package" className="max-w-sm">
      <FieldLabel>Пакет</FieldLabel>
      <Input defaultValue="@acme/ui" />
      <FieldError match>Пакет @acme/ui уже существует в workspace platform</FieldError>
    </Field>
  ),
  play: async ({ canvasElement }) => {
    const input = within(canvasElement).getByRole("textbox", { name: "Пакет" });
    await expect(input).toBeInvalid();
    await expect(input).toHaveAccessibleDescription(
      "Пакет @acme/ui уже существует в workspace platform",
    );
  },
};

export const Multiline: Story = {
  render: () => (
    <Field name="reason" className="max-w-sm">
      <FieldLabel>Причина исключения</FieldLabel>
      <Textarea placeholder="Опишите, почему находка неприменима" />
    </Field>
  ),
  play: async ({ canvasElement }) => {
    const area = within(canvasElement).getByRole("textbox", { name: "Причина исключения" });
    await userEvent.type(area, "Уязвимый код не вызывается");
    await expect(area).toHaveValue("Уязвимый код не вызывается");
  },
};

export const Disabled: Story = {
  render: () => (
    <Field disabled name="namespace" className="max-w-sm">
      <FieldLabel>Пространство имён</FieldLabel>
      <Input defaultValue="@acme/*" />
    </Field>
  ),
};
