/// Turn a real keypress into the spec Tauri registers.
///
/// Built from `event.code`, not `event.key`. `code` is the physical key and is
/// already the vocabulary Tauri uses — "KeyK", "Digit1", "F8" — while `key` is
/// what the key *produces*, which changes with the layout and with Shift: the
/// same press is "1" or "!" depending, and neither is a name Tauri knows.
///
/// Returns null for a press that is only modifiers, so holding Ctrl while
/// reaching for a letter does not register Ctrl by itself.
export const toSpec = (e: KeyboardEvent): string | null => {
	const code = e.code;
	if (/^(Control|Alt|Shift|Meta)(Left|Right)$/.test(code)) return null;

	const parts: string[] = [];
	// Cmd on a Mac and Ctrl elsewhere are the same intent, and Tauri spells
	// that CommandOrControl.
	if (e.ctrlKey || e.metaKey) parts.push('CommandOrControl');
	if (e.altKey) parts.push('Alt');
	if (e.shiftKey) parts.push('Shift');
	parts.push(code);
	return parts.join('+');
};

/// The same spec written for a person: Ctrl rather than CommandOrControl, and
/// no Key/Digit noise on the cap.
export const readable = (spec: string): string =>
	spec
		.replace('CommandOrControl', 'Ctrl')
		.replace('Digit', '')
		.replace('Key', '');
