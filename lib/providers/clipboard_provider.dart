import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/clipboard_item.dart';
import '../services/clipboard_service.dart';

/// Content-type filter options for the UI
enum ClipboardFilter { all, text, image, file, pinned }

/// Central state manager for the clipboard manager.
class ClipboardProvider extends ChangeNotifier {
  static const _storageKey = 'clipboard_history';
  static const _maxItems = 500;

  final ClipboardService _service = ClipboardService();

  List<ClipboardItem> _items = [];
  String _searchQuery = '';
  ClipboardFilter _filter = ClipboardFilter.all;
  String? _selectedItemId;
  bool _isLoading = true;

  // ── Public getters ──────────────────────────────────────────────────────

  bool get isLoading => _isLoading;
  String get searchQuery => _searchQuery;
  ClipboardFilter get filter => _filter;
  String? get selectedItemId => _selectedItemId;

  List<ClipboardItem> get pinnedItems =>
      _items.where((i) => i.isPinned).toList();

  List<ClipboardItem> get filteredItems {
    var list = _items.where((item) {
      // Apply type filter
      switch (_filter) {
        case ClipboardFilter.all:
          break;
        case ClipboardFilter.text:
          if (item.type != ClipboardItemType.text &&
              item.type != ClipboardItemType.richText) return false;
          break;
        case ClipboardFilter.image:
          if (item.type != ClipboardItemType.image) return false;
          break;
        case ClipboardFilter.file:
          if (item.type != ClipboardItemType.file) return false;
          break;
        case ClipboardFilter.pinned:
          if (!item.isPinned) return false;
          break;
      }
      // Apply search
      return item.matchesQuery(_searchQuery);
    }).toList();

    // Pinned items first, then sorted by recency
    list.sort((a, b) {
      if (a.isPinned && !b.isPinned) return -1;
      if (!a.isPinned && b.isPinned) return 1;
      return b.timestamp.compareTo(a.timestamp);
    });

    return list;
  }

  ClipboardItem? get selectedItem {
    if (_selectedItemId == null) return null;
    try {
      return _items.firstWhere((i) => i.id == _selectedItemId);
    } catch (_) {
      return null;
    }
  }

  // ── Initialisation ──────────────────────────────────────────────────────

  ClipboardProvider() {
    _init();
  }

  Future<void> _init() async {
    await _loadFromStorage();
    _service.onNewItem.listen(_onNewItem);
    _service.start();
    _isLoading = false;
    notifyListeners();
  }

  // ── Clipboard events ────────────────────────────────────────────────────

  void _onNewItem(ClipboardItem item) {
    // Deduplicate: if the latest non-pinned item has the same text, skip.
    if (_items.isNotEmpty) {
      final last = _items.first;
      if (last.type == item.type &&
          last.type == ClipboardItemType.text &&
          last.text == item.text) return;
    }

    _items.insert(0, item);
    if (_items.length > _maxItems) {
      // Remove oldest non-pinned items
      final nonPinned =
          _items.where((i) => !i.isPinned).toList();
      while (_items.length > _maxItems && nonPinned.isNotEmpty) {
        final oldest = nonPinned.removeLast();
        _items.remove(oldest);
      }
    }

    _selectedItemId ??= item.id;
    _saveToStorage();
    notifyListeners();
  }

  // ── User actions ────────────────────────────────────────────────────────

  void setSearchQuery(String query) {
    _searchQuery = query;
    notifyListeners();
  }

  void setFilter(ClipboardFilter filter) {
    _filter = filter;
    notifyListeners();
  }

  void selectItem(String id) {
    _selectedItemId = id;
    notifyListeners();
  }

  Future<void> copyItem(ClipboardItem item) async {
    await _service.copyToClipboard(item);
    // Move to top
    _items.remove(item);
    _items.insert(0, ClipboardItem(
      id: item.id,
      type: item.type,
      timestamp: DateTime.now(),
      text: item.text,
      richText: item.richText,
      imageBytes: item.imageBytes,
      imageWidth: item.imageWidth,
      imageHeight: item.imageHeight,
      filePaths: item.filePaths,
      colorHex: item.colorHex,
      sourceApp: item.sourceApp,
      sourceAppBundleId: item.sourceAppBundleId,
      sourceAppIcon: item.sourceAppIcon,
      isPinned: item.isPinned,
      tag: item.tag,
    ));
    _saveToStorage();
    notifyListeners();
  }

  void togglePin(String id) {
    final index = _items.indexWhere((i) => i.id == id);
    if (index < 0) return;
    _items[index] = _items[index].copyWith(isPinned: !_items[index].isPinned);
    _saveToStorage();
    notifyListeners();
  }

  void deleteItem(String id) {
    _items.removeWhere((i) => i.id == id);
    if (_selectedItemId == id) {
      _selectedItemId = _items.isNotEmpty ? _items.first.id : null;
    }
    _saveToStorage();
    notifyListeners();
  }

  void clearAll({bool keepPinned = true}) {
    if (keepPinned) {
      _items = _items.where((i) => i.isPinned).toList();
    } else {
      _items.clear();
    }
    _selectedItemId = _items.isNotEmpty ? _items.first.id : null;
    _saveToStorage();
    notifyListeners();
  }

  // ── Persistence ─────────────────────────────────────────────────────────

  Future<void> _saveToStorage() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final jsonList = _items.map((i) => i.toJson()).toList();
      await prefs.setString(_storageKey, jsonEncode(jsonList));
    } catch (_) {}
  }

  Future<void> _loadFromStorage() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final raw = prefs.getString(_storageKey);
      if (raw != null) {
        final list = (jsonDecode(raw) as List)
            .cast<Map<String, dynamic>>()
            .map(ClipboardItem.fromJson)
            .toList();
        _items = list;
        if (_items.isNotEmpty) _selectedItemId = _items.first.id;
      }
    } catch (_) {}
  }

  @override
  void dispose() {
    _service.dispose();
    super.dispose();
  }
}
