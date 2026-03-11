# Widget Components

Available components and modifiers for WidgetKit widgets.

## Text

```typescript
Text("Hello, World!")
Text(`${entry.name}: ${entry.value}`)
```

### Text Modifiers

```typescript
const t = Text("Styled");
t.font("title");       // .title, .headline, .body, .caption, etc.
t.color("blue");       // Named color or hex
t.bold();
```

## Layout

### VStack

```typescript
VStack([
  Text("Top"),
  Text("Bottom"),
])
```

### HStack

```typescript
HStack([
  Text("Left"),
  Spacer(),
  Text("Right"),
])
```

### ZStack

```typescript
ZStack([
  Image("background"),
  Text("Overlay"),
])
```

## Spacer

Flexible space that expands to fill available room:

```typescript
HStack([
  Text("Left"),
  Spacer(),
  Text("Right"),
])
```

## Image

Display SF Symbols or asset images:

```typescript
Image("star.fill")           // SF Symbol
Image("cloud.sun.rain.fill") // SF Symbol
```

## Modifiers

Widget components support SwiftUI-style modifiers:

### Font

```typescript
Text("Title").font("title")
Text("Body").font("body")
Text("Caption").font("caption")
```

### Color

```typescript
Text("Red text").color("red")
Text("Custom").color("#FF6600")
```

### Padding

```typescript
VStack([...]).padding(16)
```

### Frame

```typescript
widget.frame(width, height)
```

## Conditionals

Render different components based on entry data:

```typescript
render: (entry) =>
  VStack([
    entry.isOnline
      ? Text("Online").color("green")
      : Text("Offline").color("red"),
  ]),
```

## Complete Example

```typescript
import { Widget, Text, VStack, HStack, Image, Spacer } from "perry/widget";

Widget({
  kind: "StatsWidget",
  displayName: "Stats",
  description: "Shows daily stats",
  entryFields: {
    steps: "number",
    calories: "number",
    distance: "string",
  },
  render: (entry) =>
    VStack([
      HStack([
        Image("figure.walk"),
        Text("Daily Stats").font("headline"),
      ]),
      Spacer(),
      HStack([
        VStack([
          Text(`${entry.steps}`).font("title").bold(),
          Text("steps").font("caption").color("gray"),
        ]),
        Spacer(),
        VStack([
          Text(`${entry.calories}`).font("title").bold(),
          Text("cal").font("caption").color("gray"),
        ]),
        Spacer(),
        VStack([
          Text(entry.distance).font("title").bold(),
          Text("km").font("caption").color("gray"),
        ]),
      ]),
    ]).padding(16),
});
```

## Next Steps

- [Creating Widgets](creating-widgets.md) — Widget() API
- [Overview](overview.md) — Widget system overview
