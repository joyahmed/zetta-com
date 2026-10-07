import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { DEMO } from '../demo';

/// The prefix that marks a target as a group rather than an address. A colon
/// can never start an address, so the two cannot be confused.
export const GROUP = 'group:';

/// Who voice and text are aimed at. `null` is everyone, `group:<name>` a saved
/// group, anything else one machine's address.
///
/// Kept in Rust rather than only here, because a global shortcut fires without
/// the window being open and still has to know who it is talking to.
export const useTarget = (onError: (message: string) => void) => {
	const [target, setTargetState] = useState<string | null>(null);

	const setTarget = async (to: string | null) => {
		// Nothing is running in demo mode to aim; the chip still moves.
		if (DEMO) return setTargetState(to);
		try {
			if (to?.startsWith(GROUP)) {
				await invoke('set_target_group', { name: to.slice(GROUP.length) });
			} else {
				await invoke('set_target', { addr: to });
			}
			setTargetState(to);
		} catch (e) {
			onError(String(e));
		}
	};

	return { target, setTarget };
};
