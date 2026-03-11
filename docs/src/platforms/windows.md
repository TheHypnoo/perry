# Windows

Perry compiles TypeScript apps for Windows using the Win32 API.

## Requirements

- Visual Studio Build Tools with "Desktop development with C++" workload
- Windows 10 or later

## Building

```bash
perry app.ts -o app.exe --target windows
```

## UI Toolkit

Perry maps UI widgets to Win32 controls:

| Perry Widget | Win32 Class |
|-------------|------------|
| Text | Static HWND |
| Button | HWND Button |
| TextField | Edit HWND |
| SecureField | Edit (ES_PASSWORD) |
| Toggle | Checkbox |
| Slider | Trackbar (TRACKBAR_CLASSW) |
| Picker | ComboBox |
| ProgressView | PROGRESS_CLASSW |
| Image | GDI |
| VStack/HStack | Manual layout |
| ScrollView | WS_VSCROLL |
| Canvas | GDI drawing |
| Form/Section | GroupBox |

## Windows-Specific APIs

- **Menu bar**: HMENU / SetMenu
- **Dark mode**: Windows Registry detection
- **Preferences**: Windows Registry
- **Keychain**: CredWrite/CredRead/CredDelete (Windows Credential Manager)
- **Notifications**: Toast notifications
- **File dialogs**: IFileOpenDialog / IFileSaveDialog (COM)
- **Alerts**: MessageBoxW
- **Open URL**: ShellExecuteW

## Next Steps

- [Platform Overview](overview.md) — All platforms
- [UI Overview](../ui/overview.md) — UI system
