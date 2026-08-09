import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

/// Hand-entered peers, kept alongside whatever mDNS finds.
///
/// The list lives in Rust's config file rather than here, because the transport
/// has to bind to it before any window exists. This hook is a view of it.
export const useManualPeers = (onError: (message: string) => void) => {
	const [manual, setManual] = useState<string[]>([]);
	const [presets, setPresets] = useState<Preset[]>([]);

	useEffect(() => {
		invoke<Config | null>('config_get')
			.then(c => {
				setManual(c?.manual ?? []);
				setPresets(c?.presets ?? []);
			})
			.catch(() => {});
	}, []);

	const add = async (addr: string) => {
		try {
			setManual(await invoke<string[]>('manual_peers', { add: addr }));
		} catch (e) {
			onError(String(e));
		}
	};

	const remove = async (addr: string) => {
		try {
			setManual(await invoke<string[]>('manual_peers', { remove: addr }));
		} catch (e) {
			onError(String(e));
		}
	};

	/// Your own name for a machine. Empty clears it and the discovered name
	/// comes back.
	const rename = async (addr: string, label: string) => {
		try {
			await invoke('set_label', { addr, label });
		} catch (e) {
			onError(String(e));
		}
	};

	return { manual, presets, add, remove, rename };
};
