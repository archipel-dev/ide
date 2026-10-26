-- Example Archipel IDE Lua plugin (plugin API 1). See docs/plugins/lua.md.
local util = require("util")

local state = { name = "Lua", greetings = 0, todos = {}, draft = "", selected = nil }

local function say_hello(ctx)
  state.greetings = state.greetings + 1
  local message = util.greeting(ctx.setting("greeting"), state.name)
  ctx.status(message)
  if ctx.setting("notify") ~= false then
    ctx.notify("Hello Lua", message)
  end
  return message
end

local plugin = {
  commands = {
    { id = "hello_lua.sayHello", label = "Say Hello (Lua)", run = function(ctx) return { message = say_hello(ctx) } end },
    {
      id = "hello_lua.greet", label = "Greet Someone (Lua)", params = '{"name":"string"}',
      run = function(ctx, params)
        if type(params) ~= "table" or type(params.name) ~= "string" or params.name == "" then
          error("expected string field `name`")
        end
        state.name = params.name
        return { message = say_hello(ctx) }
      end,
    },
    { id = "hello_lua.openTodos", label = "Open Lua Todos", run = function(ctx) ctx.open_tab("todos", "Todos (Lua)") end },
  },
  menus = { { menu = "Tools", command = "hello_lua.sayHello" }, { menu = "Tools", command = "hello_lua.openTodos" } },
  keybindings = { { command = "hello_lua.sayHello", key = "Mod+Alt+L" } },
  tool_windows = { { id = "hello_lua", title = "Hello Lua", view = "main", icon_text = "L" } },
  editors = { { view = "todos", title = "Todos (Lua)" } },
  status_widgets = {
    { id = "count", command = "hello_lua.sayHello", text = function() return "Lua × " .. state.greetings end },
  },
  settings_pages = {
    {
      id = "hello_lua", title = "Hello Lua",
      fields = {
        { key = "greeting", label = "Greeting", kind = { type = "text", default = "Hello" } },
        { key = "notify", label = "Show notification", kind = { type = "bool", default = true } },
      },
    },
  },
  views = {},
}

-- Immediate-mode view: draw with `ui` every frame.
function plugin.views.main(ctx, ui)
  ui.heading("Hello from Lua")
  state.name = ui.text_input("name", state.name, "Name")
  ui.horizontal(function(row)
    if row.button("Say Hello") then say_hello(ctx) end
    if row.button("Open Todos") then ctx.open_tab("todos", "Todos (Lua)") end
  end)
  ui.section("Statistics")
  ui.property("Greetings", tostring(state.greetings))
  ui.property("Project", ctx.project_root or "none")
end

-- Declarative view: return a widget tree; interactions arrive in on_view_event.
function plugin.views.todos(ctx, ui, placement)
  local items = {}
  for i, todo in ipairs(state.todos) do
    items[#items + 1] = { id = tostring(i), label = (todo.done and "✓ " or "○ ") .. todo.text }
  end
  return {
    type = "column",
    children = {
      { type = "heading", text = "Todos" },
      { type = "row", children = {
        { type = "text_input", id = "draft", value = state.draft, hint = "What needs doing?" },
        { type = "button", id = "add", label = "Add" },
      } },
      #items > 0 and { type = "list", id = "todos", items = items, selected = state.selected }
        or { type = "empty", title = "Nothing to do", hint = "Add a todo above." },
      { type = "label", text = #state.todos .. " items · click one to toggle", muted = true },
    },
  }
end

function plugin.on_view_event(ctx, view, event)
  if event.id == "draft" then
    state.draft = event.value
  elseif event.id == "add" and state.draft ~= "" then
    table.insert(state.todos, { text = state.draft, done = false })
    state.draft = ""
  elseif event.id == "todos" then
    local todo = state.todos[tonumber(event.item)]
    if todo then todo.done = not todo.done end
    state.selected = event.item
  end
end

function plugin.activate(ctx)
  ctx.status("Hello Lua activated")
end

return plugin
