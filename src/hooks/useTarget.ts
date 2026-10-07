import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { DEMO } from '../demo';

/// The prefix that marks a chip key as a group rather than an address. A colon
/// can never start an address, so the two cannot be confused.
export const GROUP = 'group:';

/// The chip key for a target: null for everyone, `group:<id>` for a group, an
/// address for one PC. A deleted group gets a key no chip has, so the strip
/// selects nothing — selecting Everyone there would be the lie this replaced.
export const keyOf = (t: Target): string | null => {
	switch (t.kind) {
		case 'everyone':
			return null;
		case 'pc':
			return t.addr;
		case 'group':
			return GROUP + t.id;
		case 'gone':
			return 'gone:';
	}
};

/// What demo mode does in place of `state::resolve`: follow a rename, and turn
/// a deleted group into nobody. Nothing else in demo needs Rust's rules.
const demoResolve = (t: Target, groups: Group[]): Target => {
	if (t.kind !== 'group') return t;
	const g = groups.find(x => x.id === t.id);
	return g ? { kind: 'group', id: g.id, name: g.name } : { kind: 'gone', name: t.name };
};

/// Who voice and text are aimed at.
///
/// Rust owns it, and this only mirrors it. A global shortcut aims without the
/// window, a restart must not lose it, and editing a group has to move it —
/// all three happen in Rust, and a copy kept here was a copy that went stale:
/// the chips showed one person while the keys talked to another, and a restart
/// left the chip on someone while the socket had gone back to everyone.
///
/// So: read it when the window loads and whenever the transport starts or
/// stops, follow the `target` event for everything else, and only ever ask
/// Rust to change it — never set it here and hope. Demo mode has no Rust, so
/// there the chips are moved locally.
export const useTarget = (
	running: boolean,
	groups: Group[],
	onError: (message: string) => void
) => {
	const [target, setTargetState] = useState<Target>({ kind: 'everyone' });

	useEffect(() => {
		if (DEMO) return;
		invoke<Target>('get_target')
			.then(setTargetState)
			.catch(() => {});
	}, [running]);

	useEffect(() => {
		if (DEMO) return;
		const off = listen<Target>('target', e => setTargetState(e.payload));
		return () => {
			off.then(f => f());
		};
	}, []);

	useEffect(() => {
		if (DEMO) setTargetState(t => demoResolve(t, groups));
	}, [groups]);

	const setTarget = async (to: string | null) => {
		if (DEMO) {
			const g = to?.startsWith(GROUP)
				? groups.find(x => GROUP + x.id === to)
				: undefined;
			return setTargetState(
				to === null
					? { kind: 'everyone' }
					: g
						? { kind: 'group', id: g.id, name: g.name }
						: { kind: 'pc', addr: to }
			);
		}
		try {
			if (to?.startsWith(GROUP)) {
				await invoke('set_target_group', { id: to.slice(GROUP.length) });
			} else {
				await invoke('set_target', { addr: to });
			}
		} catch (e) {
			onError(String(e));
		}
	};

	return { target, key: keyOf(target), setTarget };
};
