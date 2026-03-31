import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:intl/intl.dart';

import '../models/clipboard_item.dart';
import '../providers/clipboard_provider.dart';

/// Right-hand panel showing the full content of the selected clipboard item.
class DetailPanel extends StatelessWidget {
  const DetailPanel({super.key});

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<ClipboardProvider>();
    final item = provider.selectedItem;

    if (item == null) {
      return _EmptyState();
    }

    return _ItemDetail(item: item);
  }
}

class _EmptyState extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.content_paste_rounded,
            size: 64,
            color: theme.colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text('Select an item to preview', style: theme.textTheme.bodyMedium),
        ],
      ),
    );
  }
}

class _ItemDetail extends StatelessWidget {
  final ClipboardItem item;

  const _ItemDetail({required this.item});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final provider = context.read<ClipboardProvider>();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // ── Toolbar ──────────────────────────────────────────────────────
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          decoration: BoxDecoration(
            color: theme.scaffoldBackgroundColor,
            border:
                Border(bottom: BorderSide(color: theme.dividerColor)),
          ),
          child: Row(
            children: [
              _MetaChip(
                icon: Icons.access_time_rounded,
                label: DateFormat('MMM d, HH:mm').format(item.timestamp),
              ),
              if (item.sourceApp != null) ...[
                const SizedBox(width: 8),
                _MetaChip(
                  icon: Icons.apps_rounded,
                  label: item.sourceApp!,
                ),
              ],
              const Spacer(),
              // Pin toggle
              IconButton(
                icon: Icon(
                  item.isPinned
                      ? Icons.push_pin
                      : Icons.push_pin_outlined,
                  size: 18,
                  color: item.isPinned
                      ? theme.colorScheme.primary
                      : theme.colorScheme.outline,
                ),
                tooltip: item.isPinned ? 'Unpin' : 'Pin',
                onPressed: () => provider.togglePin(item.id),
              ),
              // Copy
              FilledButton.icon(
                icon: const Icon(Icons.content_copy, size: 15),
                label: const Text('Copy'),
                onPressed: () async {
                  await provider.copyItem(item);
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('Copied!'),
                        duration: Duration(milliseconds: 1000),
                        behavior: SnackBarBehavior.floating,
                      ),
                    );
                  }
                },
              ),
              const SizedBox(width: 8),
              // Delete
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 18),
                tooltip: 'Delete',
                onPressed: () => provider.deleteItem(item.id),
                color: theme.colorScheme.error,
              ),
            ],
          ),
        ),

        // ── Content area ─────────────────────────────────────────────────
        Expanded(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: _ContentView(item: item),
          ),
        ),
      ],
    );
  }
}

class _ContentView extends StatelessWidget {
  final ClipboardItem item;

  const _ContentView({required this.item});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    switch (item.type) {
      case ClipboardItemType.text:
      case ClipboardItemType.richText:
        return SelectableText(
          item.text ?? '',
          style: theme.textTheme.bodyLarge?.copyWith(
            fontFamily: 'monospace',
            height: 1.5,
          ),
        );

      case ClipboardItemType.image:
        if (item.imageBytes == null) {
          return const Center(child: Text('Image data unavailable'));
        }
        return Center(
          child: ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child: Image.memory(
              item.imageBytes!,
              fit: BoxFit.contain,
            ),
          ),
        );

      case ClipboardItemType.file:
        final paths = item.filePaths ?? [];
        return ListView.separated(
          itemCount: paths.length,
          separatorBuilder: (_, __) => const Divider(height: 1),
          itemBuilder: (ctx, i) {
            final path = paths[i];
            final name = path.split('/').last;
            return ListTile(
              leading: const Icon(Icons.insert_drive_file_outlined),
              title: Text(name),
              subtitle: Text(path, style: theme.textTheme.bodySmall),
              dense: true,
            );
          },
        );

      case ClipboardItemType.color:
        final colorHex = item.colorHex ?? '#000000';
        Color color = Colors.black;
        try {
          color = Color(
            int.parse(colorHex.replaceFirst('#', '0xFF')),
          );
        } catch (_) {}
        return Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Container(
              width: 120,
              height: 120,
              decoration: BoxDecoration(
                color: color,
                borderRadius: BorderRadius.circular(16),
                boxShadow: [
                  BoxShadow(
                    blurRadius: 20,
                    color: color.withOpacity(0.4),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            Text(
              colorHex.toUpperCase(),
              style: theme.textTheme.bodyLarge?.copyWith(
                fontFamily: 'monospace',
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        );

      case ClipboardItemType.unknown:
        return Center(
          child: Text('Unknown content type',
              style: theme.textTheme.bodyMedium),
        );
    }
  }
}

class _MetaChip extends StatelessWidget {
  final IconData icon;
  final String label;

  const _MetaChip({required this.icon, required this.label});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: theme.chipTheme.backgroundColor,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 12, color: theme.textTheme.bodySmall?.color),
          const SizedBox(width: 4),
          Text(label, style: theme.textTheme.bodySmall),
        ],
      ),
    );
  }
}
