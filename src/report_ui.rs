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
.cfdrop-selection-comment-button{position:fixed;display:none;z-index:10001;min-height:38px;padding:0 12px;border:1px solid #9f7d2e;border-radius:999px;background:#ffb43f;color:#24221f;font:700 13px/1.2 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;box-shadow:0 10px 30px rgba(70,49,24,.18);cursor:pointer}
.cfdrop-selection-comment-button[data-visible="true"]{display:block}
.cfdrop-block-comment-button{position:fixed;display:none;z-index:10000;min-height:30px;padding:0 10px;border:1px solid #c69a45;border-radius:999px;background:#fff4d8;color:#24221f;font:700 12px/1.2 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;box-shadow:0 8px 22px rgba(70,49,24,.14);cursor:pointer}
.cfdrop-block-comment-button[data-visible="true"]{display:block}
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
.cfdrop-comment-item[data-active="true"]{border-color:#ffb43f;box-shadow:0 0 0 2px rgba(255,180,63,.28)}
.cfdrop-comment-meta{display:flex;justify-content:space-between;gap:8px;flex-wrap:wrap;min-width:0;margin-bottom:4px;color:#66594b;font-size:12px;overflow-wrap:anywhere}
.cfdrop-comment-meta>*{min-width:0;overflow-wrap:anywhere}
.cfdrop-comment-body{margin:0;white-space:pre-wrap;overflow-wrap:anywhere}
.cfdrop-comment-jump{min-height:30px;margin-top:8px;border:1px solid #d6c7b1;border-radius:6px;background:#fff4d8;color:#24221f;font:700 12px/1.2 system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;cursor:pointer}
.cfdrop-comment-anchor-highlight{border-bottom:2px solid #ffb43f;background:#fff4d8;color:inherit;cursor:pointer}
.cfdrop-comment-anchor-highlight[data-active="true"],.cfdrop-comment-block-marked[data-active="true"]{outline:2px solid #ffb43f;outline-offset:3px}
.cfdrop-comment-block-marked{box-shadow:inset 4px 0 0 rgba(255,180,63,.84)}
@media (max-width: 640px){
  .cfdrop-comment-button{right:12px;bottom:12px}
  .cfdrop-comment-panel{top:auto;left:0;right:0;bottom:0;width:100vw;height:auto;max-height:82vh;border-right:0;border-bottom:0;border-left:0;border-radius:10px 10px 0 0;padding:16px 16px 72px}
}
</style>
<script>
window.CFDROP_REPORT_ID = __REPORT_ID__;
(function(){
  const commentableSelector = 'section[id],h1,h2,h3,p,.card,.panel,.callout,.metric,.table-wrap,table,.code-wrap,pre,figure,.sample';
  let activeAnchor = null;
  let activeBlock = null;
  let commentsCache = [];
  let blockSequence = 1;
  const button = document.createElement('button');
  button.className = 'cfdrop-comment-button';
  button.type = 'button';
  button.textContent = '註解';
  button.setAttribute('aria-label', '新增頁面或區塊註解');

  const selectionButton = document.createElement('button');
  selectionButton.className = 'cfdrop-selection-comment-button';
  selectionButton.type = 'button';
  selectionButton.textContent = '註解';
  selectionButton.setAttribute('aria-label', '針對選取文字新增註解');

  const blockButton = document.createElement('button');
  blockButton.className = 'cfdrop-block-comment-button';
  blockButton.type = 'button';
  blockButton.textContent = '區塊註解';
  blockButton.setAttribute('aria-label', '針對目前區塊新增註解');

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
    const text = activeAnchor && activeAnchor.anchor_text ? activeAnchor.anchor_text : '';
    const label = activeAnchor && activeAnchor.block_label ? activeAnchor.block_label : '';
    preview.textContent = text ? '選取文字: ' + text : (label ? '區塊: ' + label : '');
    preview.dataset.visible = text || label ? 'true' : 'false';
  }

  function cssEscape(value) {
    if (window.CSS && typeof window.CSS.escape === 'function') return window.CSS.escape(value);
    return String(value).replace(/[^a-zA-Z0-9_-]/g, '\\\\$&');
  }

  function ensureBlockIds() {
    document.querySelectorAll(commentableSelector).forEach((block) => {
      if (block.closest('.cfdrop-comment-panel')) return;
      if (!block.id && !block.dataset.cfdropBlockId) {
        block.dataset.cfdropBlockId = 'blk_' + String(blockSequence++).padStart(4, '0');
      }
    });
  }

  function closestCommentable(node) {
    const element = node && node.nodeType === Node.ELEMENT_NODE ? node : node && node.parentElement;
    if (!element || element.closest('.cfdrop-comment-panel') || element.closest('.cfdrop-comment-button,.cfdrop-selection-comment-button,.cfdrop-block-comment-button')) return null;
    return element.closest(commentableSelector);
  }

  function selectorForBlock(block) {
    if (!block) return '';
    if (block.id) return '#' + cssEscape(block.id);
    if (!block.dataset.cfdropBlockId) {
      block.dataset.cfdropBlockId = 'blk_' + String(blockSequence++).padStart(4, '0');
    }
    return '[data-cfdrop-block-id="' + block.dataset.cfdropBlockId + '"]';
  }

  function blockKind(block) {
    if (!block) return '';
    if (block.tagName) return block.tagName.toLowerCase();
    return 'block';
  }

  function blockLabel(block) {
    if (!block) return '';
    const heading = block.matches('section') ? block.querySelector('h1,h2,h3') : null;
    const text = (heading || block).textContent || '';
    return text.trim().replace(/\\s+/g, ' ').slice(0, 160);
  }

  function pathFrom(root, node) {
    const path = [];
    let current = node;
    while (current && current !== root) {
      const parent = current.parentNode;
      if (!parent) return null;
      path.unshift(Array.prototype.indexOf.call(parent.childNodes, current));
      current = parent;
    }
    return current === root ? path : null;
  }

  function rangeFromBlock(block, range) {
    const startPath = pathFrom(block, range.startContainer);
    const endPath = pathFrom(block, range.endContainer);
    if (!startPath || !endPath) return null;
    return {
      start_path: startPath,
      start_offset: range.startOffset,
      end_path: endPath,
      end_offset: range.endOffset
    };
  }

  function quoteContext(block, text) {
    const fullText = (block && block.textContent ? block.textContent : '').replace(/\\s+/g, ' ');
    const needle = String(text || '').replace(/\\s+/g, ' ');
    const index = needle ? fullText.indexOf(needle) : -1;
    if (index < 0) return { before: '', after: '' };
    return {
      before: fullText.slice(Math.max(0, index - 80), index).trim(),
      after: fullText.slice(index + needle.length, index + needle.length + 80).trim()
    };
  }

  function buildAnchorFromBlock(block) {
    if (!block) return null;
    return {
      anchor_text: '',
      selector: selectorForBlock(block),
      path: location.pathname,
      quote_context_before: '',
      quote_context_after: '',
      anchor_version: 1,
      range: null,
      block_label: blockLabel(block),
      block_kind: blockKind(block)
    };
  }

  function buildAnchorFromSelection() {
    if (!window.getSelection || !window.getSelection().rangeCount) return null;
    const selection = window.getSelection();
    const text = String(window.getSelection().toString()).trim();
    if (!text || selection.isCollapsed) return null;
    const range = selection.getRangeAt(0).cloneRange();
    const block = closestCommentable(range.commonAncestorContainer);
    if (!block) return null;
    const context = quoteContext(block, text);
    const rect = range.getBoundingClientRect();
    return {
      anchor_text: text,
      selector: selectorForBlock(block),
      path: location.pathname,
      quote_context_before: context.before,
      quote_context_after: context.after,
      anchor_version: 1,
      range: rangeFromBlock(block, range),
      block_label: blockLabel(block),
      block_kind: blockKind(block),
      rect: { top: rect.top, right: rect.right, bottom: rect.bottom, left: rect.left }
    };
  }

  function positionButtonNear(buttonNode, rect) {
    const margin = 8;
    const width = 84;
    const left = Math.max(margin, Math.min(window.innerWidth - width - margin, rect.left));
    const top = Math.max(margin, Math.min(window.innerHeight - 44, rect.bottom + margin));
    buttonNode.style.left = left + 'px';
    buttonNode.style.top = top + 'px';
    buttonNode.style.right = 'auto';
    buttonNode.style.bottom = 'auto';
  }

  function updateSelectionAction() {
    const anchor = buildAnchorFromSelection();
    if (!anchor || !anchor.rect) {
      selectionButton.dataset.visible = 'false';
      return;
    }
    activeAnchor = anchor;
    positionButtonNear(selectionButton, anchor.rect);
    selectionButton.dataset.visible = 'true';
  }

  function updateBlockAction(block) {
    activeBlock = block;
    if (!block || panel.dataset.open === 'true') {
      blockButton.dataset.visible = 'false';
      return;
    }
    const rect = block.getBoundingClientRect();
    positionButtonNear(blockButton, { left: rect.right - 96, bottom: rect.top });
    blockButton.dataset.visible = 'true';
  }

  function openPanel(anchor) {
    activeAnchor = anchor || buildAnchorFromSelection() || buildAnchorFromBlock(activeBlock);
    updatePreview();
    selectionButton.dataset.visible = 'false';
    blockButton.dataset.visible = 'false';
    panel.dataset.open = 'true';
    bodyInput.focus();
  }

  function parseRangeJson(comment) {
    if (comment.range && typeof comment.range === 'object') return comment.range;
    if (!comment.range_json) return null;
    try {
      const parsed = JSON.parse(comment.range_json);
      return parsed && typeof parsed === 'object' ? parsed : null;
    } catch (err) {
      return null;
    }
  }

  function nodeFromPath(root, path) {
    let current = root;
    for (const index of Array.isArray(path) ? path : []) {
      current = current && current.childNodes ? current.childNodes[index] : null;
      if (!current) return null;
    }
    return current;
  }

  function highlightRange(block, rangeData, commentId) {
    if (!rangeData) return false;
    const start = nodeFromPath(block, rangeData.start_path);
    const end = nodeFromPath(block, rangeData.end_path);
    if (!start || !end) return false;
    try {
      const range = document.createRange();
      range.setStart(start, rangeData.start_offset);
      range.setEnd(end, rangeData.end_offset);
      const span = document.createElement('span');
      span.className = 'cfdrop-comment-anchor-highlight';
      span.dataset.cfdropCommentId = commentId;
      range.surroundContents(span);
      return true;
    } catch (err) {
      return false;
    }
  }

  function highlightText(block, text, commentId) {
    if (!block || !text) return false;
    const walker = document.createTreeWalker(block, NodeFilter.SHOW_TEXT, {
      acceptNode(node) {
        if (!node.nodeValue || !node.nodeValue.includes(text)) return NodeFilter.FILTER_REJECT;
        if (node.parentElement && node.parentElement.closest('.cfdrop-comment-anchor-highlight')) return NodeFilter.FILTER_REJECT;
        return NodeFilter.FILTER_ACCEPT;
      }
    });
    const node = walker.nextNode();
    if (!node) return false;
    const index = node.nodeValue.indexOf(text);
    const before = node.nodeValue.slice(0, index);
    const after = node.nodeValue.slice(index + text.length);
    const mark = document.createElement('span');
    mark.className = 'cfdrop-comment-anchor-highlight';
    mark.dataset.cfdropCommentId = commentId;
    mark.textContent = text;
    node.parentNode.insertBefore(document.createTextNode(before), node);
    node.parentNode.insertBefore(mark, node);
    node.parentNode.insertBefore(document.createTextNode(after), node);
    node.remove();
    return true;
  }

  function findBlockForComment(comment) {
    if (comment.selector) {
      try {
        const block = document.querySelector(comment.selector);
        if (block) return block;
      } catch (err) {}
    }
    if (comment.anchor_text) {
      return Array.from(document.querySelectorAll(commentableSelector)).find((block) => (block.textContent || '').includes(comment.anchor_text)) || null;
    }
    return null;
  }

  function setActiveComment(id) {
    document.querySelectorAll('[data-cfdrop-comment-id]').forEach((node) => {
      node.dataset.active = node.dataset.cfdropCommentId === id ? 'true' : 'false';
    });
    list.querySelectorAll('[data-cfdrop-comment-card]').forEach((node) => {
      node.dataset.active = node.dataset.cfdropCommentCard === id ? 'true' : 'false';
    });
    const card = list.querySelector('[data-cfdrop-comment-card="' + id + '"]');
    if (card) card.dataset.active = 'true';
  }

  function highlightComment(comment) {
    const block = findBlockForComment(comment);
    if (!block) return;
    block.classList.add('cfdrop-comment-block-marked');
    block.dataset.cfdropCommentId = comment.id || '';
    const highlighted = highlightRange(block, parseRangeJson(comment), comment.id) || highlightText(block, comment.anchor_text, comment.id);
    if (!highlighted && comment.id) {
      block.dataset.cfdropCommentId = comment.id;
    }
  }

  function renderComments(comments) {
    commentsCache = comments;
    list.replaceChildren();
    comments.forEach((comment) => {
      const item = document.createElement('li');
      item.className = 'cfdrop-comment-item';
      item.dataset.cfdropCommentCard = comment.id || '';

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
      if (comment.block_label || comment.block_kind) {
        const blockMeta = document.createElement('p');
        blockMeta.className = 'cfdrop-comment-preview';
        blockMeta.dataset.visible = 'true';
        blockMeta.textContent = '區塊: ' + (comment.block_kind || 'block') + ' / ' + (comment.block_label || '');
        item.appendChild(blockMeta);
      }

      const body = document.createElement('p');
      body.className = 'cfdrop-comment-body';
      body.textContent = comment.body || '';

      const jump = document.createElement('button');
      jump.className = 'cfdrop-comment-jump';
      jump.type = 'button';
      jump.textContent = '跳到原文';
      jump.addEventListener('click', () => {
        const block = findBlockForComment(comment);
        if (block) {
          block.scrollIntoView({ behavior: 'smooth', block: 'center' });
          setActiveComment(comment.id || '');
        }
      });

      item.appendChild(meta);
      item.appendChild(body);
      item.appendChild(jump);
      list.appendChild(item);
    });
    requestAnimationFrame(() => comments.forEach((comment) => highlightComment(comment)));
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
      openPanel(buildAnchorFromSelection() || buildAnchorFromBlock(activeBlock));
    } else {
      panel.dataset.open = 'false';
    }
  });

  selectionButton.addEventListener('pointerdown', (event) => event.preventDefault());
  selectionButton.addEventListener('click', () => openPanel(activeAnchor || buildAnchorFromSelection()));
  blockButton.addEventListener('pointerdown', (event) => event.preventDefault());
  blockButton.addEventListener('click', () => openPanel(buildAnchorFromBlock(activeBlock)));

  panel.addEventListener('submit', async (event) => {
    event.preventDefault();
    const body = bodyInput.value.trim();
    if (!body) return;
    const anchor = activeAnchor || buildAnchorFromSelection() || buildAnchorFromBlock(activeBlock) || {};
    submit.disabled = true;
    setStatus('送出中');
    try {
      const resp = await fetch('/api/comments', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
        body: JSON.stringify({
          author: authorInput.value.trim(),
          body,
          anchor_text: anchor.anchor_text || '',
          selector: anchor.selector || '',
          path: location.pathname,
          quote_context_before: anchor.quote_context_before || '',
          quote_context_after: anchor.quote_context_after || '',
          anchor_version: 1,
          range: anchor.range,
          block_label: anchor.block_label || '',
          block_kind: anchor.block_kind || ''
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
      activeAnchor = null;
      updatePreview();
      setStatus('已送出');
    } catch (err) {
      setStatus('註解功能已過期或送出失敗');
    } finally {
      submit.disabled = false;
    }
  });

  function mount() {
    ensureBlockIds();
    document.body.appendChild(button);
    document.body.appendChild(selectionButton);
    document.body.appendChild(blockButton);
    document.body.appendChild(panel);
    document.addEventListener('selectionchange', updateSelectionAction);
    document.addEventListener('keyup', updateSelectionAction);
    document.addEventListener('mouseup', updateSelectionAction);
    document.addEventListener('touchend', () => setTimeout(updateSelectionAction, 60));
    document.addEventListener('pointerover', (event) => updateBlockAction(closestCommentable(event.target)));
    document.addEventListener('focusin', (event) => updateBlockAction(closestCommentable(event.target)));
    document.addEventListener('scroll', () => {
      selectionButton.dataset.visible = 'false';
      blockButton.dataset.visible = 'false';
    }, { passive: true });
    document.addEventListener('click', (event) => {
      const anchor = event.target.closest && event.target.closest('.cfdrop-comment-anchor-highlight,.cfdrop-comment-block-marked');
      if (anchor && anchor.dataset.cfdropCommentId) {
        panel.dataset.open = 'true';
        setActiveComment(anchor.dataset.cfdropCommentId);
      }
    });
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
    html.match_indices('<')
        .filter_map(|(index, _)| {
            html.get(index..index + "</body>".len())
                .filter(|tag| tag.eq_ignore_ascii_case("</body>"))
                .map(|_| index)
        })
        .next_back()
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

    #[test]
    fn injected_ui_supports_text_anchored_comments() {
        let source = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("index.html"),
            "<!doctype html><html><body><main><p>Alpha beta gamma</p></main></body></html>",
        )
        .unwrap();

        let staged = inject_report_ui(source.path(), "demo").unwrap();
        let html = fs::read_to_string(staged.path().join("index.html")).unwrap();
        assert!(html.contains("cfdrop-selection-comment-button"));
        assert!(html.contains("document.addEventListener('selectionchange'"));
        assert!(html.contains("function buildAnchorFromSelection()"));
        assert!(html.contains("quote_context_before"));
        assert!(html.contains("quote_context_after"));
        assert!(html.contains("anchor_version: 1"));
        assert!(html.contains("range: anchor.range"));
    }

    #[test]
    fn injected_ui_supports_block_anchors_and_highlights() {
        let source = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("index.html"),
            "<!doctype html><html><body><main><section><h2>Summary</h2><p>Alpha beta gamma</p></section></main></body></html>",
        )
        .unwrap();

        let staged = inject_report_ui(source.path(), "demo").unwrap();
        let html = fs::read_to_string(staged.path().join("index.html")).unwrap();
        assert!(html.contains("data-cfdrop-block-id"));
        assert!(html.contains("const commentableSelector"));
        assert!(html.contains("function ensureBlockIds()"));
        assert!(html.contains("function buildAnchorFromBlock"));
        assert!(html.contains("block_label: anchor.block_label"));
        assert!(html.contains("block_kind: anchor.block_kind"));
        assert!(html.contains("cfdrop-comment-anchor-highlight"));
        assert!(html.contains("function highlightComment(comment)"));
        assert!(html.contains("scrollIntoView"));
    }
}
