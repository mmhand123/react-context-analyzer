use context_analyzer_core::model::Span;

pub fn format_placeholder(span: &Span) -> String {
    format!("{}:{}", span.start, span.end)
}
