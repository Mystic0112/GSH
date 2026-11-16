{
  // .gsh/tasks.json — GSH Task Run config (JSONC: // , /* */ and
  // trailing commas are allowed; they're stripped before parsing).
  // Lives at the workspace root you open in GSH. Detected tasks (npm
  // scripts, pubspec) appear automatically; this file adds/overrides them.
  // Full reference: gsh/docs/tasks-json.md
  "tasks": [
    {
      "label": "Flutter Example", // shown in the Tasks list
      "cwd": "app", // run dir, relative to this file (monorepo-friendly)
      "command": "flutter", // base executable
      "args": ["run"], // base args, before the profile
      "kind": "watch", // "watch" = long-running (dev server); else "oneShot"
      // Optional: only show this task on some OSes — "macos" | "windows" |
      // "linux", as a string or array. Omitted -> visible everywhere.
      // "platforms": ["macos", "linux"],
      // Interactive keys -> buttons that write a key to the process stdin.
      // primary=true shows a fixed button; the rest go under a key menu.
      // icon: bolt | refresh | restart | stop (omit -> a chip with the key).
      "interactiveKeys": [
        { "key": "r", "label": "Hot reload", "icon": "bolt", "primary": true },
        { "key": "R", "label": "Hot restart", "icon": "restart", "primary": true },
        { "key": "p", "label": "Toggle debug paint" },
        { "key": "o", "label": "Toggle platform" }
      ],
      // Reload-on-save: `flutter run` doesn't reload on save by itself (that's
      // an IDE plugin) — GSH watches the files and fires `onChange`.
      "watch": {
        "paths": ["lib", "assets"], // dirs to watch (relative to cwd)
        "ignore": ["build", ".dart_tool"], // skip these (avoid loops)
        "onChange": "Hot reload", // an interactiveKey label, or "__restart__"
        "debounceMs": 300 // wait after a change before firing
      },
      // Drive the building/running badge by matching the output.
      "progressPatterns": [
        { "begin": "Performing hot reload", "end": "Reloaded .* in .*ms" },
        { "begin": "Performing hot restart", "end": "Restarted application in .*ms" }
      ],
      // Named arg/env variants, picked by the chip before Run (flavor /
      // dart-define just become args here — no stack-specific keys).
      "profiles": [
        { "name": "web", "args": ["-d", "chrome"] },
        { "name": "macos", "args": ["-d", "macos"] }
      ]
    },
    {
      // No "kind" -> defaults to "oneShot".
      "label": "Node Example",
      "cwd": "site",
      "command": "npm",
      "args": ["run", "dev"]
      // Browser auto-open. "preview": true (default) opens the first local URL
      // found in the output; false turns it off; a string opens that fixed URL
      // right at start.
      // "preview": "http://localhost:3000",
      // When it opens: "always" (default: start and restart), "start" (only on
      // start — Restart and the file watcher won't reopen it) or "never".
      // "previewOpen": "start",
    },
    {
      "label": "C# Example",
      "cwd": "api",
      "command": "dotnet",
      "args": ["watch", "run"],
      "kind": "watch"
    }
  ]
}
