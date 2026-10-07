import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { DEMO, DEMO_VOLUMES } from '../demo';

/// How loud each PC is to you, by address. A missing entry is 100%.
///
/// Rust applies the change to the mixer on the next 20 ms frame, so there is
/// nothing to restart; this only keeps the screen in step with what it saved.
export const useVolumes = (onError: (m: string) => void) => {
	const [volumes, setVolumes] = useState<Record<string, number>>(DEMO ? DEMO_VOLUMES : {});

	useEffect(() => {
		if (DEMO) return;
		invoke<Config | null>('config_get')
			.then(c => setVolumes(c?.volumes ?? {}))
			.catch(() => {});
	}, []);

	const setVolume = async (addr: string, gain: number) => {
		setVolumes(v => ({ ...v, [addr]: gain }));
		if (DEMO) return;
		try {
			await invoke('set_volume', { addr, gain });
		} catch (e) {
			onError(String(e));
		}
	};

	return { volumes, setVolume };
};
