import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';

/// Who voice and text are aimed at. `null` is everyone.
///
/// Kept in Rust rather than only here, because a global shortcut fires without
/// the window being open and still has to know who it is talking to.
export const useTarget = (onError: (message: string) => void) => {
	const [target, setTargetState] = useState<string | null>(null);

	const setTarget = async (addr: string | null) => {
		try {
			await invoke('set_target', { addr });
			setTargetState(addr);
		} catch (e) {
			onError(String(e));
		}
	};

	return { target, setTarget };
};
