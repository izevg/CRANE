//! Доступность Valkey для журнала: из событий клиента fred получается один переход на смену
//! состояния. Состояния не хранятся ради метрик: гейдж v0.4.0 читает `Client::state()`.

/// Состояние связи командного клиента с Valkey, как его видит журнал.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Link {
    Available,
    /// Начальное состояние: до первого подключения команды отвечают ошибкой.
    Unavailable,
}

/// Событие клиента fred, значимое для доступности.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkEvent {
    /// `error_rx`: обрыв, зависшее соединение (`unresponsive` закрывает его с ошибкой IO)
    /// или неудачная попытка переподключения.
    Lost,
    /// `reconnect_rx`: рукопожатие с сервером прошло.
    Restored,
}

/// Что записать в журнал.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Notice {
    Unavailable,
    Available,
}

/// Новое состояние и запись в журнал. Запись есть только на смену состояния: пока Valkey
/// недоступен, каждая неудачная попытка переподключения шлёт `Lost`, и без этого журнал
/// повторял бы одну строку на каждую попытку.
pub(crate) fn step(previous: Link, event: LinkEvent) -> (Link, Option<Notice>) {
    match (previous, event) {
        (Link::Available, LinkEvent::Lost) => (Link::Unavailable, Some(Notice::Unavailable)),
        (Link::Unavailable, LinkEvent::Restored) => (Link::Available, Some(Notice::Available)),
        (Link::Available, LinkEvent::Restored) => (Link::Available, None),
        (Link::Unavailable, LinkEvent::Lost) => (Link::Unavailable, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_reports_only_state_changes() {
        // Arrange
        let cases = [
            (Link::Available, LinkEvent::Lost),
            (Link::Unavailable, LinkEvent::Lost),
            (Link::Unavailable, LinkEvent::Restored),
            (Link::Available, LinkEvent::Restored),
        ];

        // Act
        let actual = cases.map(|(previous, event)| step(previous, event));

        // Assert
        assert_eq!(
            actual,
            [
                (Link::Unavailable, Some(Notice::Unavailable)),
                (Link::Unavailable, None),
                (Link::Available, Some(Notice::Available)),
                (Link::Available, None),
            ]
        );
    }
}
