import { msg } from "@lingui/core/macro";
import { fn } from "storybook/test";
import { i18n } from "~/shared/lib/i18n";
import type { Action } from "~/shared/ui/actions";
import { PEOPLE, REPOSITORIES } from "../data/common";

export type TopRow = {
  id: string;
  title: string;
  caption: string;
  action?: Action;
  badge?: string;
};

export function retentionRows(): readonly TopRow[] {
  const npmRepository = REPOSITORIES[0].id;
  const mavenRepository = REPOSITORIES[3].id;
  return [
    {
      id: "r1",
      title: i18n._(msg`Версии без загрузок 18 мес · ${npmRepository}`),
      caption: i18n._(msg`612 версий · 94 ГБ`),
      badge: i18n._(msg`правило 1`),
    },
    {
      id: "r2",
      title: i18n._(msg`Snapshot-сборки старше 30 дн · ${mavenRepository}`),
      caption: i18n._(msg`498 версий · 71 ГБ`),
      badge: i18n._(msg`правило 2`),
    },
    {
      id: "r3",
      title: i18n._(msg`Образы без тега старше 14 дн · oci-hosted`),
      caption: i18n._(msg`174 версии · 47 ГБ`),
      badge: i18n._(msg`правило 3`),
    },
  ];
}

export const deadJobs = (): readonly TopRow[] =>
  [
    { kind: "scan.trivy", attempts: 5, error: "Timeout connecting to Trivy", id: "j-4f2a" },
    { kind: "storage.move", attempts: 3, error: "S3 PutObject: SlowDown (503)", id: "j-91cc" },
    { kind: "import.nexus", attempts: 5, error: "connection reset by peer", id: "j-07be" },
  ].map((job) => ({
    id: job.id,
    title: `${job.kind} · ${job.id}`,
    caption: i18n._(msg`попыток ${job.attempts} · Последняя ошибка: ${job.error}`),
    action: { label: i18n._(msg`Повторить ${job.id}`), run: { onSelect: fn() } },
  }));

export function integrityRows(): readonly TopRow[] {
  const runBy = PEOPLE[1];
  return [
    {
      id: "i1",
      title: i18n._(msg`Последняя проверка`),
      caption: i18n._(msg`27 окт. 2026, 03:10 · 12 мин · запустил ${runBy}`),
    },
    {
      id: "i2",
      title: i18n._(msg`Проверено`),
      caption: i18n._(msg`1 % объектов: 4 812 из 481 200`),
    },
    {
      id: "i3",
      title: i18n._(msg`Следующая`),
      caption: i18n._(msg`28 окт. 2026, 03:10 по расписанию`),
    },
  ];
}
