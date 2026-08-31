#![allow(dead_code)]

pub fn comments_worker_script(report_id: &str, auth_token: Option<&str>) -> String {
    comments_worker_script_with_storage(report_id, "memory", auth_token)
}

pub fn comments_worker_script_with_storage(
    report_id: &str,
    storage_label: &str,
    auth_token: Option<&str>,
) -> String {
    let auth = auth_token.unwrap_or("");
    let script = r##"const REPORT_ID = __REPORT_ID__;
const AUTH = __AUTH__;
const STORAGE = __STORAGE__;
const MAX_BODY_BYTES = 4000;

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json; charset=utf-8" },
  });
}

async function listComments(env) {
  if (env.DB) {
    const result = await env.DB.prepare(
      "SELECT id, report_id, kind, author, body, anchor_text, selector, path, quote_context_before, quote_context_after, user_agent, ip_hash, created_at FROM comments WHERE report_id = ? ORDER BY created_at ASC"
    ).bind(REPORT_ID).all();
    return result.results || [];
  }
  globalThis.__comments = globalThis.__comments || [];
  return globalThis.__comments;
}

async function saveComment(env, comment) {
  if (env.DB) {
    await env.DB.prepare(
      "INSERT INTO comments (id, report_id, kind, author, body, anchor_text, selector, path, quote_context_before, quote_context_after, user_agent, ip_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    ).bind(
      comment.id,
      REPORT_ID,
      comment.kind,
      comment.author,
      comment.body,
      comment.anchor_text,
      comment.selector,
      comment.path,
      comment.quote_context_before,
      comment.quote_context_after,
      comment.user_agent,
      comment.ip_hash,
      comment.created_at
    ).run();
    return;
  }
  globalThis.__comments = globalThis.__comments || [];
  globalThis.__comments.push(comment);
}

function normalizeText(value) {
  return String(value || "").replace(/\r\n/g, "\n").trim();
}

function escapeMarkdownLine(value) {
  return normalizeText(value).replace(/[\\`*_{}\[\]()#+\-.!|>]/g, "\\$&");
}

function escapeMarkdownInline(value) {
  return escapeMarkdownLine(value).replace(/\n+/g, " ");
}

function bodyToMarkdown(value) {
  const lines = normalizeText(value).split("\n");
  return lines.map((line) => `> ${escapeMarkdownLine(line)}`).join("\n");
}

function commentsToMarkdown(comments) {
  const lines = ["# Report Comments", "", `Report: ${REPORT_ID}`, `Exported: ${new Date().toISOString()}`, ""];
  comments.forEach((comment, index) => {
    const title = escapeMarkdownInline(comment.body).slice(0, 80) || "Comment";
    lines.push(`## ${index + 1}. ${title}`);
    lines.push("");
    lines.push(`- ID: ${escapeMarkdownInline(comment.id)}`);
    lines.push(`- Author: ${escapeMarkdownInline(comment.author || "Anonymous")}`);
    lines.push(`- Path: ${escapeMarkdownInline(comment.path || "/")}`);
    if (comment.selector) lines.push(`- Section: ${escapeMarkdownInline(comment.selector)}`);
    if (comment.anchor_text) lines.push(`- Anchor: ${escapeMarkdownInline(comment.anchor_text)}`);
    lines.push(`- Created: ${escapeMarkdownInline(comment.created_at)}`);
    lines.push("");
    lines.push(bodyToMarkdown(comment.body));
    lines.push("");
  });
  return lines.join("\n");
}

function newCommentId() {
  return `cmt_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const method = request.method;
    if (AUTH && request.headers.get("Authorization") !== `Basic ${AUTH}`) {
      return new Response("Unauthorized", {
        status: 401,
        headers: { "WWW-Authenticate": 'Basic realm="cfdrop", charset="UTF-8"' },
      });
    }

    if (url.pathname === "/api/health" && method === "GET") {
      return json({ ok: true, report_id: REPORT_ID, storage: env.DB ? "d1" : STORAGE, version: 1 });
    }

    if (url.pathname === "/api/comments" && method === "GET") {
      const comments = await listComments(env);
      return json({ ok: true, report_id: REPORT_ID, comments });
    }

    if (url.pathname === "/api/comments.md" && method === "GET") {
      const comments = await listComments(env);
      return new Response(commentsToMarkdown(comments), {
        status: 200,
        headers: { "Content-Type": "text/markdown; charset=utf-8" },
      });
    }

    if (url.pathname === "/api/comments" && method === "POST") {
      const raw = await request.text();
      if (new TextEncoder().encode(raw).length > MAX_BODY_BYTES) {
        return json({ ok: false, error: "body too large" }, 413);
      }

      let input;
      try {
        input = raw ? JSON.parse(raw) : {};
      } catch (err) {
        return json({ ok: false, error: "invalid json" }, 400);
      }
      if (!input || typeof input !== "object" || Array.isArray(input)) {
        return json({ ok: false, error: "invalid comment" }, 400);
      }

      const comment = {
        id: newCommentId(),
        kind: input.kind || "comment",
        author: input.author || "",
        body: input.body || "",
        anchor_text: input.anchor_text || "",
        selector: input.selector || "",
        path: input.path || "/",
        quote_context_before: input.quote_context_before || "",
        quote_context_after: input.quote_context_after || "",
        user_agent: request.headers.get("user-agent") || "",
        ip_hash: "",
        created_at: new Date().toISOString(),
      };
      await saveComment(env, comment);
      return json({ ok: true, comment });
    }

    return env.ASSETS.fetch(request);
  }
};
"##;
    script
        .replace("__REPORT_ID__", &format!("{report_id:?}"))
        .replace("__AUTH__", &format!("{auth:?}"))
        .replace("__STORAGE__", &format!("{storage_label:?}"))
}

#[cfg(test)]
mod tests {
    use super::{comments_worker_script, comments_worker_script_with_storage};

    #[test]
    fn worker_has_health_api_and_assets_fallback() {
        let script = comments_worker_script("review-demo", None);
        assert!(script.contains(r#"const REPORT_ID = "review-demo";"#));
        assert!(script.contains(r#"const STORAGE = "memory";"#));
        assert!(script.contains(r#""/api/health""#));
        assert!(script.contains("env.ASSETS.fetch(request)"));
        assert!(!script.contains("claim-preview"));
    }

    #[test]
    fn worker_has_comments_endpoints_and_markdown_export() {
        let script = comments_worker_script("review-demo", None);
        assert!(script.contains(r#""/api/comments""#));
        assert!(script.contains(r#""/api/comments.md""#));
        assert!(script.contains("method === \"POST\""));
        assert!(script.contains("text/markdown; charset=utf-8"));
    }

    #[test]
    fn worker_uses_d1_when_binding_exists() {
        let script = comments_worker_script_with_storage("review-demo", "d1", None);
        assert!(script.contains(r#"const STORAGE = "d1";"#));
        assert!(script.contains("if (env.DB)"));
        assert!(script.contains(
            "SELECT id, report_id, kind, author, body, anchor_text, selector, path, quote_context_before, quote_context_after, user_agent, ip_hash, created_at"
        ));
        assert!(script.contains("INSERT INTO comments"));
        assert!(script.contains("await listComments(env)"));
        assert!(script.contains("await saveComment(env, comment)"));
    }

    #[test]
    fn worker_markdown_export_escapes_user_controlled_structure() {
        let script = comments_worker_script("review-demo", None);
        assert!(script.contains("function escapeMarkdownInline(value)"));
        assert!(script.contains(".replace(/[\\\\`*_{}\\[\\]()#+\\-.!|>]/g, \"\\\\$&\")"));
        assert!(script.contains("function bodyToMarkdown(value)"));
        assert!(script.contains(
            "return lines.map((line) => `> ${escapeMarkdownLine(line)}`).join(\"\\n\");"
        ));
        assert!(script.contains(
            "lines.push(`- Author: ${escapeMarkdownInline(comment.author || \"Anonymous\")}`);"
        ));
        assert!(script.contains("lines.push(`- ID: ${escapeMarkdownInline(comment.id)}`);"));
    }
}
