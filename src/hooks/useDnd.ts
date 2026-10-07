import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { DEMO } from '../demo';
import { setQuiet } from '../utils/sounds';

/// Do-not-disturb: no voice played here, no sounds, no toasts, and the other
/// machines are told so they can see it beside your name.
///
/// Rust holds the real switch, because the tray can flip it with the window
/// shut. The window asks once on load and then follows the `dnd` event, which
/// Rust sends whichever side made the change — so the two never disagree.
///
/// Never saved. It starts off at every launch, on purpose: a DND forgotten
/// before a reboot would otherwise go on silently swallowing instructions.
export const useDnd = (onError: (message: string) => void) => {
	const [dnd, setDndState] = useState(false);

	useEffect(() => setQuiet(dnd), [dnd]);

	useEffect(() => {
		if (DEMO) return;
		invoke<boolean>('dnd_get')
			.then(setDndState)
			.catch(() => {});
		const off = listen<boolean>('dnd', e => setDndState(e.payload));
		return () => {
			off.then(f => f()).catch(() => {});
		};
	}, []);

	const setDnd = async (on: boolean) => {
		// Nothing to tell in demo mode; the bar still appears.
		if (DEMO) return setDndState(on);
		try {
			await invoke('set_dnd', { on });
			setDndState(on);
		} catch (e) {
			onError(String(e));
		}
	};

	return { dnd, setDnd };
};
