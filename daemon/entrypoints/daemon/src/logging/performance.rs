use std::{
    fmt::Write as _,
    io::Write as _,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use serde_json::{Map, Value, json};
use tracing::{
    Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};
use tracing_subscriber::{fmt::MakeWriter, layer::Context, registry::LookupSpan};

use omnius_pxna_engine::telemetry::PERFORMANCE_TARGET;

/// 計測の span（`perf_span!`）が閉じたときに、開いてから閉じるまでの時間と field を 1 行で出す。
///
/// 行の形は fmt layer の event とそろえ、`operation` に span の名前、`elapsed_sec` に経過時間を入れる。
/// 同じ要求から開いた span には同じ `request` を付け、`parent` に外側の計測の span の名前を入れる。
/// 非同期の span の時間は、poll されていない間も含む壁時計の時間である。
pub struct PerformanceLayer<W> {
    writer: W,
    json: bool,
    next_request: AtomicU64,
}

/// span が開いた時刻と、それまでに記録された field。
struct Timing {
    started: Instant,
    request: u64,
    fields: Map<String, Value>,
}

/// span の field を JSON の値として集める。
struct FieldVisitor<'a>(&'a mut Map<String, Value>);

impl Visit for FieldVisitor<'_> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(field.name().to_string(), json!(value));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name().to_string(), json!(value));
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().to_string(), json!(value));
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.insert(field.name().to_string(), json!(value));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_string(), json!(value));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_string(), json!(format!("{value:?}")));
    }
}

impl<W> PerformanceLayer<W> {
    pub fn new(writer: W, json: bool) -> Self {
        Self {
            writer,
            json,
            next_request: AtomicU64::new(1),
        }
    }

    fn line(&self, name: &str, timing: &Timing, parent: Option<&str>) -> String {
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
        let mut fields = Map::new();
        fields.insert("message".to_string(), json!("span closed"));
        fields.insert("operation".to_string(), json!(name));
        fields.insert("elapsed_sec".to_string(), json!(timing.started.elapsed().as_secs_f64()));
        fields.insert("request".to_string(), json!(timing.request));
        if let Some(parent) = parent {
            fields.insert("parent".to_string(), json!(parent));
        }
        for (key, value) in &timing.fields {
            fields.insert(key.clone(), value.clone());
        }

        if self.json {
            let mut line = json!({ "timestamp": timestamp, "level": "DEBUG", "fields": fields, "target": PERFORMANCE_TARGET }).to_string();
            line.push('\n');
            line
        } else {
            let mut line = format!("{timestamp}  DEBUG {PERFORMANCE_TARGET}: span closed");
            for (key, value) in fields.iter().filter(|(key, _)| *key != "message") {
                match value {
                    Value::String(text) => _ = write!(line, " {key}={text}"),
                    other => _ = write!(line, " {key}={other}"),
                }
            }
            line.push('\n');
            line
        }
    }
}

impl<S, W> tracing_subscriber::Layer<S> for PerformanceLayer<W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if attrs.metadata().target() != PERFORMANCE_TARGET {
            return;
        }
        let Some(span) = ctx.span(id) else {
            return;
        };
        let inherited = span.scope().skip(1).find_map(|ancestor| ancestor.extensions().get::<Timing>().map(|timing| timing.request));
        let request = inherited.unwrap_or_else(|| self.next_request.fetch_add(1, Ordering::Relaxed));
        let mut fields = Map::new();
        attrs.record(&mut FieldVisitor(&mut fields));
        span.extensions_mut().insert(Timing {
            started: Instant::now(),
            request,
            fields,
        });
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(timing) = span.extensions_mut().get_mut::<Timing>()
        {
            values.record(&mut FieldVisitor(&mut timing.fields));
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else {
            return;
        };
        let extensions = span.extensions();
        let Some(timing) = extensions.get::<Timing>() else {
            return;
        };
        let parent = span.scope().skip(1).find(|ancestor| ancestor.metadata().target() == PERFORMANCE_TARGET);
        let line = self.line(span.name(), timing, parent.as_ref().map(|parent| parent.name()));
        // ログを書けなくても、計測される処理は止めない。
        _ = self.writer.make_writer().write_all(line.as_bytes());
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::{Mutex, OnceLock};

    use tracing::{Instrument as _, field::Empty};
    use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt as _};

    use omnius_pxna_engine::perf_span;

    use super::*;

    /// 閉じた計測の span の行。テストはプロセスで 1 つの subscriber を共有するので、並行する別のテストの行も入る。
    static CAPTURED: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    struct CaptureWriter;

    impl std::io::Write for CaptureWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            CAPTURED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for CaptureWriter {
        type Writer = CaptureWriter;
        fn make_writer(&'a self) -> Self::Writer {
            CaptureWriter
        }
    }

    /// テストで閉じた計測の span の行を読む。
    ///
    /// subscriber はプロセスで 1 つだけ置く。テストごとに `set_default` すると、並行する別のテストが subscriber のない状態で
    /// 先に通した callsite が無効のまま覚えられ、span が欠ける。span は、最後の参照を落とした thread で閉じることもある。
    pub(crate) struct SpanCapture;

    impl SpanCapture {
        pub(crate) fn start() -> Self {
            static INSTALLED: OnceLock<()> = OnceLock::new();
            INSTALLED.get_or_init(|| {
                let subscriber = tracing_subscriber::registry().with(PerformanceLayer::new(CaptureWriter, true));
                tracing::subscriber::set_global_default(subscriber).expect("no other test sets a global subscriber");
            });
            Self
        }

        /// これまでに閉じたすべての span の field。
        pub(crate) fn fields(&self) -> Vec<Value> {
            let bytes = CAPTURED.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
            String::from_utf8(bytes)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str::<Value>(line).unwrap()["fields"].clone())
                .collect()
        }

        /// `found` が値を返すまで、閉じた span を読み直す。別の thread で閉じる span を待つ。
        pub(crate) async fn wait_for<T>(&self, found: impl Fn(&[Value]) -> Option<T>) -> T {
            for _ in 0..100 {
                if let Some(value) = found(&self.fields()) {
                    return value;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            panic!("the expected spans did not close: {:?}", self.fields());
        }
    }

    #[tokio::test]
    async fn closed_spans_report_time_fields_request_and_parent_test() {
        let capture = SpanCapture::start();

        async {
            let span = tracing::Span::current();
            async { tokio::time::sleep(std::time::Duration::from_millis(20)).await }
                .instrument(perf_span!("layer_test_inner", bytes = 3_u64))
                .await;
            span.record("ok", true);
        }
        .instrument(perf_span!("layer_test_outer", path = "/a.jpg", ok = Empty))
        .await;
        perf_span!("layer_test_other").in_scope(|| ());
        tracing::debug_span!("layer_test_not_measured").in_scope(|| ());

        let named = |fields: &[Value], name: &str| fields.iter().find(|line| line["operation"] == name).cloned();
        let (inner, outer, other) = capture
            .wait_for(|fields| Some((named(fields, "layer_test_inner")?, named(fields, "layer_test_outer")?, named(fields, "layer_test_other")?)))
            .await;
        assert_eq!((inner["parent"].as_str(), inner["bytes"].as_u64()), (Some("layer_test_outer"), Some(3)));
        assert!(inner["elapsed_sec"].as_f64().unwrap() >= 0.02);
        assert_eq!((outer["path"].as_str(), outer["ok"].as_bool()), (Some("/a.jpg"), Some(true)), "後から記録した field も出す");
        assert_eq!(inner["request"], outer["request"], "同じ要求の span は同じ request を持つ");
        assert_ne!(other["request"], outer["request"]);
        assert!(named(&capture.fields(), "layer_test_not_measured").is_none(), "計測の target でない span は出さない");
    }
}
