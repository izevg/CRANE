// Ставит data-theme до загрузки бандла, чтобы страница не мигала в чужой теме.
// Статичный браузерный скрипт: TS здесь не исполняется. Логика совпадает с colorScheme.ts (ключ crane.colorScheme).
(() => {
  var mode = "system";
  var stored = null;
  var dark = false;
  try {
    stored = localStorage.getItem("crane.colorScheme");
  } catch (_) {
    stored = null;
  }
  if (stored === "light" || stored === "dark" || stored === "system") {
    mode = stored;
  }
  if (mode === "dark") {
    dark = true;
  } else if (mode === "system") {
    try {
      dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    } catch (_) {
      dark = false;
    }
  }
  document.documentElement.setAttribute("data-theme", dark ? "dark" : "light");
  document.documentElement.style.colorScheme = dark ? "dark" : "light";
})();
