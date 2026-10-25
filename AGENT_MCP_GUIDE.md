# DesignCraft Agent & MCP Interaction Guide

This guide explains how external AI agents, scripts, and MCP (Model Context Protocol) clients can interact with, control, and inspect DesignCraft—both in headless mode and live inside the running desktop GUI.

---

## 1. Architecture Overview

DesignCraft provides two interoperable channels for programmatic control:

```
+-------------------------------------------------------------------------------+
|                             AI Agent / Automation                             |
+-------------------------------------------------------------------------------+
         |                                                       |
         | (1) Direct JSON-RPC                                   | (2) MCP Protocol
         v                                                       v
+-------------------------------+             +---------------------------------+
|  127.0.0.1:7979 (TCP Socket)  |             |  designcraft-cli mcp            |
|  JSON-RPC 2.0 lines           |             |  (stdio JSON-RPC)               |
+-------------------------------+             +---------------------------------+
         |                                                       |
         |                                         +-------------+-------------+
         |                                         | (--connect 7979)          | (Headless)
         v                                         v                           v
+-----------------------------------------------------+            +--------------------+
|            DesignCraft Desktop GUI App              |            |  Headless Engine   |
|  - Engine commands (frame, text, pubgen, export...)  |            |  (Session + render |
|  - UI widgets (dock tabs, panels, click, drag...)   |            |   in-process)      |
|  - Screenshot / window capture                      |            +--------------------+
+-----------------------------------------------------+
```

1. **Direct TCP Control Protocol** (`127.0.0.1:7979`):
   Fast, raw newline-delimited JSON-RPC 2.0 socket exposed directly by the desktop app when started with `--control <port>`.
2. **Model Context Protocol (MCP)**:
   Standardized stdio server (`designcraft-cli mcp`) that exposes tools (`list_commands`, `execute`, `inspect_document`, `screenshot`, `ui_inspect`, `ui_set`, `click`, etc.) to Claude, Antigravity, Cursor, and other agent frameworks. Can run headless or bridge to the running desktop app via `--connect <port>`.

---

## 2. Launching DesignCraft with the Control Channel

By default, the desktop app does **not** open a network port. You must pass `--control <port>` (or set the `DESIGNCRAFT_CONTROL_PORT` environment variable).

### Development Command (with PubGen feature)

```powershell
# From the repository root (f:\GitHub\designcraft)
cargo run -p designcraft --features pubgen -- --control 7979
```

> [!IMPORTANT]
> Note the double-dash `--` before `--control 7979`.
> Arguments before `--` go to `cargo`, while arguments after `--` are passed directly to `designcraft.exe`.
> Without `-- --control 7979`, the application runs normally without opening port 7979.

### Release Command

```powershell
cargo run --release -p designcraft --features pubgen -- --sample --control 7979
```

To verify that the control port is active:
```powershell
Test-NetConnection -ComputerName 127.0.0.1 -Port 7979
```

---

## 3. Channel 1: Direct TCP Control Protocol

The server listens on `127.0.0.1:<port>` only and speaks newline-delimited JSON lines: **one request object per line, one reply per line**.

### 3.1 Request & Response Format

Every line sent must be a valid JSON object containing at least `method`:

```json
{"id": 1, "method": "engine.execute", "params": {"command": "frame.create", "params": {"rect": [36, 36, 300, 200], "content": "text"}}}
```

The app responds with:

```json
{"id": 1, "ok": true, "result": {"id": 10, "story": 11}}
```

> [!WARNING]
> Any line that is not valid JSON, contains an unknown method, or exceeds 4 MiB causes the server to reply with `{"ok": false, "error": "..."}` and **immediately close the TCP connection**. Ensure every message ends with `\n` and is formatted cleanly.

### 3.2 Core Control Methods

| Method | Params | Description |
|---|---|---|
| `engine.execute` | `{"command": "<id>", "params": { ... }}` | Execute any engine or UI command (`frame.create`, `style.paragraph.apply`, `file.exportPdf`, etc.). |
| `engine.commands` | — | List all available commands, shortcuts, parameter schemas, and whether they are enabled. |
| `document.inspect` | — | Summary of active document: pages, spreads, items, stories, styles, selection. |
| `ui.inspect` | — | Summary of UI state: active tool, canvas rectangle, zoom, open dialogs, dock tabs. |
| `ui.set` | `{"dockTab": "pubgen", "panel": "...", "zoom": 1.0, ...}` | Switch active panel/dock tab, change brightness, zoom, toggle guides/rulers. |
| `ui.screenshot` | `{"path": "C:/path/shot.png"}` | Capture the complete window as PNG. If `path` is omitted, returns base64 PNG data. |
| `ui.render` | `{"page": 0, "scale": 2.0, "path": "..."}` | Render a specific document page offscreen. |
| `ui.click` | `{"x": 100, "y": 200, "button": "left"}` | Synthetic pointer click in screen (egui) space. Reaches buttons, menus, and panels. |
| `ui.drag` | `{"x": 10, "y": 10, "toX": 100, "toY": 100, "steps": 8}` | Synthetic drag gesture. |
| `ui.key` / `ui.text` | `{"key": "Z", "cmd": true}` / `{"text": "Hello"}` | Keyboard shortcuts or typing text into active inputs. |
| `app.open` / `app.export` | `{"path": "..."}` | Open or export documents. |
| `app.quit` | — | Terminate the application immediately. |

### 3.3 Quick Interactivity via CLI Helper

`designcraft-cli app` is a built-in convenience command that connects to the running app, sends the request, prints the output, and disconnects:

```powershell
# Run an engine command
cargo run -p designcraft-cli -- app --port 7979 frame.create '{"rect":[36,36,200,100],"content":"text"}'

# Call any control method directly
cargo run -p designcraft-cli -- app --port 7979 --method ui.inspect
cargo run -p designcraft-cli -- app --port 7979 --method ui.screenshot '{"path":"outputs/current_ui.png"}'
```

### 3.4 Direct TCP in PowerShell

You can send commands directly using PowerShell without external dependencies:

```powershell
function Send-DesignCraftCommand {
    param (
        [string]$Method,
        [hashtable]$Params = @{},
        [int]$Port = 7979
    )
    $client = New-Object System.Net.Sockets.TcpClient("127.0.0.1", $Port)
    $stream = $client.GetStream()
    $writer = New-Object System.IO.StreamWriter($stream)
    $reader = New-Object System.IO.StreamReader($stream)
    $writer.AutoFlush = $true

    $payload = @{ id = 1; method = $Method; params = $Params } | ConvertTo-Json -Compress
    $writer.WriteLine($payload)
    $response = $reader.ReadLine()

    $client.Close()
    return ($response | ConvertFrom-Json)
}

# Example 1: Inspect UI
Send-DesignCraftCommand -Method "ui.inspect"

# Example 2: Take a screenshot
Send-DesignCraftCommand -Method "ui.screenshot" -Params @{ path = "f:/GitHub/designcraft/outputs/test_screenshot.png" }
```

---

## 4. Channel 2: Model Context Protocol (MCP)

DesignCraft includes a native MCP server implementation in `crates/mcp` and exposed through `designcraft-cli mcp`.

### 4.1 Running the MCP Server

There are two modes:

1. **Connected Mode (Desktop GUI)**:
   Connects to an already-running DesignCraft instance at `--control 7979`. Full access to UI clicks, dialogs, screenshots, and live rendering.
   ```sh
   designcraft-cli mcp --connect 7979
   ```

2. **Headless Mode**:
   Runs an in-process engine session without GUI. Ideal for headless CI/CD, document compilation, or text layout.
   ```sh
   designcraft-cli mcp
   ```

### 4.2 Configuring MCP in Agent Tools

#### A. Antigravity IDE
Add the following to your MCP configuration (`.gemini/antigravity-ide/mcp/designcraft.json` or project MCP config):

```json
{
  "mcpServers": {
    "designcraft": {
      "command": "cargo",
      "args": [
        "run",
        "-p",
        "designcraft-cli",
        "--",
        "mcp",
        "--connect",
        "7979"
      ]
    }
  }
}
```

#### B. Claude Desktop (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "designcraft": {
      "command": "cargo",
      "args": [
        "run",
        "--release",
        "-p",
        "designcraft-cli",
        "--",
        "mcp",
        "--connect",
        "7979"
      ],
      "cwd": "f:/GitHub/designcraft"
    }
  }
}
```

### 4.3 Available MCP Tools

When registered, agents have access to the following tools:

- `list_commands`: Search and list all engine commands and parameters.
- `execute`: Execute an engine command by ID with JSON parameters.
- `batch`: Execute sequential command chains with result references (`$0.id`, `$last.story`).
- `inspect_document`: Retrieve document hierarchy, pages, items, stories, and selection.
- `get_story` / `set_story_text`: Inspect or replace text in any story or frame.
- `new_document`, `open_document`, `save_document`, `place_image`.
- `render_page`: Render a page offscreen and return PNG data.
- `screenshot`: Capture the running desktop window (app-connected only).
- `ui_inspect`: Inspect active UI panels, dock states, dialogs, and coordinates.
- `ui_set`: Switch tabs/panels, adjust zoom, toggle grids/rulers, change brightness.
- `click` / `drag`: Send mouse clicks and drags in screen coordinates.
- `key` / `type_text`: Send keystrokes and type text.
- `dialog_set` / `dialog_confirm` / `dialog_cancel`: Control modal dialogs.

---

## 5. PubGen Automation Recipes

The PubGen panel (`crates/ui-egui/src/panels/pubgen/mod.rs`) can be automated via control scripts or MCP.

### Recipe 1: Switch to the PubGen Dock Tab

```json
{"id": 1, "method": "ui.set", "params": {"dockTab": "pubgen"}}
```

Or via CLI:
```powershell
cargo run -p designcraft-cli -- app --port 7979 --method ui.set '{"dockTab":"pubgen"}'
```

### Recipe 2: Trigger Master Excel Import

If using commands registered in the engine or quick actions:
```powershell
cargo run -p designcraft-cli -- app --port 7979 pubgen.import_master_excel '{"path":"f:/path/to/data.xlsx"}'
```

### Recipe 3: Visual Verification (Screenshot)

To verify the PubGen panel redesign visually after making UI code changes:
```powershell
# 1. Switch to Pubgen tab
cargo run -p designcraft-cli -- app --port 7979 --method ui.set '{"dockTab":"pubgen"}'

# 2. Capture screenshot to outputs/pubgen_panel.png
cargo run -p designcraft-cli -- app --port 7979 --method ui.screenshot '{"path":"f:/GitHub/designcraft/outputs/pubgen_panel.png"}'
```

---

## 6. Common Pitfalls & Troubleshooting

1. **`No connection could be made because the target machine actively refused it (127.0.0.1:7979)`**
   - The desktop app was launched without `--control 7979`. Restart it with `-- --control 7979`.
2. **Connection closes immediately upon sending data**
   - Verify that the message sent is valid JSON and ends with a newline (`\n`).
   - Check that `method` is a string and matches an accepted method.
3. **`pubgen` commands or tests not found**
   - Make sure you pass `--features pubgen` to both cargo run and cargo test.
4. **Synthetic UI clicks not hitting the button**
   - Coordinates for `ui.click` are in logical egui screen points (window top-left is `0, 0`).
   - Use `ui.inspect` or take a `ui.screenshot` first to inspect element coordinates.

---

## 7. Further Reference

- [control-protocol.md](file:///f:/GitHub/designcraft/docs/control-protocol.md): Low-level wire protocol specification.
- [mcp.md](file:///f:/GitHub/designcraft/docs/mcp.md): Detailed MCP specification and command schemas.
- [agents.md](file:///f:/GitHub/designcraft/docs/agents.md): Agent conventions, script syntax, and batch operations.
