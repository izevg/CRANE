use super::*;
use std::time::SystemTime;

const NAME: &str = "px:name";

fn entry() -> Cached {
    Cached {
        fetched_at: SystemTime::UNIX_EPOCH,
        state: CachedState::Missing { misses: 1 },
    }
}

#[test]
fn admit_keeps_entry_when_nothing_was_invalidated_meanwhile() {
    // Arrange
    let local = Local::new(1 << 20);
    let seen = local.generation();

    // Act
    local.admit(NAME, entry(), seen);

    // Assert
    assert_eq!(local.l1.get(NAME), Some(entry()));
}

#[test]
fn admit_drops_entry_read_before_an_invalidation() {
    // Arrange
    let local = Local::new(1 << 20);
    let seen = local.generation();
    local.invalidate("px:other");

    // Act
    local.admit(NAME, entry(), seen);

    // Assert
    assert_eq!(
        local.l1.get(NAME),
        None,
        "любая инвалидация после чтения L2 делает прочитанное подозрительным"
    );
}

#[test]
fn invalidate_removes_only_the_named_entry() {
    // Arrange
    let local = Local::new(1 << 20);
    local.l1.insert(NAME.to_owned(), entry());
    local.l1.insert("px:other".to_owned(), entry());

    // Act
    local.invalidate(NAME);

    // Assert
    assert_eq!(
        (local.l1.get(NAME), local.l1.get("px:other")),
        (None, Some(entry())),
        "инвалидация выбрасывает только названный ключ"
    );
}

#[test]
fn forget_all_empties_l1() {
    // Arrange
    let local = Local::new(1 << 20);
    local.l1.insert(NAME.to_owned(), entry());
    local.l1.insert("px:other".to_owned(), entry());

    // Act
    local.forget_all();

    // Assert
    assert_eq!((local.l1.get(NAME), local.l1.get("px:other")), (None, None));
}

#[test]
fn availability_is_reported_only_when_it_changes() {
    // Arrange
    let cases = [(false, false), (true, true), (false, true), (true, false)];

    // Act
    let actual = cases.map(|(available, next)| availability_change(available, next));

    // Assert
    assert_eq!(actual, [None, None, Some(true), Some(false)]);
}

#[test]
fn weight_counts_overhead_plus_document_body_and_etag() {
    // Arrange
    let document = |body: &'static [u8], etag: Option<&str>| Cached {
        fetched_at: SystemTime::UNIX_EPOCH,
        state: CachedState::Document {
            body: body.into(),
            etag: etag.map(|etag| crane_ports::proxy::ETag::new(etag.to_owned())),
            unchanged: 0,
            max_age: None,
        },
    };
    let entries = [
        entry(),
        document(b"0123456789", None),
        document(b"0123456789", Some("abc")),
    ];

    // Act
    let actual = entries.map(|entry| weight(&entry));

    // Assert
    assert_eq!(actual, [64, 74, 77]);
}

#[tokio::test]
async fn dropping_the_guard_aborts_the_task() {
    // Arrange
    let witness = Arc::new(());
    let held = Arc::clone(&witness);
    let guard = AbortOnDrop(tokio::spawn(async move {
        std::future::pending::<()>().await;
        drop(held);
    }));
    tokio::task::yield_now().await;

    // Act
    drop(guard);

    // Assert
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(Arc::strong_count(&witness), 1);
}

#[test]
fn connection_gives_up_on_a_silent_server_sooner_than_fred_defaults_and_cuts_a_hung_frame() {
    // Arrange
    let defaults = ConnectionConfig::default();

    // Act
    let actual = connection_config();

    // Assert
    assert_eq!(
        (
            actual.connection_timeout < defaults.connection_timeout,
            actual.internal_command_timeout < defaults.internal_command_timeout,
            actual.unresponsive.max_timeout.is_some(),
            actual.unresponsive.interval * 2 < COMMAND_TIMEOUT,
        ),
        (true, true, true, true),
        "рукопожатие и соединение короче умолчаний fred, зависший кадр рвёт соединение, проверка чаще половины потолка команды"
    );
}

#[test]
fn every_command_has_a_deadline_and_fails_fast_without_a_connection() {
    // Arrange
    let ceiling = Duration::from_secs(1);

    // Act
    let options = command_options();

    // Assert
    assert_eq!(
        (
            options.timeout.is_some_and(|limit| limit <= ceiling),
            options.fail_fast
        ),
        (true, true)
    );
}
