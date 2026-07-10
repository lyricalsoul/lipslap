use std::sync::{Arc, Mutex};
use std::time::Instant;

/// One recorded `timed!`/`timed_async!` call: when it started and how long it
/// took, both relative to the enclosing `with_trace` scope, so a UI can lay
/// them out as a waterfall (position = offset, width = duration) instead of
/// just a bare duration list.
#[derive(Clone)]
pub struct TraceEntry {
    pub label: String,
    pub offset_ms: u128,
    pub duration_ms: u128,
}

struct TraceState {
    start: Instant,
    entries: Vec<TraceEntry>,
}

tokio::task_local! {
    static TRACE: Arc<Mutex<TraceState>>;
}

pub async fn with_trace<T>(f: impl std::future::Future<Output = T>) -> (T, Vec<TraceEntry>) {
    let state = Arc::new(Mutex::new(TraceState {
        start: Instant::now(),
        entries: Vec::new(),
    }));
    let result = TRACE.scope(state.clone(), f).await;
    let entries = std::mem::take(&mut state.lock().unwrap().entries);
    (result, entries)
}

/// A snapshot of the current `with_trace` scope (if any), so it can be
/// carried across a `spawn_blocking` boundary — task-locals don't propagate
/// onto the blocking thread pool on their own.
#[derive(Clone)]
pub struct TraceContext(Option<Arc<Mutex<TraceState>>>);

pub fn capture_trace_context() -> TraceContext {
    TraceContext(TRACE.try_with(|t| t.clone()).ok())
}

/// Re-enters a captured trace scope around sync `f`, so `timed!` calls made
/// on a blocking-pool thread still record into the original request's trace.
pub fn with_trace_context<T>(ctx: TraceContext, f: impl FnOnce() -> T) -> T {
    match ctx.0 {
        Some(state) => TRACE.sync_scope(state, f),
        None => f(),
    }
}

pub fn debug(scope: &str, message: &str) {
    if std::env::var("NODE_ENV").as_deref() == Ok("production") {
        return;
    }
    let now = chrono_like_time();
    eprintln!("{now}  [DEBUG] ({scope}): {message}");
}

fn chrono_like_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

pub fn timed<T>(scope: &str, label: &str, f: impl FnOnce() -> T) -> T {
    if !cfg!(debug_assertions) {
        return f();
    }
    let start = Instant::now();
    let result = f();
    record(scope, label, start);
    result
}

pub async fn timed_async<T>(scope: &str, label: &str, f: impl std::future::Future<Output = T>) -> T {
    if !cfg!(debug_assertions) {
        return f.await;
    }
    let start = Instant::now();
    let result = f.await;
    record(scope, label, start);
    result
}

fn record(scope: &str, label: &str, call_start: Instant) {
    let elapsed_ms = call_start.elapsed().as_millis();
    debug(scope, &format!("{label} took {elapsed_ms}ms"));
    let _ = TRACE.try_with(|t| {
        let mut state = t.lock().unwrap();
        let offset_ms = call_start.duration_since(state.start).as_millis();
        state.entries.push(TraceEntry {
            label: format!("{scope}.{label}"),
            offset_ms,
            duration_ms: elapsed_ms,
        });
    });
}

/// `timed!(scope, foo(a, b))`
/// `timed!(scope, "label", foo(a, b))`
#[macro_export]
macro_rules! timed {
    ($scope:expr, $label:expr, $fn:ident($($args:tt)*)) => {
        $crate::logging::timed($scope, $label, || $fn($($args)*))
    };
    ($scope:expr, $fn:ident($($args:tt)*)) => {
        $crate::logging::timed($scope, stringify!($fn), || $fn($($args)*))
    };
}
