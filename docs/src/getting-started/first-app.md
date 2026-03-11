# First Native App

Perry compiles declarative TypeScript UI code to native platform widgets. No Electron, no WebView — real AppKit on macOS, UIKit on iOS, GTK4 on Linux, Win32 on Windows.

## A Simple Counter

Create `counter.ts`:

```typescript
import { App, Text, Button, VStack, State } from "perry/ui";

const count = State(0);

App("My Counter", () =>
  VStack([
    Text(`Count: ${count.get()}`),
    Button("Increment", () => {
      count.set(count.get() + 1);
    }),
    Button("Reset", () => {
      count.set(0);
    }),
  ])
);
```

Compile and run:

```bash
perry counter.ts -o counter
./counter
```

A native window opens with a label and two buttons. Clicking "Increment" updates the count in real-time.

## How It Works

- **`App(title, renderFn)`** — Creates a native application window. The render function defines the UI.
- **`State(initialValue)`** — Creates reactive state. When you call `.set()`, the UI re-renders.
- **`VStack([...])`** — Vertical stack layout (like SwiftUI's VStack or CSS flexbox column).
- **`Text(string)`** — A text label. Template literals with `${state.get()}` update reactively.
- **`Button(label, onClick)`** — A native button with a click handler.

## A Todo App

```typescript
import { App, Text, Button, TextField, VStack, HStack, State, ForEach } from "perry/ui";

const todos = State<string[]>([]);
const input = State("");

App("Todo App", () =>
  VStack([
    HStack([
      TextField(input, "Add a todo..."),
      Button("Add", () => {
        const text = input.get();
        if (text.length > 0) {
          todos.set([...todos.get(), text]);
          input.set("");
        }
      }),
    ]),
    ForEach(todos, (todo, index) =>
      HStack([
        Text(todo),
        Button("Remove", () => {
          const items = todos.get();
          todos.set(items.filter((_, i) => i !== index));
        }),
      ])
    ),
  ])
);
```

## Cross-Platform

The same code runs on all 6 platforms:

```bash
# macOS (default)
perry app.ts -o app
./app

# iOS Simulator
perry app.ts -o app --target ios-simulator

# Web (generates HTML)
perry app.ts -o app --target web
open app.html

# Other platforms
perry app.ts -o app --target windows
perry app.ts -o app --target linux
perry app.ts -o app --target android
```

Each target compiles to the platform's native widget toolkit. See [Platforms](../platforms/overview.md) for details.

## Adding Styling

```typescript
import { App, Text, Button, VStack, State } from "perry/ui";

const count = State(0);

App("Styled Counter", () => {
  const label = Text(`Count: ${count.get()}`);
  label.setFontSize(24);
  label.setColor("#333333");

  const btn = Button("Increment", () => count.set(count.get() + 1));
  btn.setCornerRadius(8);
  btn.setBackgroundColor("#007AFF");

  const stack = VStack([label, btn]);
  stack.setPadding(20);
  return stack;
});
```

See [Styling](../ui/styling.md) for all available style properties.

## Next Steps

- [Project Configuration](project-config.md) — Set up `package.json` for Perry projects
- [UI Overview](../ui/overview.md) — Complete guide to Perry's UI system
- [Widgets Reference](../ui/widgets.md) — All available widgets
- [State Management](../ui/state.md) — Reactive state and bindings
