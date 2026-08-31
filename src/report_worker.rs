#![allow(dead_code)]

pub fn comments_worker_script(report_id: &str, auth_token: Option<&str>) -> String {
    let auth = auth_token.unwrap_or("");
    format!(
        r#"const REPORT_ID = {report_id:?};
const AUTH = {auth:?};

export default {{
  async fetch(request, env) {{
    const url = new URL(request.url);
    if (AUTH && request.headers.get("Authorization") !== `Basic ${{AUTH}}`) {{
      return new Response("Unauthorized", {{
        status: 401,
        headers: {{ "WWW-Authenticate": 'Basic realm="cfdrop", charset="UTF-8"' }},
      }});
    }}
    if (url.pathname === "/api/health") {{
      return Response.json({{ ok: true, report_id: REPORT_ID, storage: "memory", version: 1 }});
    }}
    return env.ASSETS.fetch(request);
  }}
}};
"#
    )
}

#[cfg(test)]
mod tests {
    use super::comments_worker_script;

    #[test]
    fn worker_has_health_api_and_assets_fallback() {
        let script = comments_worker_script("review-demo", None);
        assert!(script.contains(r#"const REPORT_ID = "review-demo";"#));
        assert!(script.contains(r#""/api/health""#));
        assert!(script.contains("env.ASSETS.fetch(request)"));
        assert!(!script.contains("claim-preview"));
    }
}
