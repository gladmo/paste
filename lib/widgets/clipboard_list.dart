import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:intl/intl.dart';

import '../models/clipboard_item.dart';
import '../providers/clipboard_provider.dart';

class ClipboardList extends StatelessWidget {
  const ClipboardList({super.key});

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<ClipboardProvider>();

    if (provider.isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    final items = provider.filteredItems;

    if (items.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.content_paste_off_outlined,
                size: 48,
                color: Theme.of(context).textTheme.bodySmall?.color),
            const SizedBox(height: 12),
            Text(
              'No items yet',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ],
        ),
      );
    }

    return ListView.builder(
      padding: const EdgeInsets.symmetric(vertical: 4),
      itemCount: items.length,
      itemBuilder: (ctx, i) {
        final item = items[i];
        return _ClipboardTile(
          key: ValueKey(item.id),
          item: item,
          isSelected: provider.selectedItemId == item.id,
        );
      },
    );
  }
}

class _ClipboardTile extends StatelessWidget {
  final ClipboardItem item;
  final bool isSelected;

  const _ClipboardTile({
    super.key,
    required this.item,
    required this.isSelected,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final provider = context.read<ClipboardProvider>();

    final bgColor = isSelected
        ? theme.colorScheme.primary.withOpacity(0.12)
        : Colors.transparent;

    return GestureDetector(
      onTap: () => provider.selectItem(item.id),
      onDoubleTap: () async {
        await provider.copyItem(item);
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(
              content: Text('Copied to clipboard'),
              duration: Duration(milliseconds: 1200),
              behavior: SnackBarBehavior.floating,
            ),
          );
        }
      },
      child: Container(
        color: bgColor,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Type icon
            _TypeBadge(type: item.type),
            const SizedBox(width: 10),
            // Content preview
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    item.preview,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.bodyLarge,
                  ),
                  const SizedBox(height: 2),
                  Row(
                    children: [
                      if (item.sourceApp != null) ...[
                        Icon(Icons.apps_rounded,
                            size: 10,
                            color: theme.textTheme.labelSmall?.color),
                        const SizedBox(width: 2),
                        Text(
                          item.sourceApp!,
                          style: theme.textTheme.labelSmall,
                        ),
                        const SizedBox(width: 6),
                      ],
                      Text(
                        _formatTime(item.timestamp),
                        style: theme.textTheme.labelSmall,
                      ),
                    ],
                  ),
                ],
              ),
            ),
            // Pin indicator
            if (item.isPinned)
              Icon(Icons.push_pin, size: 12, color: theme.colorScheme.primary),
          ],
        ),
      ),
    );
  }

  String _formatTime(DateTime dt) {
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inSeconds < 60) return 'Just now';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
    if (diff.inHours < 24) return '${diff.inHours}h ago';
    return DateFormat('MMM d').format(dt);
  }
}

class _TypeBadge extends StatelessWidget {
  final ClipboardItemType type;

  const _TypeBadge({required this.type});

  @override
  Widget build(BuildContext context) {
    final (icon, color) = _meta(type, Theme.of(context));
    return Container(
      width: 32,
      height: 32,
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Icon(icon, size: 16, color: color),
    );
  }

  (IconData, Color) _meta(ClipboardItemType type, ThemeData theme) {
    switch (type) {
      case ClipboardItemType.text:
      case ClipboardItemType.richText:
        return (Icons.text_fields_rounded, const Color(0xFF5C6BC0));
      case ClipboardItemType.image:
        return (Icons.image_outlined, const Color(0xFF26A69A));
      case ClipboardItemType.file:
        return (Icons.folder_outlined, const Color(0xFFFF7043));
      case ClipboardItemType.color:
        return (Icons.palette_outlined, const Color(0xFFAB47BC));
      case ClipboardItemType.unknown:
        return (Icons.help_outline, theme.colorScheme.outline);
    }
  }
}
