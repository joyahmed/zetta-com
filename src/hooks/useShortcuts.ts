import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

/// The key list, plus the two keys that only ask the window to show something.
///
/// Those two are events rather than commands because the work is the
/// frontend's: Rust knows a key was pressed, not what a panel looks like.
export const useShortcuts = () => {
	const [shortcuts, setShortcuts] = useState<ShortcutInfo[]>([]);
	const [showShortcuts, setShowShortcuts] = useState(false);
	const [showAddPc, setShowAddPc] = useState(false);

	useEffect(() => {
		invoke<ShortcutInfo[]>('shortcuts').then(setShortcuts).catch(() => {});
	}, []);

	useEffect(() => {
		// Adding a PC lost its global key: it is a once-per-machine job with a
		// button already on screen, and it was costing a system-wide chord.
		const unlisten = [listen('show-shortcuts', () => setShowShortcuts(v => !v))];
		// Each listen() resolves to its own unlisten. Without this every
		// re-render would stack another one and a single keypress would fire
		// the handler as many times as the component had rendered.
		return () => {
			unlisten.forEach(p => p.then(f => f()).catch(() => {}));
		};
	}, []);

	return {
		shortcuts,
		showShortcuts,
		setShowShortcuts,
		showAddPc,
		setShowAddPc
	};
};
