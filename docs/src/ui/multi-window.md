# Multi-Window

Perry supports creating multiple native windows in a single application.

## Creating Windows

```typescript
import { App, Window, Text, Button, VStack } from "perry/ui";

App("Multi-Window App", () =>
  VStack([
    Text("Main Window"),
    Button("Open New Window", () => {
      Window("Second Window", () =>
        VStack([
          Text("This is a second window"),
          Button("Close", () => {
            // Close this window
          }),
        ])
      );
    }),
  ])
);
```

`Window(title, renderFn)` creates a new native window with its own widget tree.

## Platform Notes

| Platform | Implementation |
|----------|---------------|
| macOS | NSWindow |
| Windows | CreateWindowEx |
| Linux | GtkWindow |
| Web | Floating `<div>` |
| iOS/Android | Modal view controller / Dialog |

On mobile platforms, "windows" are presented as modal views or dialogs since mobile apps typically use a single-window model.

## Next Steps

- [Dialogs](dialogs.md) — Modal dialogs and sheets
- [Menus](menus.md) — Menu bar and toolbar
- [UI Overview](overview.md) — Full UI system overview
