import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

/// Custom title bar that blends with the macOS / Windows system chrome.
class TitleBar extends StatelessWidget {
  const TitleBar({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return GestureDetector(
      onPanStart: (_) => windowManager.startDragging(),
      child: Container(
        height: 52,
        decoration: BoxDecoration(
          color: theme.scaffoldBackgroundColor,
          border: Border(
            bottom: BorderSide(color: theme.dividerColor),
          ),
        ),
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          children: [
            // App icon + name
            const _WindowControls(),
            const SizedBox(width: 12),
            Icon(
              Icons.content_paste_rounded,
              size: 20,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(width: 8),
            Text(
              'Paste',
              style: theme.textTheme.bodyLarge?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const Spacer(),
            // Settings button placeholder
            IconButton(
              icon: const Icon(Icons.settings_outlined, size: 18),
              tooltip: 'Settings',
              onPressed: () {},
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(),
            ),
          ],
        ),
      ),
    );
  }
}

class _WindowControls extends StatelessWidget {
  const _WindowControls();

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        _CircleButton(
          color: const Color(0xFFFF5F57),
          tooltip: 'Close',
          onTap: () => windowManager.close(),
        ),
        const SizedBox(width: 8),
        _CircleButton(
          color: const Color(0xFFFFBD2E),
          tooltip: 'Minimize',
          onTap: () => windowManager.minimize(),
        ),
        const SizedBox(width: 8),
        _CircleButton(
          color: const Color(0xFF28C840),
          tooltip: 'Maximize',
          onTap: () => windowManager.maximize(),
        ),
      ],
    );
  }
}

class _CircleButton extends StatelessWidget {
  final Color color;
  final String tooltip;
  final VoidCallback onTap;

  const _CircleButton({
    required this.color,
    required this.tooltip,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: GestureDetector(
        onTap: onTap,
        child: Container(
          width: 12,
          height: 12,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
        ),
      ),
    );
  }
}
