import { useLingui } from "@lingui/react/macro";
import type { Component } from "~/shared/api";
import { useTextFormat } from "~/shared/lib/i18n";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { COMPONENT_STATUS } from "../model/labels";

const NO_VALUE = "—";

/** Задержка с единицей; нет данных — «—», а не 0. */
function Latency({ ms }: { ms: number | null | undefined }) {
  const { t } = useLingui();
  const format = useTextFormat();
  if (ms === null || ms === undefined) {
    return <>{NO_VALUE}</>;
  }
  const value = format.number(ms);
  return <>{t`${value} мс`}</>;
}

/** Состояние компонентов платформы. Состояние — бейдж с текстом; задержка без данных — «—». */
export function ComponentsCard({ components }: { components: readonly Component[] }) {
  const { t } = useLingui();
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t`Компоненты`}</CardTitle>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t`Компонент`}</TableHead>
              <TableHead>{t`Состояние`}</TableHead>
              <TableHead className="text-right">{t`Задержка`}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {components.map((component) => (
              <TableRow key={component.name}>
                <TableCell className="font-semibold">{component.name}</TableCell>
                <TableCell>
                  <StatusBadge {...COMPONENT_STATUS[component.state]} />
                </TableCell>
                <TableCell className="text-right tabular-nums">
                  <Latency ms={component.latency_ms} />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
