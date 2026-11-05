# DrawCraft native format (`.drawcraft`)

Version **1**. UTF-8 JSON, pretty-printed by DrawCraft, maximum **16 MiB** on load.

The format is intentionally boring: it is a direct, readable representation of the document model. Unknown fields are preserved so newer files can be opened, edited, and re-saved without losing data.

## Top level

```json
{
  "version": 1,
  "active_page": "page-1",
  "pages": [ ... ],
  "meta": {}
}
```

| Field | Type | Notes |
| --- | --- | --- |
| `version` | integer | Must be `1`. |
| `active_page` | string | Must match a page `id`. |
| `pages` | array | At least one page; at most 256. |
| `meta` | object | Arbitrary document metadata. |
| *unknown* | any | Preserved round-trip. |

## Page

```json
{
  "id": "page-1",
  "name": "Page 1",
  "shapes": [ ... ],
  "future_page_field": { "kept": true }
}
```

| Field | Type | Notes |
| --- | --- | --- |
| `id` | string | Unique within the document. |
| `name` | string | Display name; up to 512 bytes. |
| `shapes` | array | At most 100,000 records per page. |

## Shape

```json
{
  "id": "shape-1",
  "kind": "rectangle",
  "pos": { "x": 10.0, "y": 20.0 },
  "size": { "x": 120.0, "y": 80.0 },
  "rotation": 0.0,
  "flip_x": false,
  "flip_y": false,
  "style": { ... },
  "text": "Hello",
  "src": null,
  "points": [],
  "locked": false,
  "group_id": null,
  "parent_id": null,
  "label": "",
  "arrow_start": null,
  "arrow_end": null,
  "arrow_style": "straight",
  "arrow_start_head": "none",
  "arrow_end_head": "arrow",
  "z": 0
}
```

| Field | Type | Notes |
| --- | --- | --- |
| `id` | string | Unique within the page. |
| `kind` | string | One of the kinds below. |
| `pos` / `size` | `{x,y}` | Local bounds before rotation. |
| `rotation` | number | Radians, around the shape centre. |
| `flip_x` / `flip_y` | bool | Mirror in local space. |
| `style` | object | See below. |
| `text` | string | Text content for text/sticky/note/frame/arrow. |
| `src` | string or null | Image data URL, path, or URL. |
| `points` | array | Absolute points for lines, polylines, arrows, freehand. |
| `locked` | bool | Locked shapes ignore selection. |
| `group_id` | string or null | Shapes sharing an id move/select together. |
| `parent_id` | string or null | Frame membership; clipped by the frame. |
| `label` | string | Fallback text/label. |
| `arrow_start` / `arrow_end` | string or null | Bound shape ids. |
| `arrow_style` | `straight` / `bent` / `elbow` | Routing. |
| `arrow_start_head` / `arrow_end_head` | `none` / `arrow` / `triangle` / `diamond` / `square` / `circle` | Heads. |
| `z` | integer | Paint order; ties are broken by id. |

At most 100,000 points per shape. Coordinates and sizes must be finite; sizes must be non-negative.

### Style

```json
{
  "stroke": "#262626",
  "fill_color": "#d9c8f5",
  "stroke_width": 2.0,
  "fill": "semi",
  "dash": "solid",
  "opacity": 1.0,
  "font_size": 18.0,
  "font_family": "sans"
}
```

| Field | Values |
| --- | --- |
| `stroke` / `fill_color` | `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, CSS `rgb()`/`rgba()`, or common names. |
| `stroke_width` | `0..=100`. |
| `fill` | `none`, `semi`, `solid`, `pattern`, `full`. |
| `dash` | `none`, `solid`, `dashed`, `dotted`, `hand_drawn`. |
| `opacity` | `0.0..=1.0`. |
| `font_size` | `1..=512`. |
| `font_family` | Free-form string; `sans` by default. |

### Shape kinds

`rectangle`, `ellipse`, `triangle`, `diamond`, `pentagon`, `hexagon`, `octagon`,
`star`, `heart`, `round_rect`, `parallelogram`, `trapezoid`, `arrow_shape`,
`double_arrow`, `cross`, `home_plate`, `chevron`, `cylinder`, `cloud`, `note`,
`text`, `sticky`, `image`, `frame`, `draw`, `highlighter`, `line`, `polyline`,
`arrow`, `laser`, `group`.

`group` records are not normally written; grouping uses `group_id`.

## Validation

`Document::validate` enforces:

- supported format version;
- at least one page and at most 256 pages;
- unique page ids, unique shape ids per page;
- active page exists;
- finite geometry, non-negative sizes, bounded style values;
- max shapes per page and points per shape;
- arrow bindings and frame parents resolve on the same page;
- every shape's unknown data is retained as-is.

A file that fails validation is rejected rather than partially loaded.

## Unknown data

Unknown keys are preserved at every level:

- document-level unknown keys;
- page-level unknown keys;
- shape-level unknown keys;
- style-level unknown keys.

DrawCraft never silently drops records it does not understand.

## Images

Image records store a `src` string. Supported sources:

- `data:image/...;base64,...` (recommended for portability);
- a local path;
- an HTTP(S) URL.

The renderer caps decoded images to 8192×8192 and 32 MiB. The document loader caps the whole file to 16 MiB.

## Container decision

**v1 keeps the single-file UTF-8 JSON container** (format version 1). Reasons visible in the code today:

- readable diffs and easy agent inspection;
- unknown-field preservation is JSON-native at every level;
- one parser surface for load, save, autosave, and fuzz (`format::from_json`);
- no new archive dependency.

Trade-off: base64-embedded images count against the 16 MiB `MAX_BYTES` loader cap.

**v2 zip trigger:** move to a zip container (manifest + asset files) when either (a) typical boards regularly exceed the 16 MiB JSON cap because of embedded images, or (b) we need streaming/partial asset load. Migration rule when that lands: readers accept both containers; writers opt in to zip (default stays JSON until a later release). No migration code ships in this slice.

## Autosave files

Autosaves live under the session config directory (honours `XDG_CONFIG_HOME`, else `APPDATA`, else `~/.config`):

```
<config>/drawcraft/autosave/<key>.drawcraft
<config>/drawcraft/autosave/<key>.drawcraft.meta.json
```

- `<key>` is a stable FNV-1a hash of the canonical source path, or `untitled-<session-id>` for unsaved documents.
- The `.drawcraft` file is a normal v1 document written with `format::write_atomic`.
- The sidecar JSON records:

| Field | Meaning |
| --- | --- |
| `source_path` | Original path, or null for untitled |
| `source_mtime` / `source_len` / `source_hash` | Disk fingerprint at autosave time |
| `revision` | Session edit counter |
| `saved_at` | Unix seconds |
| `session_id` | Process-local session id |

Recovery keeps an autosave when it is newer than the source mtime and its document content differs. Equal content is discarded silently (also clears a stale background write after a successful save).

## Interoperability

`.drawcraft` is the native format. A clean-room `.tldr` reader/writer is planned for v1.1; it is not part of format v1 and no copied tldraw code or assets are used.
