# State Management

Perry uses reactive state to automatically update the UI when data changes.

## Creating State

```typescript
import { State } from "perry/ui";

const count = State(0);           // number state
const name = State("Perry");      // string state
const items = State<string[]>([]); // array state
```

`State(initialValue)` creates a reactive state container.

## Reading and Writing

```typescript
const value = count.get();  // Read current value
count.set(42);              // Set new value → triggers UI update
```

Every `.set()` call re-renders the widget tree with the new value.

## Reactive Text

Template literals with `state.get()` update automatically:

```typescript
import { Text, State } from "perry/ui";

const count = State(0);
Text(`Count: ${count.get()}`);
// The text updates whenever count changes
```

This works because Perry detects `State.get()` calls inside template literals and creates reactive bindings.

## Two-Way Binding

`TextField` and other input widgets bind to state bidirectionally:

```typescript
import { TextField, State } from "perry/ui";

const input = State("");
TextField(input, "Type here...");

// input.get() always reflects what the user typed
// input.set("hello") updates the text field
```

Controls that support two-way binding:
- `TextField(state, placeholder)` — text input
- `SecureField(state, placeholder)` — password input
- `Toggle(label, state)` — boolean toggle
- `Slider(state, min, max)` — numeric slider
- `Picker(options, state)` — selection

## onChange Callbacks

Listen for state changes:

```typescript
import { State } from "perry/ui";

const count = State(0);
count.onChange((newValue) => {
  console.log(`Count changed to ${newValue}`);
});
```

## ForEach

Render a list from array state:

```typescript
import { VStack, Text, ForEach, State } from "perry/ui";

const items = State(["Apple", "Banana", "Cherry"]);

VStack([
  ForEach(items, (item, index) =>
    Text(`${index + 1}. ${item}`)
  ),
]);
```

`ForEach` re-renders the list when the state changes:

```typescript
// Add an item
items.set([...items.get(), "Date"]);

// Remove an item
items.set(items.get().filter((_, i) => i !== 1));
```

## Conditional Rendering

Use state to conditionally show widgets:

```typescript
import { VStack, Text, Button, State } from "perry/ui";

const showDetails = State(false);

VStack([
  Button("Toggle", () => showDetails.set(!showDetails.get())),
  showDetails.get() ? Text("Details are visible!") : Spacer(),
]);
```

## Multi-State Text

Text can depend on multiple state values:

```typescript
const firstName = State("John");
const lastName = State("Doe");

Text(`Hello, ${firstName.get()} ${lastName.get()}!`);
// Updates when either firstName or lastName changes
```

## State with Objects and Arrays

```typescript
const user = State({ name: "Perry", age: 0 });

// Update by replacing the whole object
user.set({ ...user.get(), age: 1 });

const todos = State<{ text: string; done: boolean }[]>([]);

// Add a todo
todos.set([...todos.get(), { text: "New task", done: false }]);

// Toggle a todo
const items = todos.get();
items[0].done = !items[0].done;
todos.set([...items]);
```

> **Note**: State uses identity comparison. You must create a new array/object reference for changes to be detected. Mutating in-place without calling `.set()` with a new reference won't trigger updates.

## Complete Example

```typescript
import { App, Text, Button, TextField, VStack, HStack, State, ForEach, Spacer, Divider } from "perry/ui";

const todos = State<string[]>([]);
const input = State("");

App("Todo App", () =>
  VStack([
    Text("My Todos"),

    HStack([
      TextField(input, "What needs to be done?"),
      Button("Add", () => {
        const text = input.get();
        if (text.length > 0) {
          todos.set([...todos.get(), text]);
          input.set("");
        }
      }),
    ]),

    Divider(),

    ForEach(todos, (todo, index) =>
      HStack([
        Text(todo),
        Spacer(),
        Button("Delete", () => {
          todos.set(todos.get().filter((_, i) => i !== index));
        }),
      ])
    ),

    Spacer(),
    Text(`${todos.get().length} items`),
  ])
);
```

## Next Steps

- [Events](events.md) — Click, hover, keyboard events
- [Widgets](widgets.md) — All available widgets
- [Layout](layout.md) — Layout containers
