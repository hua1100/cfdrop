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
    let block = r###"<style>
.cfdrop-comment-button{position:fixed;right:16px;bottom:16px;z-index:10000;min-height:44px;padding:0 16px;border:1px solid #9f7d2e;border-radius:999px;background:#ffb43f;color:#24221f;font:700 14px/1.2 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;box-shadow:0 10px 30px rgba(70,49,24,.18);cursor:pointer}
.cfdrop-comment-panel{position:fixed;top:0;right:0;bottom:0;z-index:9999;display:none;width:min(420px,100vw);height:100vh;overflow:auto;box-sizing:border-box;padding:16px 16px 72px;border:0;border-left:1px solid #d9cdbb;border-radius:0;background:#fffaf1;color:#24221f;box-shadow:-18px 0 44px rgba(70,49,24,.16);font:14px/1.4 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
.cfdrop-comment-panel[data-open="true"]{display:flex;flex-direction:column;gap:12px}
.cfdrop-comment-panel *{box-sizing:border-box}
.cfdrop-comment-panel label{display:flex;flex-direction:column;gap:4px;font-weight:650}
.cfdrop-comment-panel input,.cfdrop-comment-panel textarea{width:100%;border:1px solid #d6c7b1;border-radius:6px;background:#fff;color:#24221f;font:inherit}
.cfdrop-comment-panel input{min-height:36px;padding:7px 9px}
.cfdrop-comment-panel textarea{min-height:108px;resize:vertical;padding:8px 9px}
.cfdrop-comment-submit{min-height:40px;border:1px solid #1f5e4d;border-radius:6px;background:#23745e;color:#fff;font-weight:750;cursor:pointer}
.cfdrop-comment-submit:disabled{cursor:not-allowed;opacity:.62}
.cfdrop-comment-status{min-height:20px;margin:0;color:#684c1f}
.cfdrop-comment-preview{display:none;margin:0;padding:8px;border-left:3px solid #ffb43f;background:#fff4d8;color:#4c4031;white-space:pre-wrap;overflow-wrap:anywhere}
.cfdrop-comment-preview[data-visible="true"]{display:block}
.cfdrop-comment-list{display:flex;flex-direction:column;gap:8px;margin:0;padding:0;list-style:none}
.cfdrop-comment-item{padding:10px;border:1px solid #e1d4c0;border-radius:8px;background:#fffdf8}
.cfdrop-comment-meta{display:flex;justify-content:space-between;gap:8px;flex-wrap:wrap;min-width:0;margin-bottom:4px;color:#66594b;font-size:12px;overflow-wrap:anywhere}
.cfdrop-comment-meta>*{min-width:0;overflow-wrap:anywhere}
.cfdrop-comment-body{margin:0;white-space:pre-wrap;overflow-wrap:anywhere}
@media (max-width: 640px){
  .cfdrop-comment-button{right:12px;bottom:12px}
  .cfdrop-comment-panel{top:auto;left:0;right:0;bottom:0;width:100vw;height:auto;max-height:82vh;border-right:0;border-bottom:0;border-left:0;border-radius:10px 10px 0 0;padding:16px 16px 72px}
}
</style>
<script>
window.CFDROP_REPORT_ID = __REPORT_ID__;
(function(){
  let selectedText = '';
  let commentsCache = [];
  const button = document.createElement('button');
  button.className = 'cfdrop-comment-button';
  button.type = 'button';
  button.textContent = '註解';

  const panel = document.createElement('form');
  panel.className = 'cfdrop-comment-panel';

  const authorLabel = document.createElement('label');
  authorLabel.textContent = '名字';
  const authorInput = document.createElement('input');
  authorInput.name = 'author';
  authorInput.autocomplete = 'name';
  authorInput.maxLength = 80;
  authorLabel.appendChild(authorInput);

  const preview = document.createElement('p');
  preview.className = 'cfdrop-comment-preview';

  const bodyLabel = document.createElement('label');
  bodyLabel.textContent = '註解';
  const bodyInput = document.createElement('textarea');
  bodyInput.name = 'body';
  bodyInput.required = true;
  bodyInput.maxLength = 4000;
  bodyLabel.appendChild(bodyInput);

  const submit = document.createElement('button');
  submit.className = 'cfdrop-comment-submit';
  submit.type = 'submit';
  submit.textContent = '送出';

  const status = document.createElement('p');
  status.className = 'cfdrop-comment-status';
  status.setAttribute('role', 'status');

  const list = document.createElement('ul');
  list.className = 'cfdrop-comment-list';

  panel.appendChild(authorLabel);
  panel.appendChild(preview);
  panel.appendChild(bodyLabel);
  panel.appendChild(submit);
  panel.appendChild(status);
  panel.appendChild(list);

  function setStatus(message) {
    status.textContent = message;
  }

  function updatePreview() {
    preview.textContent = selectedText ? '選取文字: ' + selectedText : '';
    preview.dataset.visible = selectedText ? 'true' : 'false';
  }

  function renderComments(comments) {
    commentsCache = comments;
    list.replaceChildren();
    comments.forEach((comment) => {
      const item = document.createElement('li');
      item.className = 'cfdrop-comment-item';

      const meta = document.createElement('div');
      meta.className = 'cfdrop-comment-meta';
      const author = document.createElement('span');
      author.textContent = comment.author || 'Anonymous';
      const created = document.createElement('time');
      created.dateTime = comment.created_at || '';
      created.textContent = comment.created_at ? new Date(comment.created_at).toLocaleString() : '';
      meta.appendChild(author);
      meta.appendChild(created);

      if (comment.anchor_text) {
        const anchor = document.createElement('p');
        anchor.className = 'cfdrop-comment-preview';
        anchor.dataset.visible = 'true';
        anchor.textContent = '選取文字: ' + comment.anchor_text;
        item.appendChild(anchor);
      }

      const body = document.createElement('p');
      body.className = 'cfdrop-comment-body';
      body.textContent = comment.body || '';

      item.appendChild(meta);
      item.appendChild(body);
      list.appendChild(item);
    });
  }

  async function loadComments() {
    try {
      const resp = await fetch('/api/comments', { headers: { 'Accept': 'application/json' } });
      if (!resp.ok) {
        setStatus('註解功能已過期或暫時無法載入');
        return;
      }
      const data = await resp.json();
      renderComments(Array.isArray(data.comments) ? data.comments : []);
    } catch (err) {
      setStatus('註解功能暫時無法載入');
    }
  }

  button.addEventListener('click', () => {
    const opening = panel.dataset.open !== 'true';
    if (opening) {
      selectedText = String(window.getSelection ? window.getSelection().toString() : '').trim();
      updatePreview();
    }
    panel.dataset.open = opening ? 'true' : 'false';
  });

  panel.addEventListener('submit', async (event) => {
    event.preventDefault();
    const body = bodyInput.value.trim();
    if (!body) return;
    submit.disabled = true;
    setStatus('送出中');
    try {
      const resp = await fetch('/api/comments', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
        body: JSON.stringify({
          author: authorInput.value.trim(),
          body,
          anchor_text: selectedText,
          path: location.pathname
        })
      });
      if (!resp.ok) {
        setStatus('註解功能已過期或送出失敗');
        return;
      }
      const data = await resp.json();
      if (data.comment) {
        renderComments(commentsCache.concat([data.comment]));
      } else {
        await loadComments();
      }
      bodyInput.value = '';
      selectedText = '';
      updatePreview();
      setStatus('已送出');
    } catch (err) {
      setStatus('註解功能已過期或送出失敗');
    } finally {
      submit.disabled = false;
    }
  });

  function mount() {
    document.body.appendChild(button);
    document.body.appendChild(panel);
    loadComments();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mount);
  } else {
    mount();
  }
})();
</script>"###
    .replace("__REPORT_ID__", &escaped);
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
        assert!(html.contains("renderComments"));
        assert!(html.contains("window.getSelection().toString()"));
        assert!(html.contains("submit.disabled = true"));
        assert!(html.contains("document.readyState"));
        assert!(html.contains("textContent = comment.body"));
        assert!(html.contains("top:0;right:0;bottom:0"));
        assert!(html.contains("height:100vh"));
        assert!(html.contains("@media (max-width: 640px)"));
        assert!(html.contains("top:auto;left:0;right:0;bottom:0"));
        assert!(html.contains("flex-wrap:wrap;min-width:0"));
        assert!(html.contains(".cfdrop-comment-meta>*{min-width:0;overflow-wrap:anywhere}"));
        assert!(!html.contains("innerHTML"));
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
