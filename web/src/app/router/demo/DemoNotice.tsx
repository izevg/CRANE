import { useLingui } from "@lingui/react/macro";
import { Callout, CalloutDescription, CalloutTitle } from "~/shared/ui/primitives";

/**
 * Постоянная плашка демо-режима: роль и текст, не только цвет; не закрывается. Один экземпляр на документ:
 * её рисует либо шелл (над PageHeader), либо страница без шелла (над карточкой), но не оба сразу.
 */
export function DemoNotice() {
  const { t } = useLingui();
  return (
    <div role="note" className="mb-6">
      <Callout tone="info">
        <CalloutTitle>{t`Демо-режим`}</CalloutTitle>
        <CalloutDescription>
          {t`Данные на экранах примерные, не с сервера. Не принимайте по ним решений.`}
        </CalloutDescription>
      </Callout>
    </div>
  );
}
