import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../providers/clipboard_provider.dart';

class SearchField extends StatefulWidget {
  const SearchField({super.key});

  @override
  State<SearchField> createState() => _SearchFieldState();
}

class _SearchFieldState extends State<SearchField> {
  final _controller = TextEditingController();
  final _focusNode = FocusNode();

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      color: theme.scaffoldBackgroundColor,
      child: TextField(
        controller: _controller,
        focusNode: _focusNode,
        autofocus: false,
        textInputAction: TextInputAction.search,
        decoration: InputDecoration(
          hintText: 'Search clipboard…',
          prefixIcon: const Icon(Icons.search, size: 18),
          suffixIcon: _controller.text.isNotEmpty
              ? IconButton(
                  icon: const Icon(Icons.close, size: 16),
                  onPressed: () {
                    _controller.clear();
                    context.read<ClipboardProvider>().setSearchQuery('');
                  },
                )
              : null,
        ),
        onChanged: (v) {
          setState(() {});
          context.read<ClipboardProvider>().setSearchQuery(v);
        },
      ),
    );
  }
}
