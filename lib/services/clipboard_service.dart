import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:uuid/uuid.dart';

import '../models/clipboard_item.dart';

/// Monitors the system clipboard and converts new entries into [ClipboardItem].
class ClipboardService {
  static const _pollInterval = Duration(milliseconds: 800);

  final _uuid = const Uuid();
  Timer? _timer;
  String? _lastTextHash;
  final StreamController<ClipboardItem> _controller =
      StreamController.broadcast();

  Stream<ClipboardItem> get onNewItem => _controller.stream;

  void start() {
    _timer?.cancel();
    _timer = Timer.periodic(_pollInterval, (_) => _poll());
  }

  void stop() {
    _timer?.cancel();
    _timer = null;
  }

  Future<void> _poll() async {
    try {
      final reader = await SystemClipboard.instance?.read();
      if (reader == null) return;

      // ── Image ────────────────────────────────────────────────────────────
      if (reader.canProvide(Formats.png) ||
          reader.canProvide(Formats.jpeg) ||
          reader.canProvide(Formats.gif) ||
          reader.canProvide(Formats.webp)) {
        final format = reader.canProvide(Formats.png)
            ? Formats.png
            : reader.canProvide(Formats.jpeg)
                ? Formats.jpeg
                : reader.canProvide(Formats.webp)
                    ? Formats.webp
                    : Formats.gif;

        final bytes = await _readSimpleValue<Uint8List>(reader, format);
        if (bytes != null) {
          final hash = _bytesHash(bytes);
          if (hash == _lastTextHash) return;
          _lastTextHash = hash;
          final item = ClipboardItem(
            id: _uuid.v4(),
            type: ClipboardItemType.image,
            timestamp: DateTime.now(),
            imageBytes: bytes,
            sourceApp: await _frontmostAppName(),
          );
          _controller.add(item);
          return;
        }
      }

      // ── Files ────────────────────────────────────────────────────────────
      if (reader.canProvide(Formats.fileUri)) {
        final uriValue =
            await _readSimpleValue<Uri>(reader, Formats.fileUri);
        if (uriValue != null) {
          final path = uriValue.toFilePath();
          final hash = path;
          if (hash == _lastTextHash) return;
          _lastTextHash = hash;
          final item = ClipboardItem(
            id: _uuid.v4(),
            type: ClipboardItemType.file,
            timestamp: DateTime.now(),
            filePaths: [path],
            sourceApp: await _frontmostAppName(),
          );
          _controller.add(item);
          return;
        }
      }

      // ── Plain text ───────────────────────────────────────────────────────
      if (reader.canProvide(Formats.plainText)) {
        final text =
            await _readSimpleValue<String>(reader, Formats.plainText);
        if (text != null && text.isNotEmpty) {
          if (text == _lastTextHash) return;
          _lastTextHash = text;
          final item = ClipboardItem(
            id: _uuid.v4(),
            type: ClipboardItemType.text,
            timestamp: DateTime.now(),
            text: text,
            sourceApp: await _frontmostAppName(),
          );
          _controller.add(item);
          return;
        }
      }
    } catch (_) {
      // Clipboard may be locked; ignore transient errors.
    }
  }

  Future<T?> _readSimpleValue<T>(
      ClipboardReader reader, ValueFormat<T> format) async {
    final completer = Completer<T?>();
    reader.getValue<T>(format, (value) {
      if (!completer.isCompleted) completer.complete(value);
    }, onError: (e) {
      if (!completer.isCompleted) completer.complete(null);
    });
    return completer.future.timeout(const Duration(seconds: 2),
        onTimeout: () => null);
  }

  String _bytesHash(Uint8List bytes) {
    // Simple rolling sum for change detection (not cryptographic)
    int h = 0;
    for (int i = 0; i < bytes.length && i < 512; i++) {
      h = (h * 31 + bytes[i]) & 0xFFFFFFFF;
    }
    return '$h:${bytes.length}';
  }

  /// Attempts to retrieve the name of the currently focused / frontmost app.
  /// Returns null on unsupported platforms or if detection fails.
  Future<String?> _frontmostAppName() async {
    try {
      if (Platform.isMacOS) {
        // Use AppleScript to ask System Events for the frontmost application.
        final result = await Process.run('osascript', [
          '-e',
          'tell application "System Events" to get name of first application process whose frontmost is true',
        ]);
        if (result.exitCode == 0) {
          return (result.stdout as String).trim();
        }
      } else if (Platform.isWindows) {
        final result = await Process.run('powershell', [
          '-Command',
          r'(Get-Process | Where-Object {$_.MainWindowHandle -ne 0} | Sort-Object CPU -Descending | Select-Object -First 1).Name',
        ]);
        if (result.exitCode == 0) {
          return (result.stdout as String).trim();
        }
      } else if (Platform.isLinux) {
        final result = await Process.run(
            'xdotool', ['getactivewindow', 'getwindowname']);
        if (result.exitCode == 0) {
          return (result.stdout as String).trim();
        }
      }
    } catch (_) {}
    return null;
  }

  /// Writes the given [item] back to the system clipboard.
  Future<void> copyToClipboard(ClipboardItem item) async {
    switch (item.type) {
      case ClipboardItemType.text:
      case ClipboardItemType.richText:
        if (item.text != null) {
          await Clipboard.setData(ClipboardData(text: item.text!));
        }
        break;
      case ClipboardItemType.image:
        if (item.imageBytes != null) {
          final clipboard = SystemClipboard.instance;
          if (clipboard != null) {
            await clipboard.write([
              DataWriterItem()..add(Formats.png(item.imageBytes!)),
            ]);
          }
        }
        break;
      case ClipboardItemType.file:
        if (item.filePaths != null && item.filePaths!.isNotEmpty) {
          // For files, copy the first file's path as text (simplistic).
          await Clipboard.setData(ClipboardData(text: item.filePaths!.first));
        }
        break;
      default:
        if (item.text != null) {
          await Clipboard.setData(ClipboardData(text: item.text!));
        }
    }
  }

  void dispose() {
    stop();
    _controller.close();
  }
}
