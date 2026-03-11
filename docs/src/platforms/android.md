# Android

Perry compiles TypeScript apps for Android using JNI (Java Native Interface).

## Requirements

- Android NDK
- Android SDK
- Rust Android targets:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi
  ```

## Building

```bash
perry app.ts -o app --target android
```

## UI Toolkit

Perry maps UI widgets to Android views via JNI:

| Perry Widget | Android Class |
|-------------|--------------|
| Text | TextView |
| Button | Button |
| TextField | EditText |
| SecureField | EditText (ES_PASSWORD) |
| Toggle | Switch |
| Slider | SeekBar |
| Picker | Spinner + ArrayAdapter |
| Image | ImageView |
| VStack | LinearLayout (vertical) |
| HStack | LinearLayout (horizontal) |
| ZStack | FrameLayout |
| ScrollView | ScrollView |
| Canvas | Canvas + Bitmap |
| NavigationStack | FrameLayout |

## Android-Specific APIs

- **Dark mode**: `Configuration.uiMode` detection
- **Preferences**: SharedPreferences
- **Keychain**: Android Keystore
- **Notifications**: NotificationManager
- **Open URL**: `Intent.ACTION_VIEW`
- **Alerts**: `PerryBridge.showAlert`
- **Sheets**: Dialog (modal)

## Differences from Desktop

- **Touch-only**: No hover events, no right-click context menus
- **Single window**: Multi-window maps to Dialog views
- **Toolbar**: Horizontal LinearLayout
- **Font**: Typeface-based font family support

## Next Steps

- [Platform Overview](overview.md) — All platforms
- [UI Overview](../ui/overview.md) — UI system
