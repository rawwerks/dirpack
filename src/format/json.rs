use crate::format::Formatter;

pub struct JsonFormatter {
    output: String,
}

impl JsonFormatter {
    pub fn new(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
        }
    }
}

impl Formatter for JsonFormatter {
    fn format(&self) -> String {
        self.output.clone()
    }
}
