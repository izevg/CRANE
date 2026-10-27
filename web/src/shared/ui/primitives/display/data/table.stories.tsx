import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/shared/ui/primitives";

const VERSIONS = [
  { version: "1.4.2", state: "Выпущена", published: "27 окт, 14:05", by: "Иванов И." },
  { version: "2.0.0-rc.1", state: "Карантин", published: "26 окт, 18:40", by: "Петрова А." },
  { version: "1.3.9", state: "Заблокирована", published: "9 окт, 11:12", by: "Иванов И." },
];

const meta = {
  title: "Примитивы/Table",
  component: Table,
  render: () => (
    <Table>
      <TableCaption>Версии @acme/ui</TableCaption>
      <TableHeader>
        <TableRow>
          <TableHead>Версия</TableHead>
          <TableHead>Состояние</TableHead>
          <TableHead>Опубликована</TableHead>
          <TableHead>Кем</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {VERSIONS.map((row) => (
          <TableRow key={row.version}>
            <TableCell className="tabular-nums">{row.version}</TableCell>
            <TableCell>{row.state}</TableCell>
            <TableCell>{row.published}</TableCell>
            <TableCell>{row.by}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  ),
} satisfies Meta<typeof Table>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Versions: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("table", { name: "Версии @acme/ui" })).toBeInTheDocument();
    await expect(canvas.getAllByRole("row")).toHaveLength(4);
  },
};
