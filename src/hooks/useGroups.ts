import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { DEMO, DEMO_GROUPS } from '../demo';

/// The rules Rust applies in `config::clean_groups`, for demo mode only, where
/// there is no Rust to ask. Kept to the same words so a screenshot of the
/// refusal is a screenshot of the real one.
const demoClean = (next: Group[]): Group[] => {
	const seen: string[] = [];
	return next.map((g, i) => {
		const name = g.name.trim();
		if (!name) throw new Error('A group needs a name.');
		if (seen.includes(name.toLowerCase()))
			throw new Error(`There is already a group called ${name}.`);
		seen.push(name.toLowerCase());
		return {
			id: g.id || `demo-${Date.now()}-${i}`,
			name,
			members: [...new Set(g.members)].sort()
		};
	});
};

/// Saved groups of PCs: "Devs", "Front desk".
///
/// Saved whole on every change rather than one group at a time: there are a
/// handful of them, and one write of the list cannot leave the file holding
/// half an edit.
///
/// The list shown is only ever what Rust kept. It used to be set to the edit
/// first and then corrected — so a save Rust refused still showed, briefly or
/// for good, as if it had worked. A failure is thrown to the caller, which is
/// the editor, so it can say why with the name still in front of you.
export const useGroups = () => {
	const [groups, setGroups] = useState<Group[]>(DEMO ? DEMO_GROUPS : []);

	useEffect(() => {
		if (DEMO) return;
		invoke<Config | null>('config_get')
			.then(c => setGroups(c?.groups ?? []))
			.catch(() => {});
	}, []);

	const save = async (next: Group[]) => {
		if (DEMO) return setGroups(demoClean(next));
		// Not caught here: a refusal goes to whoever asked for the save.
		setGroups(await invoke<Group[]>('set_groups', { groups: next }));
	};

	return { groups, save };
};
