use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn inject_report_ui(source: &Path, report_id: &str) -> Result<tempfile::TempDir> {
    let staged = tempfile::tempdir().context("creating report staging dir")?;
    for item in WalkDir::new(source).follow_links(false) {
        let item = item.context("walking report source")?;
        let rel = item
            .path()
            .strip_prefix(source)
            .context("computing report relative path")?;
        if rel
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        {
            continue;
        }
        let target = staged.path().join(rel);
        if item.file_type().is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("creating {}", target.display()))?;
            continue;
        }
        if !item.file_type().is_file() {
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        if item
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("html"))
        {
            let html = fs::read_to_string(item.path())
                .with_context(|| format!("reading {}", item.path().display()))?;
            fs::write(&target, inject_html(&html, report_id))
                .with_context(|| format!("writing {}", target.display()))?;
        } else {
            fs::copy(item.path(), &target)
                .with_context(|| format!("copying {}", item.path().display()))?;
        }
    }
    Ok(staged)
}

fn inject_html(html: &str, report_id: &str) -> String {
    let escaped = serde_json::to_string(report_id).unwrap();
    let block = format!(
        r#"<style>
.cfdrop-comment-button{{position:fixed;right:16px;bottom:16px;z-index:9999;min-height:44px;padding:0 14px;border:1px solid #d9cdbb;border-radius:999px;background:#ffb43f;color:#24221f;font-weight:700}}
.cfdrop-comment-panel{{position:fixed;left:12px;right:12px;bottom:72px;z-index:9999;display:none;padding:12px;border:1px solid #d9cdbb;border-radius:10px;background:#fffaf1;color:#24221f;box-shadow:0 18px 44px rgba(70,49,24,.16)}}
.cfdrop-comment-panel[data-open="true"]{{display:block}}
.cfdrop-comment-panel textarea{{width:100%;min-height:96px}}
</style>
<script>
window.CFDROP_REPORT_ID = {escaped};
(function(){{
  const button = document.createElement('button');
  button.className = 'cfdrop-comment-button';
  button.type = 'button';
  button.textContent = '註解';
  const panel = document.createElement('form');
  panel.className = 'cfdrop-comment-panel';
  panel.innerHTML = '<label>名字 <input name="author" autocomplete="name"></label><label>註解 <textarea name="body" required></textarea></label><button type="submit">送出</button><p role="status"></p>';
  button.addEventListener('click', () => panel.dataset.open = panel.dataset.open === 'true' ? 'false' : 'true');
  panel.addEventListener('submit', async (event) => {{
    event.preventDefault();
    const status = panel.querySelector('[role="status"]');
    const selection = String(window.getSelection ? window.getSelection() : '');
    const body = panel.elements.body.value.trim();
    if (!body) return;
    const resp = await fetch('/api/comments', {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      body: JSON.stringify({{
        author: panel.elements.author.value.trim(),
        body,
        anchor_text: selection,
        path: location.pathname
      }})
    }});
    status.textContent = resp.ok ? '已送出' : '送出失敗';
    if (resp.ok) panel.elements.body.value = '';
  }});
  document.addEventListener('DOMContentLoaded', () => {{
    document.body.appendChild(button);
    document.body.appendChild(panel);
  }});
}})();
</script>"#
    );
    if let Some(index) = find_last_body_close(html) {
        format!("{}{}{}", &html[..index], block, &html[index..])
    } else {
        format!("{html}{block}")
    }
}

fn find_last_body_close(html: &str) -> Option<usize> {
    html.match_indices(|c: char| c == '<')
        .filter_map(|(index, _)| {
            html.get(index..index + "</body>".len())
                .filter(|tag| tag.eq_ignore_ascii_case("</body>"))
                .map(|_| index)
        })
        .last()
}

#[cfg(test)]
mod tests {
    use super::inject_report_ui;
    use std::fs;

    #[test]
    fn injects_ui_into_html_and_copies_assets() {
        let source = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("index.html"),
            "<!doctype html><html><head></head><body><main><p>Hello</p></main></body></html>",
        )
        .unwrap();
        fs::write(
            source.path().join("template.html"),
            r#"<!doctype html><html><body><script>const demo = "</body>";</script><main>Template</main></body></html>"#,
        )
        .unwrap();
        fs::write(
            source.path().join("upper.HTML"),
            "<!doctype html><html><body><main>Upper</main></body></html>",
        )
        .unwrap();
        fs::write(source.path().join("note.txt"), "keep").unwrap();
        fs::create_dir_all(source.path().join("nested/.cache")).unwrap();
        fs::write(source.path().join("nested/.cache/private.txt"), "skip").unwrap();

        let staged = inject_report_ui(source.path(), "demo").unwrap();
        let html = fs::read_to_string(staged.path().join("index.html")).unwrap();
        assert!(html.contains("window.CFDROP_REPORT_ID = \"demo\""));
        assert!(html.contains("/api/comments"));
        assert!(html.contains("cfdrop-comment"));
        assert_eq!(
            fs::read_to_string(staged.path().join("note.txt")).unwrap(),
            "keep"
        );
        assert!(!staged.path().join("nested/.cache").exists());
        assert!(!staged.path().join("nested/.cache/private.txt").exists());

        let template = fs::read_to_string(staged.path().join("template.html")).unwrap();
        assert_eq!(template.matches("window.CFDROP_REPORT_ID").count(), 1);
        let demo_body_close = template.find(r#""</body>""#).unwrap();
        let injected = template.find("window.CFDROP_REPORT_ID").unwrap();
        let final_body_close = template.rfind("</body>").unwrap();
        assert!(demo_body_close < injected);
        assert!(injected < final_body_close);

        let upper = fs::read_to_string(staged.path().join("upper.HTML")).unwrap();
        assert!(upper.contains("window.CFDROP_REPORT_ID = \"demo\""));
    }
}
