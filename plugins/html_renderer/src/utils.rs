use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

pub fn format_html_data_url(html: &str) -> String {
    format!(
        "data:text/html,{}",
        utf8_percent_encode(html, NON_ALPHANUMERIC)
    )
}
