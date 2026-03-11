# Other System APIs

Additional platform-level APIs.

## Open URL

Open a URL in the default browser or application:

```typescript
import { openURL } from "perry/system";

openURL("https://example.com");
openURL("mailto:user@example.com");
```

| Platform | Implementation |
|----------|---------------|
| macOS | NSWorkspace.open |
| iOS | UIApplication.open |
| Android | Intent.ACTION_VIEW |
| Windows | ShellExecuteW |
| Linux | xdg-open |
| Web | window.open |

## Dark Mode Detection

```typescript
import { isDarkMode } from "perry/system";

if (isDarkMode()) {
  // Use dark theme colors
}
```

| Platform | Detection |
|----------|-----------|
| macOS | NSApp.effectiveAppearance |
| iOS | UITraitCollection |
| Android | Configuration.uiMode |
| Windows | Registry (AppsUseLightTheme) |
| Linux | GTK settings |
| Web | prefers-color-scheme media query |

## Clipboard

```typescript
import { clipboardGet, clipboardSet } from "perry/system";

clipboardSet("Copied text!");
const text = clipboardGet();
```

## Next Steps

- [Overview](overview.md) — All system APIs
- [UI Overview](../ui/overview.md) — Building UIs
