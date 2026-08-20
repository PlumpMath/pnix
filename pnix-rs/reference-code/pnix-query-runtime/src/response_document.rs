use crate::KernelResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseDocumentProjection<'a> {
  pub native_text: &'a str,
  pub response_document_px: &'a str,
  pub fragment_found: bool,
  pub fragment_kind: Option<&'a str>,
  pub fragment_visibility: Option<&'a str>,
  pub fragment_content_org: Option<&'a str>,
  pub fragment_content_px: Option<&'a str>,
  pub fragment_content_html: Option<&'a str>,
  pub fragment_content_speech: Option<&'a str>,
}

pub fn response_document_projection(response: &KernelResponse) -> ResponseDocumentProjection<'_> {
  let fragment = response
    .output_fragments
    .iter()
    .find(|fragment| fragment.kind == "response-document");

  ResponseDocumentProjection {
    native_text: response.response_document_org.as_str(),
    response_document_px: response.response_document_px.as_str(),
    fragment_found: fragment.is_some(),
    fragment_kind: fragment.map(|fragment| fragment.kind.as_str()),
    fragment_visibility: fragment.map(|fragment| fragment.visibility.as_str()),
    fragment_content_org: fragment.map(|fragment| fragment.content_org.as_str()),
    fragment_content_px: fragment.and_then(|fragment| fragment.content_px.as_deref()),
    fragment_content_html: fragment.and_then(|fragment| fragment.content_html.as_deref()),
    fragment_content_speech: fragment.and_then(|fragment| fragment.content_speech.as_deref()),
  }
}

pub fn response_document_native_text(response: &KernelResponse) -> &str {
  response.response_document_org.as_str()
}

pub fn response_document_html(native_text: &str) -> String {
  format!(
    r#"<article class="pnix-response-document" data-pnix-response-document="true"><pre>{}</pre></article>"#,
    escape_html(native_text)
  )
}

pub fn response_document_speech_text(native_text: &str) -> String {
  native_text
    .lines()
    .filter_map(|line| {
      let line = line.trim();
      if line.is_empty() {
        None
      } else {
        Some(
          line
            .strip_prefix("* ")
            .or_else(|| line.strip_prefix("** "))
            .or_else(|| line.strip_prefix("- "))
            .unwrap_or(line)
            .trim_matches('~')
            .to_string(),
        )
      }
    })
    .collect::<Vec<_>>()
    .join(" ")
}

fn escape_html(value: &str) -> String {
  value
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
}
