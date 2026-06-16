use serde_json::{json, Value};

pub struct Pagination {
    pub limit:  usize,
    pub offset: usize,
}

impl Pagination {
    pub fn from_args(args: &Value) -> Self {
        Self {
            limit:  args["limit"].as_u64().unwrap_or(20) as usize,
            offset: args["offset"].as_u64().unwrap_or(0) as usize,
        }
    }

    /// JSON schema properties fragment — spread into any tool's `inputSchema.properties`.
    pub fn schema() -> Value {
        json!({
            "limit":  { "type": "integer", "description": "Results per page (default 20).",        "default": 20 },
            "offset": { "type": "integer", "description": "Number of results to skip (default 0).", "default": 0  }
        })
    }

    /// Slice `items`, render each with `f`, join with `sep`, append a pagination footer.
    /// Returns the full formatted string, or a "no results" message.
    pub fn render<T, F>(&self, items: &[T], sep: &str, f: F) -> String
    where
        F: Fn(&T) -> String,
    {
        let total = items.len();
        let page: Vec<_> = items.iter().skip(self.offset).take(self.limit).collect();

        if page.is_empty() {
            return if total == 0 {
                String::from("No results found.")
            } else {
                format!("No results at offset {} (total: {total}).", self.offset)
            };
        }

        let body   = page.iter().map(|i| f(i)).collect::<Vec<_>>().join(sep);
        let end    = self.offset + page.len();
        let pages  = total.div_ceil(self.limit);
        let cur    = self.offset / self.limit + 1;

        let footer = if end < total {
            format!(
                "\n\n(Page {cur}/{pages} · {}-{} of {total} · offset={end} for next page)",
                self.offset + 1, end
            )
        } else {
            format!("\n\n(Page {cur}/{pages} · {}-{end} of {total})", self.offset + 1)
        };

        format!("{body}{footer}")
    }
}
