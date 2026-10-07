import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { DEMO, DEMO_GROUPS } from '../demo';

/// Saved groups of PCs: "Devs", "Front desk".
///
/// Saved whole on every change rather than one group at a time: there are a
/// handful of them, and one write of the list cannot leave the file holding
/// half an edit.
export const useGroups = (onError: (m: string) => void) => {
	const [groups, setGroups] = useState<Group[]>(DEMO ? DEMO_GROUPS : []);

	useEffect(() => {
		if (DEMO) return;
		invoke<Config | null>('config_get')
			.then(c => setGroups(c?.groups ?? []))
			.catch(() => {});
	}, []);

	const save = async (next: Group[]) => {
		setGroups(next);
		if (DEMO) return;
		try {
			// Rust trims names and drops blanks and repeats; show what it kept.
			setGroups(await invoke<Group[]>('set_groups', { groups: next }));
		} catch (e) {
			onError(String(e));
		}
	};

	return { groups, save };
};
