import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

/// Polled rather than pushed. The key is registered globally in Rust, so it
/// works while the window is not focused — which is the whole point of a global
/// shortcut and also why the webview cannot simply listen for keydown.
const POLL_MS = 100;

export const usePtt = () => {
	const [held, setHeld] = useState(false);

	useEffect(() => {
		let alive = true;
		const id = setInterval(async () => {
			try {
				const h = await invoke<boolean>('ptt_held');
				if (alive) setHeld(h);
			} catch {
				// A failed poll is not worth an error banner; the next one is
				// a tenth of a second away.
			}
		}, POLL_MS);
		return () => {
			alive = false;
			clearInterval(id);
		};
	}, []);

	return held;
};
