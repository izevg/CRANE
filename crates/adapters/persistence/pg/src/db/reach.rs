//! Различение «БД недоступна» (повтор позже имеет смысл, клиенту 503) и сбоя (баг, клиенту 500).

/// Соединение или сервер БД недоступны: повтор позже имеет смысл.
pub(crate) fn is_unreachable(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::PoolTimedOut
        | sqlx::Error::PoolClosed
        | sqlx::Error::Io(_)
        | sqlx::Error::Tls(_)
        | sqlx::Error::WorkerCrashed => true,
        sqlx::Error::Database(db) => db.code().is_some_and(|code| unreachable_code(&code)),
        _ => false,
    }
}

/// SQLSTATE «сервер не принимает запросы»: класс 08 (сбой соединения) и остановка сервера
/// (57P01 `admin_shutdown`, 57P02 `crash_shutdown`, 57P03 `cannot_connect_now`). При `compose stop`
/// живая сессия получает именно 57P01, а не ошибку ввода-вывода.
fn unreachable_code(code: &str) -> bool {
    code.starts_with("08") || matches!(code, "57P01" | "57P02" | "57P03")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_shutdown_and_connection_sqlstates_are_unreachable() {
        // Arrange
        let codes = [
            "08000", "08006", "57P01", "57P02", "57P03", "23505", "40001", "57014", "42P01", "",
        ];

        // Act
        let actual = codes.map(unreachable_code);

        // Assert
        assert_eq!(
            actual,
            [
                true, true, true, true, true, false, false, false, false, false
            ]
        );
    }

    #[test]
    fn connection_and_pool_errors_are_unreachable_but_query_errors_are_not() {
        // Arrange
        let cases = [
            sqlx::Error::PoolTimedOut,
            sqlx::Error::PoolClosed,
            sqlx::Error::WorkerCrashed,
            sqlx::Error::Io(std::io::Error::other("connection refused")),
            sqlx::Error::RowNotFound,
            sqlx::Error::ColumnNotFound("x".to_owned()),
        ];

        // Act
        let actual = cases.map(|error| is_unreachable(&error));

        // Assert
        assert_eq!(actual, [true, true, true, true, false, false]);
    }
}
