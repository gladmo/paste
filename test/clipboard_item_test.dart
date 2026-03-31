import 'package:flutter_test/flutter_test.dart';
import 'package:paste/models/clipboard_item.dart';

void main() {
  group('ClipboardItem', () {
    test('text preview is trimmed and collapsed', () {
      final item = ClipboardItem(
        id: '1',
        type: ClipboardItemType.text,
        timestamp: DateTime.now(),
        text: '  hello   world  ',
      );
      expect(item.preview, 'hello world');
    });

    test('image preview shows dimensions when available', () {
      final item = ClipboardItem(
        id: '2',
        type: ClipboardItemType.image,
        timestamp: DateTime.now(),
        imageWidth: 800,
        imageHeight: 600,
      );
      expect(item.preview, contains('800'));
      expect(item.preview, contains('600'));
    });

    test('file preview returns file name for single file', () {
      final item = ClipboardItem(
        id: '3',
        type: ClipboardItemType.file,
        timestamp: DateTime.now(),
        filePaths: ['/Users/alice/Documents/report.pdf'],
      );
      expect(item.preview, 'report.pdf');
    });

    test('file preview shows count for multiple files', () {
      final item = ClipboardItem(
        id: '4',
        type: ClipboardItemType.file,
        timestamp: DateTime.now(),
        filePaths: ['/a/b.txt', '/a/c.txt', '/a/d.txt'],
      );
      expect(item.preview, '3 files');
    });

    test('matchesQuery returns true for matching text', () {
      final item = ClipboardItem(
        id: '5',
        type: ClipboardItemType.text,
        timestamp: DateTime.now(),
        text: 'Hello World',
      );
      expect(item.matchesQuery('hello'), isTrue);
      expect(item.matchesQuery('WORLD'), isTrue);
      expect(item.matchesQuery('flutter'), isFalse);
    });

    test('matchesQuery returns true for empty query', () {
      final item = ClipboardItem(
        id: '6',
        type: ClipboardItemType.text,
        timestamp: DateTime.now(),
        text: 'anything',
      );
      expect(item.matchesQuery(''), isTrue);
    });

    test('matchesQuery matches source app', () {
      final item = ClipboardItem(
        id: '7',
        type: ClipboardItemType.text,
        timestamp: DateTime.now(),
        text: 'some text',
        sourceApp: 'Visual Studio Code',
      );
      expect(item.matchesQuery('visual'), isTrue);
    });

    test('toJson / fromJson roundtrip', () {
      final original = ClipboardItem(
        id: 'abc',
        type: ClipboardItemType.text,
        timestamp: DateTime(2024, 6, 1, 12, 0, 0),
        text: 'roundtrip test',
        sourceApp: 'Finder',
        isPinned: true,
      );
      final json = original.toJson();
      final restored = ClipboardItem.fromJson(json);
      expect(restored.id, original.id);
      expect(restored.type, original.type);
      expect(restored.text, original.text);
      expect(restored.sourceApp, original.sourceApp);
      expect(restored.isPinned, original.isPinned);
      expect(restored.timestamp, original.timestamp);
    });

    test('copyWith preserves fields and overrides correctly', () {
      final item = ClipboardItem(
        id: 'x',
        type: ClipboardItemType.text,
        timestamp: DateTime.now(),
        text: 'original',
        isPinned: false,
      );
      final pinned = item.copyWith(isPinned: true);
      expect(pinned.isPinned, isTrue);
      expect(pinned.id, item.id);
      expect(pinned.text, item.text);
    });
  });
}
