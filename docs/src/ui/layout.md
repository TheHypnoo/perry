# Layout

Perry provides layout containers that arrange child widgets using the platform's native layout system.

## VStack

Arranges children vertically (top to bottom).

```typescript
import { VStack, Text, Button } from "perry/ui";

VStack([
  Text("First"),
  Text("Second"),
  Text("Third"),
]);
```

**Methods:**
- `setPadding(padding: number)` — Set padding around all edges
- `setSpacing(spacing: number)` — Set spacing between children

## HStack

Arranges children horizontally (left to right).

```typescript
import { HStack, Text, Button } from "perry/ui";

HStack([
  Button("Cancel", () => {}),
  Spacer(),
  Button("OK", () => {}),
]);
```

## ZStack

Layers children on top of each other (back to front).

```typescript
import { ZStack, Text, Image } from "perry/ui";

ZStack([
  Image("background.png"),
  Text("Overlay text"),
]);
```

## ScrollView

A scrollable container.

```typescript
import { ScrollView, VStack, Text } from "perry/ui";

ScrollView(
  VStack(
    Array.from({ length: 100 }, (_, i) => Text(`Row ${i}`))
  )
);
```

**Methods:**
- `setRefreshControl(callback: () => void)` — Add pull-to-refresh (calls callback on pull)
- `endRefreshing()` — Stop the refresh indicator

## LazyVStack

A vertically scrolling list that lazily renders items. More efficient than `ScrollView` + `VStack` for large lists.

```typescript
import { LazyVStack, Text } from "perry/ui";

LazyVStack(1000, (index) => {
  return Text(`Row ${index}`);
});
```

## NavigationStack

A navigation container that supports push/pop navigation.

```typescript
import { NavigationStack, Text, Button } from "perry/ui";

NavigationStack([
  Text("Home Screen"),
  Button("Go to Details", () => {
    // Push a new view
  }),
]);
```

## Spacer

A flexible space that expands to fill available room.

```typescript
import { HStack, Text, Spacer } from "perry/ui";

HStack([
  Text("Left"),
  Spacer(),
  Text("Right"),
]);
```

Use `Spacer()` inside `HStack` or `VStack` to push widgets apart.

## Divider

A visual separator line.

```typescript
import { VStack, Text, Divider } from "perry/ui";

VStack([
  Text("Section 1"),
  Divider(),
  Text("Section 2"),
]);
```

## Nesting Layouts

Layouts can be nested freely:

```typescript
import { App, VStack, HStack, Text, Button, Spacer, Divider } from "perry/ui";

App("Layout Example", () =>
  VStack([
    // Header
    HStack([
      Text("My App"),
      Spacer(),
      Button("Settings", () => {}),
    ]),
    Divider(),
    // Content
    VStack([
      Text("Welcome!"),
      HStack([
        Button("Action 1", () => {}),
        Button("Action 2", () => {}),
      ]),
    ]),
    Spacer(),
    // Footer
    Text("v1.0.0"),
  ])
);
```

## Child Management

Containers support dynamic child management:

```typescript
const stack = VStack([]);
// Add children dynamically
stack.addChild(Text("New child"));
stack.addChildAt(0, Text("Prepended"));
stack.removeChild(someWidget);
stack.reorderChild(widget, 2);
stack.clearChildren();
```

**Methods:**
- `addChild(widget)` — Append a child widget
- `addChildAt(index, widget)` — Insert a child at a specific position
- `removeChild(widget)` — Remove a child widget
- `reorderChild(widget, newIndex)` — Move a child to a new position
- `clearChildren()` — Remove all children

## Next Steps

- [Styling](styling.md) — Colors, padding, sizing
- [Widgets](widgets.md) — All available widgets
- [State Management](state.md) — Dynamic UI with state
