# Creating Widgets

Define iOS home screen widgets using the `Widget()` function.

## Widget Declaration

```typescript
import { Widget, Text, VStack, HStack, Image, Spacer } from "perry/widget";

Widget({
  kind: "WeatherWidget",
  displayName: "Weather",
  description: "Shows current weather",
  entryFields: {
    temperature: "number",
    condition: "string",
    location: "string",
  },
  render: (entry) =>
    VStack([
      HStack([
        Text(entry.location),
        Spacer(),
        Image("cloud.sun.fill"),
      ]),
      Text(`${entry.temperature}°`),
      Text(entry.condition),
    ]),
});
```

## Widget Options

| Property | Type | Description |
|----------|------|-------------|
| `kind` | `string` | Unique identifier for the widget |
| `displayName` | `string` | Name shown in widget gallery |
| `description` | `string` | Description in widget gallery |
| `entryFields` | `object` | Data fields with types (`"string"`, `"number"`, `"boolean"`) |
| `render` | `function` | Render function receiving entry data, returns widget tree |

## Entry Fields

Entry fields define the data your widget displays. Each field has a name and type:

```typescript
entryFields: {
  title: "string",
  count: "number",
  isActive: "boolean",
}
```

These compile to a Swift `TimelineEntry` struct:

```swift
struct WeatherEntry: TimelineEntry {
    let date: Date
    let temperature: Double
    let condition: String
    let location: String
}
```

## Conditionals in Render

Use ternary expressions for conditional rendering:

```typescript
render: (entry) =>
  VStack([
    Text(entry.isActive ? "Active" : "Inactive"),
    entry.count > 0 ? Text(`${entry.count} items`) : Spacer(),
  ]),
```

## Template Literals

Template literals in widget text are compiled to Swift string interpolation:

```typescript
Text(`${entry.name}: ${entry.score} points`)
// Compiles to: Text("\(entry.name): \(entry.score) points")
```

## Multiple Widgets

Define multiple widgets in a single file. They're bundled into a `WidgetBundle`:

```typescript
Widget({
  kind: "SmallWidget",
  // ...
});

Widget({
  kind: "LargeWidget",
  // ...
});
```

## Next Steps

- [Components](components.md) — Available widget components and modifiers
- [Overview](overview.md) — Widget system overview
