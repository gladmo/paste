import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:window_manager/window_manager.dart';
import 'package:hotkey_manager/hotkey_manager.dart';

import '../providers/clipboard_provider.dart';
import '../widgets/clipboard_list.dart';
import '../widgets/detail_panel.dart';
import '../widgets/search_field.dart';
import '../widgets/filter_bar.dart';
import '../widgets/title_bar.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> with WindowListener {
  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    _registerHotKey();
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    hotKeyManager.unregisterAll();
    super.dispose();
  }

  /// Register a global hotkey (⌘⇧V on macOS, Ctrl+Shift+V elsewhere) to
  /// show / hide the window.
  Future<void> _registerHotKey() async {
    final hotKey = HotKey(
      key: PhysicalKeyboardKey.keyV,
      modifiers: [HotKeyModifier.meta, HotKeyModifier.shift],
      scope: HotKeyScope.system,
    );
    try {
      await hotKeyManager.register(
        hotKey,
        keyDownHandler: (_) async {
          final isVisible = await windowManager.isVisible();
          if (isVisible) {
            final isFocused = await windowManager.isFocused();
            if (isFocused) {
              await windowManager.hide();
            } else {
              await windowManager.focus();
            }
          } else {
            await windowManager.show();
            await windowManager.focus();
          }
        },
      );
    } catch (_) {
      // Hotkey registration may fail if another app holds it.
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      backgroundColor: theme.scaffoldBackgroundColor,
      body: Column(
        children: [
          const TitleBar(),
          const SearchField(),
          const FilterBar(),
          Expanded(
            child: Row(
              children: [
                // Left panel – clipboard list
                SizedBox(
                  width: 300,
                  child: Column(
                    children: [
                      const Expanded(child: ClipboardList()),
                    ],
                  ),
                ),
                // Divider
                VerticalDivider(
                  width: 1,
                  thickness: 1,
                  color: theme.dividerColor,
                ),
                // Right panel – detail view
                const Expanded(child: DetailPanel()),
              ],
            ),
          ),
          _BottomBar(),
        ],
      ),
    );
  }
}

class _BottomBar extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final provider = context.watch<ClipboardProvider>();
    final count = provider.filteredItems.length;

    return Container(
      height: 36,
      decoration: BoxDecoration(
        color: theme.scaffoldBackgroundColor,
        border: Border(
          top: BorderSide(color: theme.dividerColor),
        ),
      ),
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: Row(
        children: [
          Text(
            '$count item${count == 1 ? '' : 's'}',
            style: theme.textTheme.bodySmall,
          ),
          const Spacer(),
          IconButton(
            icon: const Icon(Icons.delete_sweep_outlined, size: 18),
            tooltip: 'Clear history',
            onPressed: () => _confirmClear(context),
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(),
          ),
        ],
      ),
    );
  }

  Future<void> _confirmClear(BuildContext context) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Clear History'),
        content: const Text(
          'Remove all clipboard history? Pinned items will be kept.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Clear'),
          ),
        ],
      ),
    );
    if (confirmed == true && context.mounted) {
      context.read<ClipboardProvider>().clearAll();
    }
  }
}
