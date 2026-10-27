import { toast } from "sonner";
import { copyToClipboard } from "~/shared/lib/core";

/** Успех пропадает сам; ошибка остаётся, пока пользователь её не закроет (patterns §30). */
const SUCCESS_MS = 4000;
const COPY_REQUEST_ID = "Копировать request_id";
const COPY_FAILED = "Не удалось скопировать — выделите текст вручную";

type ErrorToast = {
  title: string;
  /** Идентификатор запроса для обращения в поддержку: показывается в тосте и копируется кнопкой. */
  requestId?: string;
};

/**
 * Тосты — не единственный носитель результата важного действия: состояние должно меняться и на странице.
 * Контейнер `Toaster` монтируется один раз в корне приложения.
 */
export const showToast = {
  success(title: string): void {
    toast.success(title, { duration: SUCCESS_MS });
  },
  info(title: string): void {
    toast.info(title, { duration: SUCCESS_MS });
  },
  error({ title, requestId }: ErrorToast): void {
    toast.error(title, {
      duration: Number.POSITIVE_INFINITY,
      closeButton: true,
      ...(requestId === undefined
        ? {}
        : {
            description: `request_id: ${requestId}`,
            action: {
              label: COPY_REQUEST_ID,
              onClick: (event) => {
                // тост остаётся на экране: по умолчанию sonner закрывает его после действия
                event.preventDefault();
                void copyToClipboard(requestId).then((copied) => {
                  if (!copied) {
                    toast.error(COPY_FAILED, {
                      duration: Number.POSITIVE_INFINITY,
                      closeButton: true,
                    });
                  }
                });
              },
            },
          }),
    });
  },
};
