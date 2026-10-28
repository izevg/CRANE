//! Просмотр: источник, секрет без значения, нечитаемый секрет с причиной, `restart_required`.

use crane_app::settings::Shown;
use crane_domain::settings::{Origin, Unreadable};
use futures_executor::block_on;

use crate::fixtures::{
    HIDDEN, LIMIT, LOCKED, REQUIRED, TOKEN, UnreadableStore, seeded, version, world,
};

#[test]
fn restart_required_reflects_difference_between_started_and_current_versions() {
    // Arrange
    let worlds = [
        (seeded(&[]), &[][..]),
        (seeded(&[(LIMIT, "10")]), &[(LIMIT, 1)][..]),
        (seeded(&[(LIMIT, "10")]), &[][..]),
        (seeded(&[]), &[(LIMIT, 1)][..]),
        (seeded(&[(LIMIT, "10")]), &[(LIMIT, 2)][..]),
    ]
    .map(|(store, started)| world(store, true, started).0);

    // Act
    let actual = worlds.map(|settings| block_on(settings.view()).unwrap().restart_required);

    // Assert
    assert_eq!(actual, [false, false, true, true, true]);
}

#[test]
fn view_lists_only_console_keys_and_ignores_foreign_rows() {
    // Arrange
    let (settings, _) = world(seeded(&[(HIDDEN, "x"), ("ghost", "y")]), true, &[]);

    // Act
    let list = block_on(settings.view()).unwrap();

    // Assert
    let keys: Vec<&str> = list.settings.iter().map(|view| view.def.key).collect();
    assert_eq!(
        (keys, list.restart_required),
        (vec![LIMIT, TOKEN, LOCKED, REQUIRED], false)
    );
}

#[test]
fn view_never_reveals_secret_value() {
    // Arrange
    let (settings, _) = world(seeded(&[(TOKEN, "tok_TOPSECRET")]), true, &[(TOKEN, 1)]);

    // Act
    let list = block_on(settings.view()).unwrap();

    // Assert
    let shown = list.settings.iter().find_map(|view| {
        view.stored
            .as_ref()
            .filter(|_| view.def.key == TOKEN)
            .map(|row| row.value.clone())
    });
    assert_eq!(
        (shown, format!("{list:?}").contains("TOPSECRET")),
        (Some(Shown::Secret), false)
    );
}

#[test]
fn view_shows_reason_for_unreadable_secret() {
    // Arrange
    let reason = Unreadable::WrongKey {
        stored_key_id: "aaa".to_owned(),
        current_key_id: "bbb".to_owned(),
    };
    let (settings, _) = world(UnreadableStore(reason.clone()), true, &[(TOKEN, 3)]);

    // Act
    let list = block_on(settings.view()).unwrap();

    // Assert
    let row = list.settings[1].stored.as_ref().unwrap();
    assert_eq!(
        (row.value.clone(), row.version),
        (Shown::Unreadable(reason), version(3))
    );
}

#[test]
fn view_marks_inherited_key_as_env_and_keeps_stored_row_shadowed() {
    // Arrange
    let (settings, _) = world(seeded(&[(LOCKED, "stored")]), true, &[(LOCKED, 1)]);

    // Act
    let list = block_on(settings.view()).unwrap();

    // Assert
    let locked = &list.settings[2];
    assert_eq!(
        (
            locked.origin(),
            locked.locked(),
            locked.shadowed().is_some()
        ),
        (Origin::Env, true, true)
    );
}
