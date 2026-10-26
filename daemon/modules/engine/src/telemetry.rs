//! 処理時間の計測。
//!
//! 計測する処理は `perf_span!` の span で囲む。daemon の layer が、span が閉じたときに、開いてから閉じるまでの時間と
//! span の field を 1 行のログに出す。計測を無効にしたときは span ができないので、計測の費用はかからない。

/// 計測の span とログの target。
pub const PERFORMANCE_TARGET: &str = "pxna::performance";

/// 計測の span を作る。名前はログの `operation` になる。field は `tracing::debug_span!` と同じ書き方で渡す。
///
/// 結果が後で分かる field は `tracing::field::Empty` で宣言し、`Span::record` で埋める。
/// 非同期の処理は `Instrument::instrument` で、同期の処理は `entered` で囲む。
#[macro_export]
macro_rules! perf_span {
    ($name:literal $($fields:tt)*) => {
        ::tracing::debug_span!(target: $crate::telemetry::PERFORMANCE_TARGET, $name $($fields)*)
    };
}
