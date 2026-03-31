# Paste – Clipboard Manager

A Flutter-based clipboard manager inspired by [Paste for macOS](https://pasteapp.io/).  
Quickly recall anything you've ever copied: text, images, files and more.

---

## Features

| Feature | Description |
|---------|-------------|
| 📋 Clipboard History | Automatically captures every item you copy and keeps them in a searchable list |
| 🔤 Text | Full-text preview with monospace rendering and selectable content |
| 🖼️ Images | PNG / JPEG / GIF / WebP thumbnails with full-size preview |
| 📁 Files | File paths displayed with filename previews |
| 🎨 Colors | Hex color swatches auto-detected from copied color strings |
| 🔍 Search | Instant full-text search across all clipboard entries |
| 🏷️ Type Filters | Filter by All / Text / Images / Files / Pinned |
| 📌 Pin / Favourite | Pin important items so they survive a history clear |
| 🖥️ Source App | Shows which application the content was copied from |
| ⌨️ Global Hotkey | Press **⌘⇧V** (macOS) / **Ctrl+Shift+V** (Windows/Linux) to toggle the window |
| 🌙 Dark Mode | Full light / dark theme following system preference |
| 💾 Persistence | History is saved across restarts using `shared_preferences` |

---

## Getting Started

### Prerequisites

- Flutter **≥ 3.10** with Desktop support enabled
- macOS 10.14+, Windows 10+, or Linux (X11)

### Run

```bash
# Clone
git clone https://github.com/gladmo/paste.git
cd paste

# Get dependencies
flutter pub get

# Run on your platform
flutter run -d macos      # macOS
flutter run -d windows    # Windows
flutter run -d linux      # Linux
```

### Permissions (macOS)

The app reads the system clipboard via the `super_clipboard` plugin. On macOS you may need to grant **Accessibility** or **Automation** permission in *System Settings → Privacy & Security* for source-app detection (the AppleScript query) to work.

---

## Architecture

```
lib/
├── main.dart                   # App entry point, window setup, hotkey
├── models/
│   └── clipboard_item.dart     # Data model (type, content, metadata)
├── providers/
│   └── clipboard_provider.dart # ChangeNotifier – state, filtering, persistence
├── services/
│   └── clipboard_service.dart  # Clipboard polling & source-app detection
├── screens/
│   └── home_screen.dart        # Main screen (split-pane layout)
├── widgets/
│   ├── title_bar.dart          # Custom draggable title bar with traffic lights
│   ├── search_field.dart       # Search input
│   ├── filter_bar.dart         # Type-filter chips
│   ├── clipboard_list.dart     # Scrollable list of clipboard items
│   └── detail_panel.dart       # Full-content preview + actions
└── theme/
    └── app_theme.dart          # Light & dark MaterialTheme definitions
```

### State Management

[Provider](https://pub.dev/packages/provider) (`ChangeNotifier`) is used for all UI state.  
`ClipboardProvider` owns the item list, search query, active filter, and selected item.  
`ClipboardService` polls the system clipboard every 800 ms and emits new items via a `Stream`.

### Persistence

The clipboard history (up to 500 items) is serialised to JSON and stored with  
`SharedPreferences` so it survives app restarts. Image bytes are included, so  
very large histories may grow the preferences file—future work can move images  
to a local SQLite database.

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘⇧V` / `Ctrl+Shift+V` | Show / hide the Paste window (global) |
| Double-click item | Copy the item back to clipboard |

---

## Running Tests

```bash
flutter test
```

---

## Contributing

Pull requests are welcome. For major changes please open an issue first to discuss what you'd like to change.

## License

MIT
