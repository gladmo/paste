import 'dart:typed_data';

/// Enum for clipboard item content types
enum ClipboardItemType {
  text,
  richText,
  image,
  file,
  color,
  unknown,
}

/// Represents a single clipboard history entry
class ClipboardItem {
  final String id;
  final ClipboardItemType type;
  final DateTime timestamp;

  /// Plain text content (for text type)
  final String? text;

  /// Rich text content (HTML/RTF)
  final String? richText;

  /// Image bytes (for image type)
  final Uint8List? imageBytes;
  final int? imageWidth;
  final int? imageHeight;

  /// File paths (for file type)
  final List<String>? filePaths;

  /// Hex color string (for color type)
  final String? colorHex;

  /// Name of the source application (e.g., "Safari", "VS Code")
  final String? sourceApp;

  /// Bundle ID / executable of the source application
  final String? sourceAppBundleId;

  /// Optional icon bytes for the source app
  final Uint8List? sourceAppIcon;

  /// Whether this item is pinned (favorite)
  bool isPinned;

  /// Optional user-assigned tag/label
  String? tag;

  ClipboardItem({
    required this.id,
    required this.type,
    required this.timestamp,
    this.text,
    this.richText,
    this.imageBytes,
    this.imageWidth,
    this.imageHeight,
    this.filePaths,
    this.colorHex,
    this.sourceApp,
    this.sourceAppBundleId,
    this.sourceAppIcon,
    this.isPinned = false,
    this.tag,
  });

  /// Returns a short preview string for display purposes
  String get preview {
    switch (type) {
      case ClipboardItemType.text:
      case ClipboardItemType.richText:
        return (text ?? '').trim().replaceAll(RegExp(r'\s+'), ' ');
      case ClipboardItemType.image:
        final w = imageWidth != null ? '${imageWidth}×$imageHeight' : '';
        return 'Image $w'.trim();
      case ClipboardItemType.file:
        if (filePaths == null || filePaths!.isEmpty) return 'File';
        if (filePaths!.length == 1) return filePaths!.first.split('/').last;
        return '${filePaths!.length} files';
      case ClipboardItemType.color:
        return colorHex ?? 'Color';
      case ClipboardItemType.unknown:
        return 'Unknown';
    }
  }

  /// Returns true if the item matches the given search query
  bool matchesQuery(String query) {
    if (query.isEmpty) return true;
    final q = query.toLowerCase();
    if (text != null && text!.toLowerCase().contains(q)) return true;
    if (richText != null && richText!.toLowerCase().contains(q)) return true;
    if (sourceApp != null && sourceApp!.toLowerCase().contains(q)) return true;
    if (colorHex != null && colorHex!.toLowerCase().contains(q)) return true;
    if (filePaths != null) {
      for (final p in filePaths!) {
        if (p.toLowerCase().contains(q)) return true;
      }
    }
    return false;
  }

  /// Serialize to JSON-compatible map for persistence
  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'type': type.index,
      'timestamp': timestamp.toIso8601String(),
      'text': text,
      'richText': richText,
      'imageBytes': imageBytes?.toList(),
      'imageWidth': imageWidth,
      'imageHeight': imageHeight,
      'filePaths': filePaths,
      'colorHex': colorHex,
      'sourceApp': sourceApp,
      'sourceAppBundleId': sourceAppBundleId,
      'isPinned': isPinned,
      'tag': tag,
    };
  }

  factory ClipboardItem.fromJson(Map<String, dynamic> json) {
    return ClipboardItem(
      id: json['id'] as String,
      type: ClipboardItemType.values[json['type'] as int],
      timestamp: DateTime.parse(json['timestamp'] as String),
      text: json['text'] as String?,
      richText: json['richText'] as String?,
      imageBytes: json['imageBytes'] != null
          ? Uint8List.fromList((json['imageBytes'] as List).cast<int>())
          : null,
      imageWidth: json['imageWidth'] as int?,
      imageHeight: json['imageHeight'] as int?,
      filePaths: (json['filePaths'] as List?)?.cast<String>(),
      colorHex: json['colorHex'] as String?,
      sourceApp: json['sourceApp'] as String?,
      sourceAppBundleId: json['sourceAppBundleId'] as String?,
      isPinned: json['isPinned'] as bool? ?? false,
      tag: json['tag'] as String?,
    );
  }

  ClipboardItem copyWith({
    bool? isPinned,
    String? tag,
  }) {
    return ClipboardItem(
      id: id,
      type: type,
      timestamp: timestamp,
      text: text,
      richText: richText,
      imageBytes: imageBytes,
      imageWidth: imageWidth,
      imageHeight: imageHeight,
      filePaths: filePaths,
      colorHex: colorHex,
      sourceApp: sourceApp,
      sourceAppBundleId: sourceAppBundleId,
      sourceAppIcon: sourceAppIcon,
      isPinned: isPinned ?? this.isPinned,
      tag: tag ?? this.tag,
    );
  }
}
