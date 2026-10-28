//! Общее для адаптеров на `sqlx`: время в `timestamptz` и различение «БД недоступна» и «сбой».

mod reach;
mod time;

pub(crate) use reach::is_unreachable;
pub(crate) use time::{from_db_time, to_db_time};
