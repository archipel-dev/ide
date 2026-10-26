# nexus-http screenshots

Captured under Xvfb (1600×1000, OpenGL renderer) from the development host
`cargo run -p nexus-http --example standalone -- --demo`. The demo seeds an
"Acme Store API" workspace and serves real responses from a local HTTP server,
so every status, timing, header and cookie shown is a real response.
Environment variables pick the scene (see the example's doc comment), e.g.
`NEXUS_SEND=1 NEXUS_SELECT="List products" NEXUS_RESPONSE_TAB=Tests`.

| File | View |
|---|---|
| `01-request-params-json-response.png` | Sidebar tree with method colors, environment picker, URL bar with highlighted `{{ variables }}`, Params table (one disabled row), pretty JSON response with status/time/size pills and the JSONPath filter |
| `02-body-json-created.png` | JSON body editor with highlighting and Beautify; `201 Created` response |
| `03-auth-bearer-chained.png` | Bearer auth whose token is a `{% response %}` tag reading `$.token` from the Login response; empty response pane with the keyboard hints |
| `04-response-headers.png` | Request Headers tab and response Headers tab |
| `05-response-timeline.png` | Timeline: the raw request (rendered variables) and response |
| `06-tests-assertions.png` | Assertion editor (status, JSONPath, time) and the passing results |
| `07-environments.png` | Manage Environments: base/sub environments with colors, the JSON editor and secret keys |
| `08-request-switcher.png` | Ctrl+P request switcher, recent first, with folder paths and URLs |
| `09-generate-code.png` | Generate Code: curl snippet with variables resolved; Python, JS, Go, Rust, Java and Kotlin targets |
| `10-import.png` | Import dialog (curl, Insomnia v4/v5, Postman v2.1, OpenAPI 3; paste or file) |
| `11-light-theme.png` | Light theme: colors derive from the host's egui visuals |
| `12-graphql-body.png` | GraphQL body: query and variables editors and the response |
| `13-multipart-body.png` | Multipart form with a text field and a file field |
| `14-collection-runner.png` | Collection Runner on the Products folder: progress bar, summary pills, a failed JSONPath assertion expanded |
| `16-websocket.png` | WebSocket request: message composer with a template tag, two sent messages, acknowledgements and server-pushed updates |
| `17-server-sent-events.png` | Server-Sent Events price feed with event names and ids |
| `18-variable-autocomplete.png` | Typing `{{` in the URL opens variable completion with value previews (secret values are masked) |
| `19-graphql-schema-completion.png` | GraphQL: schema fetched by introspection, explorer expanded, field completion for `Product` at the caret |
| `15-collection-runner-workspace.png` | Whole-workspace run in the light theme; the multipart request fails with an actionable missing-file error |

## Differences from the reference client

- The response preview renders HTML as source and does not render images
  (it shows type and size).
- GraphQL argument and enum completion and query validation are not
  implemented, and there is no gRPC support yet (planned).
- These screenshots come from the development host. They will be retaken in
  `archipel-ide --app nexus-http` once the plugin API (#48 / #65) is merged.
