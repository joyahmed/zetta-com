import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

/// Whether Windows starts the app at login, and how long it waits before
/// binding when it does.
///
/// The two live together because neither means anything alone: a delay applies
/// only to a login start, and a login start with no delay is the failure the
/// delay exists for. They are stored apart, though — the registry entry belongs
/// to the plugin, the wait to our own config — so this is the only place that
/// knows they are one setting.
export const useStartup = (onError: (m: string) => void) => {
	const [on, setOn] = useState(false);
	const [delay, setDelay] = useState('10');

	// Read the registry rather than remembering what we last wrote: the entry
	// can be turned off in Task Manager's Startup tab, and a switch that
	// disagrees with the machine is worse than no switch.
	useEffect(() => {
		isEnabled().then(setOn).catch(() => {});
		invoke<Config | null>('config_get')
			.then(c => c && setDelay(String(c.startDelay)))
			.catch(() => {});
	}, []);

	const choose = async (next: boolean) => {
		try {
			await (next ? enable() : disable());
			// From the plugin, not from `next`: enabling can fail silently on a
			// locked-down machine, and this is the answer that is true.
			setOn(await isEnabled());
		} catch (e) {
			onError(String(e));
		}
	};

	// Committed on blur, not per keystroke. Typing "45" over "20" passes through
	// "4", and saving that would write a four-second wait to disk on the way to
	// a valid number.
	const commit = async () => {
		const n = Number.parseInt(delay, 10);
		if (Number.isNaN(n) || n < 0) return setDelay('10');
		try {
			setDelay(String(await invoke<number>('set_start_delay', { seconds: n })));
		} catch (e) {
			onError(String(e));
		}
	};

	return { on, choose, delay, setDelay, commit };
};
