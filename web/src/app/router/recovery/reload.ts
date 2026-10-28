/** Полная перезагрузка на текущий (целевой) адрес. Вынесена, чтобы тесты подменяли браузерный API. */
export function reloadPage(): void {
  window.location.reload();
}
