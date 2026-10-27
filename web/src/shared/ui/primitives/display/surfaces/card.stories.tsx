import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  Button,
  Card,
  CardAction,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Card",
  component: Card,
  render: (args) => (
    <Card {...args} className="max-w-md">
      <CardHeader>
        <CardTitle>Происхождение последней версии</CardTitle>
        <CardAction>
          <Button variant="ghost" size="sm">
            Открыть
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        Пайплайн #4521, коммит 9f2c1ab, ветка main. Опубликовано из OIDC CI.
      </CardContent>
      <CardFooter>Проверено 27 окт, 14:05</CardFooter>
    </Card>
  ),
} satisfies Meta<typeof Card>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const WithoutFooter: Story = {
  render: () => (
    <Card className="max-w-md">
      <CardHeader>
        <CardTitle>Риски</CardTitle>
      </CardHeader>
      <CardContent>P1 · 1 находка. Лицензии: всё разрешено.</CardContent>
    </Card>
  ),
};
