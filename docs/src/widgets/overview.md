# Widgets (WidgetKit) Overview

Perry can compile TypeScript widget declarations to native SwiftUI WidgetKit extensions for iOS home screen widgets.

## What Are Widgets?

iOS home screen widgets display glanceable information outside your app. Perry's `perry/widget` module lets you define widgets in TypeScript that compile to native SwiftUI code.

```typescript
import { Widget, Text, VStack } from "perry/widget";

Widget({
  kind: "MyWidget",
  displayName: "My Widget",
  description: "Shows a greeting",
  entryFields: { name: "string" },
  render: (entry) =>
    VStack([
      Text(`Hello, ${entry.name}!`),
    ]),
});
```

## How It Works

```
TypeScript widget declaration
    ↓ Parse & Lower to WidgetDecl HIR
    ↓ perry-codegen-swiftui emits SwiftUI source
    ↓
Complete WidgetKit extension:
  - Entry struct
  - View
  - TimelineProvider
  - WidgetBundle
  - Info.plist
```

The compiler generates a complete SwiftUI WidgetKit extension — no Swift knowledge required.

## Building

```bash
perry widget.ts --target ios-widget
```

This produces a WidgetKit extension directory that can be added to an Xcode project.

## Next Steps

- [Creating Widgets](creating-widgets.md) — Widget() API in detail
- [Components](components.md) — Available widget components
- [iOS Platform](../platforms/ios.md) — iOS cross-compilation
