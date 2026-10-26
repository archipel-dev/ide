# nexus-http

A clean-room, Insomnia-class API client that ships as a first-party Archipel IDE plugin
bundled by default (Tools › HTTP Client, command `nexus.open`) and also runs standalone (`archipel-ide --app dev.archipel.nexus-http`); `.http`/`.rest` files open in its editor. Tracking issue: #49.

| Crate | Purpose |
|---|---|
| `.` (`nexus-http`) | egui views: sidebar, request editor, response viewer, environments, request switcher, import/export, code generation. Renders into any `egui::Ui` and follows the host theme. |
| `core` (`nexus-http-core`) | Headless engine with no UI dependency: data model, git-friendly `.nexus/` storage, `{{ variables }}` and `{% tags %}` templating with response chaining, auth (Basic, Bearer, API key, Digest, OAuth 2.0, AWS SigV4), request execution on reqwest + rustls with redirects, cookies, proxies, client certificates, timings and a timeline, JSONPath filters, assertions, response history, importers (curl, Insomnia v4/v5, Postman v2.1, OpenAPI 3, own export) and code generation (curl, Python, JS fetch, Go, Rust, Java, Kotlin). |

Until the plugin API (#48) is merged, run the UI in the development host:

```bash
cargo run -p nexus-http --example standalone -- --demo   # seeded workspace + local demo API
cargo run -p nexus-http --example standalone -- ~/my-api # stores in ~/my-api/.nexus
```

Shortcuts: Ctrl+Enter send, Ctrl+L focus URL, Ctrl+P request switcher, Ctrl+E
environments, Ctrl+N new request, Ctrl+S save, Esc cancel. Status and evidence: `docs/parity/nexus-http.md`.

## Storage layout

```text
.nexus/
  .gitignore                      ignores local/
  workspaces/<workspace-id>/
    workspace.json                name, settings, tree order, environment order
    items/<item-id>.json          one folder (without children) or request
    environments/base.json
    environments/<env-id>.json    secret values are blanked here
  local/<workspace-id>/           machine-local, not committed
    secrets.json  cookies.json  history.json
```

## Templates

`{{ host }}`, `{{ api.version }}`, `{{ users[0].id }}` and `{{ _.token }}` read the
merged variables (base environment < sub environment < folder variables). Tags:
`{% uuid %}`, `{% now 'iso'|'millis'|'unix'|'date'|'time' %}`,
`{% base64 'encode', value %}`, `{% urlencode value %}`, `{% hash 'sha256', value %}`,
`{% random_int 1, 10 %}`, `{% env 'NAME' %}` and
`{% response 'body', 'req_id', '$.token' %}` for chaining.

## Tests

`cargo test -p nexus-http-core` runs unit tests (including the RFC 2617 Digest,
RFC 7636 PKCE and AWS SigV4 `get-vanilla` vectors) and end-to-end tests against a
local HTTP server.
