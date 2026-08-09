import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';
import { loadSaved, saveSaved } from '../utils/storage';

/// Manually entered peers, kept alongside whatever mDNS finds.
///
/// The list lives in Rust's config file rather than here, because the transport
/// has to bind to it before any window exists. This hook is a view of it.
export const useManualPeers = (onError: (message: string) => void) => {
	const [manual, setManual] = useState<string[]>([]);
	const [presets, setPresets] = useState<Preset[]>([]);

	// Read synchronously at first render, not inside the effect below. The
	// transport hook clears this same key on mount, and whichever effect ran
	// second would otherwise decide whether the migration happened at all.
	const stray = useRef(loadSaved().peer.trim()).current;

	useEffect(() => {
		const load = async () => {
			const cfg = await invoke<Config | null>('config_get').catch(() => null);
			setPresets(cfg?.presets ?? []);
			let list = cfg?.manual ?? [];

			// One-time migration. "Fallback address" in Settings was a second,
			// separate place to type a PC: stored in localStorage, silently
			// merged into the send list by the session, and impossible to see or
			// delete in the PCs list — so it showed up as a machine nobody could
			// account for. Folded in here so a PC lives in exactly one place.
			if (stray) {
				try {
					list = await invoke<string[]>('manual_peers', { add: stray });
				} catch {
					// A stored address that no longer parses is simply dropped;
					// failing a launch over a dead setting helps nobody.
				}
				saveSaved({ ...loadSaved(), peer: '' });
			}

			setManual(list);
		};
		load();
	}, [stray]);

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

	/// Correct an address in place.
	///
	/// The new one is added before the old one is removed, so a typo is refused
	/// while the entry it was meant to replace is still there. The reverse order
	/// would delete a working PC and then fail to add its replacement.
	const edit = async (from: string, to: string) => {
		try {
			await invoke<string[]>('manual_peers', { add: to });
			setManual(await invoke<string[]>('manual_peers', { remove: from }));
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

	return { manual, presets, add, remove, edit, rename };
};
