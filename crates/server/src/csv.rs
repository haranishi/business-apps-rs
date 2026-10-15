// SPDX-License-Identifier: Apache-2.0
use crate::AppError;
use serde_json::Value;

pub(crate) const MAX_ROWS: usize = 10_000;
const MAX_BYTES: usize = 16 * 1024 * 1024;

pub(crate) struct Csv {
    output: String,
    limit: usize,
}

impl Csv {
    pub(crate) fn new() -> Self {
        Self {
            output: "\u{feff}".into(),
            limit: MAX_BYTES,
        }
    }

    pub(crate) fn row<'a>(
        &mut self,
        values: impl IntoIterator<Item = &'a Value>,
    ) -> Result<(), AppError> {
        for (index, value) in values.into_iter().enumerate() {
            if index > 0 {
                self.push(",")?;
            }
            self.push("\"")?;
            match value {
                Value::String(text) => {
                    // Quoting CSV syntax does not stop spreadsheet formula evaluation.
                    // Text and labels are untrusted; typed negative numbers stay numeric.
                    if formula_like(text) {
                        self.push("'")?;
                    }
                    for part in text.split_inclusive('"') {
                        self.push(part)?;
                        if part.ends_with('"') {
                            self.push("\"")?;
                        }
                    }
                }
                Value::Null => {}
                value => self.push(&value.to_string())?,
            }
            self.push("\"")?;
        }
        self.push("\r\n")
    }

    fn push(&mut self, text: &str) -> Result<(), AppError> {
        if text.len() > self.limit.saturating_sub(self.output.len()) {
            return Err(AppError::ExportLimit);
        }
        self.output.push_str(text);
        Ok(())
    }

    pub(crate) fn finish(self) -> String {
        self.output
    }
}

fn formula_like(text: &str) -> bool {
    text.starts_with(['\t', '\r', '\n'])
        || text
            .trim_start_matches(|c: char| c.is_whitespace() || c.is_control() || c == '\u{feff}')
            .starts_with(['=', '+', '-', '@', '＝', '＋', '－', '＠'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn quotes_unicode_multiline_and_typed_values() {
        let mut csv = Csv::new();
        let cells = [
            json!("東京,\"会社\"\r\n二行目"),
            json!(null),
            json!(false),
            json!(0),
            json!(-2.5),
        ];
        csv.row(&cells).unwrap();
        assert_eq!(
            csv.finish(),
            "\u{feff}\"東京,\"\"会社\"\"\r\n二行目\",\"\",\"false\",\"0\",\"-2.5\"\r\n"
        );
    }

    #[test]
    fn neutralizes_formula_prefixes_including_padded_and_control_text() {
        for text in [
            "=1+1",
            "+1",
            "-1",
            "@SUM(A1)",
            " \t=1",
            "\rtext",
            "\ntext",
            "\ttext",
            "\0=1",
            "\u{feff}=1",
            "＝1",
            "＋1",
            "－1",
            "＠SUM(A1)",
        ] {
            let mut csv = Csv::new();
            csv.row(&[json!(text)]).unwrap();
            assert_eq!(csv.finish(), format!("\u{feff}\"'{text}\"\r\n"));
        }
        let mut csv = Csv::new();
        csv.row(&[json!("ordinary = text"), json!(" 日本語")])
            .unwrap();
        assert_eq!(csv.finish(), "\u{feff}\"ordinary = text\",\" 日本語\"\r\n");
    }

    #[test]
    fn byte_limit_counts_bom_quotes_escaping_and_crlf() {
        let mut csv = Csv {
            output: "\u{feff}".into(),
            limit: 10,
        };
        csv.row(&[json!("abc")]).unwrap();
        assert_eq!(csv.finish().len(), 10);
        let mut csv = Csv {
            output: "\u{feff}".into(),
            limit: 9,
        };
        assert!(matches!(
            csv.row(&[json!("abc")]),
            Err(AppError::ExportLimit)
        ));
        let mut csv = Csv {
            output: "\u{feff}".into(),
            limit: 10,
        };
        assert!(matches!(
            csv.row(&[json!("あい")]),
            Err(AppError::ExportLimit)
        ));
    }
}
