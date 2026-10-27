//! Перехват JSON-журнала сервера на время одного вызова.

use std::cell::RefCell;
use std::io;
use std::sync::Once;

thread_local! {
    /// Журнал текущего перехвата на этом потоке; `None` — вне перехвата.
    static CAPTURE: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

/// Писатель глобального subscriber'а: дописывает строку в буфер перехвата текущего потока.
#[derive(Clone, Copy)]
struct RoutedWriter;

impl io::Write for RoutedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        CAPTURE.with(|slot| {
            if let Some(sink) = slot.borrow_mut().as_mut() {
                sink.extend_from_slice(buf);
            }
        });
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RoutedWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        *self
    }
}

/// Снимает перехват потока и при панике теста.
struct CaptureGuard;

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        CAPTURE.with(|slot| slot.borrow_mut().take());
    }
}

/// Выполняет `run` и возвращает результат вместе с JSON-журналом, как его пишет сервер.
///
/// Subscriber ставится глобальным ровно один раз на бинарь, а не через `with_default` на каждый
/// тест. Scoped-диспетчеры соседних параллельных тестов портят глобальный кэш интереса
/// callsite'ов: пока зарегистрирован единственный диспетчер, tracing-core считает интерес по
/// диспетчеру потока, впервые дошедшего до callsite. Поток без своего диспетчера кэширует
/// `Interest::never`, и событие пропадает у всех. С постоянным глобальным subscriber'ом
/// интерес не зависит от потока, а изоляция тестов обеспечивается маршрутом по потоку:
/// запрос исполняется на `current_thread` runtime в том же потоке, что и перехват.
///
/// Граница: события фоновых задач (например, spawn ревалидации), которые `run` не дождался,
/// в этот перехват не попадают; они могут попасть в следующий перехват того же потока
/// или потеряться.
pub(crate) fn capture_logs<R>(run: impl FnOnce() -> R) -> (R, String) {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let subscriber = tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_writer(RoutedWriter)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
    });
    CAPTURE.with(|slot| *slot.borrow_mut() = Some(Vec::new()));
    let guard = CaptureGuard;
    let result = run();
    let bytes = CAPTURE
        .with(|slot| slot.borrow_mut().take())
        .unwrap_or_default();
    drop(guard);
    (result, String::from_utf8(bytes).unwrap())
}
