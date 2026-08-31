# Anchored Report Comments Spec

**Date:** 2026-08-31  
**Repo:** `hua1100/cfdrop`, fork of `oablab/cfdrop`  
**Status:** Proposal spec for next implementation slice  
**Depends on:** `docs/specs/temporary-report-comments.md`

## Problem

The first temporary report comments implementation lets reviewers leave comments
on a deployed report and lets an agent fetch those comments back as Markdown or
JSON. It also stores optional `anchor_text`, `selector`, and quote context fields.

However, the current browser experience is still closer to a page-level comment
drawer than Google Docs-style inline review:

- Reviewers do not see a floating comment action next to selected text.
- Submitted comments do not visually highlight the exact selected text or block.
- Clicking an existing comment does not jump to the referenced location.
- Mobile text selection can be lost when the reviewer taps the fixed comment
  button, so comments may arrive without an anchor.
- The UI does not produce stable block selectors for reports that lack explicit
  IDs on every paragraph, card, table, heading, or code block.

The next version should make comments visibly anchored to text or report blocks
while preserving the temporary, no-login nature of `cfdrop report deploy`.

## Goals

- Let reviewers select text and attach a comment to that selected range.
- Let reviewers attach a comment to a whole report block when no text is
  selected.
- Show visible highlights or markers on already-commented text and blocks.
- Let reviewers click between a comment card and its highlighted anchor.
- Capture enough location metadata for agents to understand and apply feedback
  after exporting Markdown.
- Work on desktop and mobile, including touch text selection and virtual
  keyboard behavior.
- Keep the existing `/api/comments` contract backward compatible.
- Keep `cfdrop deploy` unchanged and keep anchored comments inside
  `cfdrop report deploy`.

## Non-Goals

- Real-time collaborative editing.
- Persisted Google Docs-style resolved/open thread state.
- User accounts, named permissions, email invites, or ownership transfer beyond
  the existing temporary account claim URL.
- Pixel-perfect restoration of a text range after arbitrary report rewrites.
- Direct browser edits to the deployed static report.
- Long-term hosting after the temporary Cloudflare account expires.

## User Experience

### Text-Anchored Comment

1. Reviewer selects text in the report body.
2. A compact floating `註解` button appears near the selection.
3. Reviewer taps the floating button.
4. The comment composer opens and shows the selected text preview.
5. Reviewer enters optional name and required comment body.
6. On submit, the selected text is highlighted in the report.
7. The new comment appears in the side drawer on desktop or bottom sheet on
   mobile.

### Block-Anchored Comment

1. Reviewer hovers, focuses, or long-presses a commentable block such as a
   heading, paragraph, card, table wrapper, code block, or callout.
2. A small block comment affordance appears.
3. Reviewer opens the composer without selecting text.
4. The comment is attached to the block selector.
5. The block shows a restrained marker, such as an amber left rule or small
   numbered badge.

### Reviewing Existing Comments

- Existing comments load from `GET /api/comments`.
- Each comment card shows author, timestamp, anchor preview, section/block label,
  and comment body.
- Clicking a comment scrolls to the anchor and briefly emphasizes the highlight.
- Clicking a highlight or block marker opens the drawer and focuses the matching
  comment card.
- If the original anchor cannot be found, the comment remains visible in the
  drawer with a `找不到原文位置` status.

## Anchoring Model

Each submitted comment should include three layers of location data. The UI uses
the most precise layer available, then falls back in order.

1. **Range anchor:** precise text range within a text node.
2. **Text quote anchor:** selected text plus before/after context.
3. **Block anchor:** stable selector for the nearest commentable report block.

### Range Anchor

Range anchors are best-effort and only need to work within the current deployed
HTML version.

```json
{
  "anchor_version": 1,
  "range": {
    "start_path": [3, 1, 0],
    "start_offset": 12,
    "end_path": [3, 1, 0],
    "end_offset": 28
  }
}
```

Path arrays are child indexes from a stable block root, not from `document.body`.
This keeps anchors less fragile when the injected comment UI adds extra DOM at
the end of the document.

### Text Quote Anchor

Text quote anchors make exported feedback useful even if the exact DOM range no
longer resolves.

```json
{
  "anchor_text": "PR #1 已在 2026-08-31 合併",
  "quote_context_before": "可留言報告頁。",
  "quote_context_after": "核心測試與 lint 皆通過。"
}
```

Recommended limits:

- `anchor_text`: max 1,000 UTF-8 bytes.
- `quote_context_before`: max 1,000 UTF-8 bytes.
- `quote_context_after`: max 1,000 UTF-8 bytes.

### Block Anchor

Block anchors attach comments to the nearest semantic block. The injected UI may
add generated IDs to staged HTML, but it must not modify the user's source
directory.

```json
{
  "selector": "[data-cfdrop-block-id=\"blk_0037\"]",
  "block_label": "驗證結果",
  "block_kind": "section"
}
```

Commentable block candidates:

- `section[id]`
- `h1`, `h2`, `h3`
- `p`
- `.card`, `.panel`, `.callout`, `.metric`
- `.table-wrap`, `table`
- `.code-wrap`, `pre`
- `figure`, `.sample`

## API Contract

Keep existing endpoints:

```text
GET  /api/health
GET  /api/comments
GET  /api/comments.md
POST /api/comments
```

Extend `POST /api/comments` with optional anchoring fields:

```json
{
  "kind": "comment",
  "author": "Hua",
  "body": "這段可以再精簡。",
  "anchor_text": "臨時報告留言功能已完成",
  "selector": "[data-cfdrop-block-id=\"blk_0002\"]",
  "path": "/",
  "quote_context_before": "Supervisor Review",
  "quote_context_after": "這次開發把 cfdrop",
  "anchor_version": 1,
  "range": {
    "start_path": [0, 1, 0],
    "start_offset": 0,
    "end_path": [0, 1, 0],
    "end_offset": 12
  },
  "block_label": "臨時報告留言功能已完成",
  "block_kind": "h1"
}
```

Backward compatibility:

- Comments without range fields remain valid.
- Existing `anchor_text`, `selector`, `path`, `quote_context_before`, and
  `quote_context_after` fields keep their current meaning.
- Unknown JSON fields continue to be ignored.
- Old deployments can still be exported by newer CLIs as long as
  `/api/comments.md` or `/api/comments` is available.

## Storage Schema

Current D1 schema already includes:

```sql
anchor_text TEXT,
selector TEXT,
path TEXT,
quote_context_before TEXT,
quote_context_after TEXT
```

Add nullable columns for richer anchors:

```sql
anchor_version INTEGER,
range_json TEXT,
block_label TEXT,
block_kind TEXT
```

If adding columns to the current `CREATE TABLE IF NOT EXISTS` statement, new
temporary deployments can create the expanded schema directly. There is no need
to migrate already-expired temporary deployments.

## Markdown Export

Markdown should help an agent apply feedback without opening the browser.

```markdown
## 1. 這段可以再精簡

- ID: cmt_...
- Author: Hua
- Path: /
- Section: [data-cfdrop-block-id="blk_0002"]
- Block: h1 / 臨時報告留言功能已完成
- Anchor: 臨時報告留言功能已完成
- Context Before: Supervisor Review
- Context After: 這次開發把 cfdrop
- Created: 2026-08-31T13:34:41.137Z

> 這段可以再精簡。
```

Rules:

- Escape Markdown metacharacters in every user-controlled field.
- Do not render user comment body as HTML.
- Include block metadata only when present.
- Keep comments ordered by `created_at ASC`.

## UI Requirements

### Selection Capture

- Listen to `selectionchange`, `mouseup`, `keyup`, and touch selection events.
- Ignore selections inside the injected comment panel.
- Trim whitespace but preserve internal newlines.
- Capture the selected range before moving focus into the composer.
- On mobile, open the composer from a floating selection action so tapping the
  fixed bottom button does not clear the selection first.

### Floating Action

- The floating action appears only when a non-empty selection exists inside a
  commentable report block.
- It should be positioned near the range bounding rect.
- It must stay inside the viewport at 390px width.
- It must not cause horizontal page overflow.
- It should disappear when selection is cleared, the user scrolls far away, or
  the composer opens.

### Highlight Rendering

- Render highlights with injected wrapper spans only after comments load.
- Use `textContent` for all user-authored comment content.
- If wrapping a range fails, fall back to a block marker and show a warning in
  the comment card.
- Use warm CIS colors: amber highlight, ink text, no bright red/green/blue.
- Avoid layout shift: highlight spans must not alter line height.

### Block Markers

- Add `data-cfdrop-block-id` to staged HTML for commentable blocks that lack a
  stable `id`.
- Do not write generated IDs back to the source report directory.
- Prefer existing author-supplied IDs in selectors when available.
- Block markers must be keyboard focusable and accessible.

### Drawer Behavior

- Desktop: side drawer with comment list, composer, and focused comment state.
- Mobile: bottom sheet with max height under the viewport and safe bottom
  padding for virtual keyboards.
- Comment cards should include a `跳到原文` action when an anchor is resolvable.
- The active comment should highlight both the card and corresponding page
  anchor.

## Security and Privacy

- Keep the existing Basic Auth behavior for `--auth user:pass`; it must protect
  both static assets and comment APIs.
- Do not expose Cloudflare API tokens or claim URLs in generated HTML, Worker
  responses, comments, or Markdown export.
- Never use `innerHTML` for reviewer-provided fields.
- Enforce existing payload limits and add limits for new fields:
  - `range_json`: max 2,000 UTF-8 bytes.
  - `block_label`: max 300 UTF-8 bytes.
  - `block_kind`: max 80 UTF-8 bytes.
- Continue rejecting non-JSON comment POSTs.

## Accessibility

- Floating and block comment buttons must be reachable by keyboard.
- Buttons must have accessible names that describe whether they comment on the
  selection or block.
- Highlight markers should not rely on color alone; use a subtle border or
  numbered marker where practical.
- Drawer focus should move into the composer when opened, then return to the
  invoking anchor when closed.
- Status updates should use `role="status"` as in the current MVP.

## Implementation Notes

- Keep the code in `src/report_ui.rs` self-contained because the injected UI must
  work in static reports without external bundles.
- Add tests by asserting generated HTML/JS contains the core behaviors and by
  using small fixture HTML for block ID injection.
- Keep `src/report_worker.rs` backward compatible by accepting old comments and
  returning new fields only when present.
- Store complex range data as JSON text in D1 to avoid overfitting the temporary
  storage schema.
- If exact range restoration is unreliable, ship block-level anchoring first but
  keep the API fields reserved.

## Acceptance Criteria

- Selecting text in a deployed report reveals a floating `註解` action near the
  selection on desktop and mobile.
- Submitting a selected-text comment stores `anchor_text`, `selector`,
  `quote_context_before`, `quote_context_after`, and range metadata when
  available.
- Reloading the report fetches comments and highlights anchored text when the
  range can be resolved.
- Clicking a comment scrolls to and emphasizes the relevant text or block.
- Whole-block comments work without selecting text.
- `cfdrop report comments --format md` includes Anchor, Context, and Block lines
  when those fields are present.
- Existing comments from v0.7.0 still display and export correctly.
- At 1440px and 390px widths, the injected UI causes no horizontal page overflow.
- `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` pass.

## Open Questions

- Should block-level affordances always be visible on hover/focus, or should they
  appear only after opening the comment drawer?
- Should the Markdown export group comments by block/section, or keep strict
  chronological order?
- Should anonymous author remain the default, or should the UI remember the last
  entered author name in `localStorage`?
- Should temporary comments support `resolved` state in the next slice, or remain
  append-only until a long-lived backend exists?
