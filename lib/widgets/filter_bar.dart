import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../providers/clipboard_provider.dart';

class FilterBar extends StatelessWidget {
  const FilterBar({super.key});

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<ClipboardProvider>();
    final theme = Theme.of(context);

    return Container(
      color: theme.scaffoldBackgroundColor,
      padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: ClipboardFilter.values.map((f) {
            final selected = provider.filter == f;
            return Padding(
              padding: const EdgeInsets.only(right: 6),
              child: FilterChip(
                label: Text(_label(f)),
                avatar: Icon(_icon(f), size: 14),
                selected: selected,
                onSelected: (_) => provider.setFilter(f),
                showCheckmark: false,
                visualDensity: VisualDensity.compact,
              ),
            );
          }).toList(),
        ),
      ),
    );
  }

  String _label(ClipboardFilter f) {
    switch (f) {
      case ClipboardFilter.all:
        return 'All';
      case ClipboardFilter.text:
        return 'Text';
      case ClipboardFilter.image:
        return 'Images';
      case ClipboardFilter.file:
        return 'Files';
      case ClipboardFilter.pinned:
        return 'Pinned';
    }
  }

  IconData _icon(ClipboardFilter f) {
    switch (f) {
      case ClipboardFilter.all:
        return Icons.apps_rounded;
      case ClipboardFilter.text:
        return Icons.text_fields_rounded;
      case ClipboardFilter.image:
        return Icons.image_outlined;
      case ClipboardFilter.file:
        return Icons.folder_outlined;
      case ClipboardFilter.pinned:
        return Icons.push_pin_outlined;
    }
  }
}
