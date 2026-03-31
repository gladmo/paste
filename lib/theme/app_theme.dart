import 'package:flutter/material.dart';

class AppTheme {
  AppTheme._();

  // Brand colors
  static const Color accent = Color(0xFF5C6BC0); // indigo-400
  static const Color accentLight = Color(0xFF7986CB); // indigo-300
  static const Color danger = Color(0xFFEF5350);

  // ── Light ────────────────────────────────────────────────────────────────
  static final ThemeData lightTheme = ThemeData(
    useMaterial3: true,
    brightness: Brightness.light,
    colorSchemeSeed: accent,
    scaffoldBackgroundColor: const Color(0xFFF5F5F7),
    cardColor: Colors.white,
    dividerColor: const Color(0xFFE0E0E0),
    appBarTheme: const AppBarTheme(
      backgroundColor: Color(0xFFF5F5F7),
      foregroundColor: Color(0xFF1C1C1E),
      elevation: 0,
      scrolledUnderElevation: 1,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: Colors.white,
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: Color(0xFFE0E0E0)),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: Color(0xFFE0E0E0)),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: accent, width: 1.5),
      ),
    ),
    textTheme: const TextTheme(
      bodyLarge: TextStyle(fontSize: 14, color: Color(0xFF1C1C1E)),
      bodyMedium: TextStyle(fontSize: 13, color: Color(0xFF3A3A3C)),
      bodySmall: TextStyle(fontSize: 11, color: Color(0xFF8E8E93)),
      labelSmall: TextStyle(fontSize: 10, color: Color(0xFF8E8E93)),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: const Color(0xFFEEEEF0),
      selectedColor: accent.withOpacity(0.15),
      labelStyle: const TextStyle(fontSize: 12),
      side: BorderSide.none,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    ),
  );

  // ── Dark ─────────────────────────────────────────────────────────────────
  static final ThemeData darkTheme = ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorSchemeSeed: accent,
    scaffoldBackgroundColor: const Color(0xFF1C1C1E),
    cardColor: const Color(0xFF2C2C2E),
    dividerColor: const Color(0xFF3A3A3C),
    appBarTheme: const AppBarTheme(
      backgroundColor: Color(0xFF1C1C1E),
      foregroundColor: Color(0xFFF2F2F7),
      elevation: 0,
      scrolledUnderElevation: 1,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: const Color(0xFF2C2C2E),
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: Color(0xFF3A3A3C)),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: Color(0xFF3A3A3C)),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: accentLight, width: 1.5),
      ),
    ),
    textTheme: const TextTheme(
      bodyLarge: TextStyle(fontSize: 14, color: Color(0xFFF2F2F7)),
      bodyMedium: TextStyle(fontSize: 13, color: Color(0xFFAEAEB2)),
      bodySmall: TextStyle(fontSize: 11, color: Color(0xFF636366)),
      labelSmall: TextStyle(fontSize: 10, color: Color(0xFF636366)),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: const Color(0xFF3A3A3C),
      selectedColor: accent.withOpacity(0.25),
      labelStyle: const TextStyle(fontSize: 12),
      side: BorderSide.none,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    ),
  );
}
