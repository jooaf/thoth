use anyhow::Result;
use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::cmark;
use serde_json::Value;

pub fn format_markdown(input: &str) -> Result<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(input, options);
    let mut output = String::new();
    cmark(parser, &mut output)?;
    Ok(output)
}

pub fn format_json(input: &str) -> Result<String> {
    let parsed: Value = serde_json::from_str(input)?;
    Ok(serde_json::to_string_pretty(&parsed)?)
}
