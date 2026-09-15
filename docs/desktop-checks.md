# Live desktop acceptance checks

These require the target Omarchy Quattro desktop. Automated Qt fixtures do not verify compositor behavior. This checklist has not yet been run on the user's machine.

- Install Core from a clean checkout and confirm its service enables without creating a second Quickshell process.
- Open/close the empty manager, including Escape; confirm the rest of the desktop remains interactive.
- Install a separate trusted widget package. Confirm installation alone creates no desktop surface, and Add creates one card.
- Confirm normal application windows cover the bottom-layer card and pointer input outside the card reaches the desktop.
- Arrange using drag, arrow keys and Shift+arrow; switch sizes and monitors. Restart the shell and verify persistence.
- Unplug/reconnect the preferred monitor and check fallback plus retained preference. Test fractional scale and a small screen.
- Hide/show, enter/leave fullscreen, and verify the widget stops its producers while unloaded.
- Switch light/dark themes and scale. Check contrast, clipping, text readability and arrangement controls.
- Try a malformed manifest, unsupported API and invalid saved layout. Confirm clear failure with the layout preserved.
- Kill a registry write, inspect the reported lock recovery instructions, and verify recovery without deleting settings.
- Validate lock-screen behavior on the target compositor. Core does not query private Quattro lock services; the compositor's secure lock must cover these surfaces.
- Remove a widget deliberately; confirm its package and settings disappear. Disable Core and confirm widget data remains on disk.
