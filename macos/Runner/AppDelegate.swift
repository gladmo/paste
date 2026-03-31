import FlutterMacOS
import Foundation
import window_manager
import hotkey_manager
import local_notifier
import screen_retriever

@NSApplicationMain
class AppDelegate: FlutterAppDelegate, NSApplicationDelegate {
  override func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
    if !flag {
      for window in NSApplication.shared.windows {
        window.makeKeyAndOrderFront(self)
      }
    }
    return true
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }
}
