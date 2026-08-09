import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { DEMO, DEMO_ROOM } from '../demo';

/// The room passphrase and its short code.
///
/// The code comes back from Rust rather than being derived here: it is a hash
/// of the key, and two implementations of the same hash are two chances to
/// disagree. One of them would then show a code that matched nothing.
export const useRoom = (onError: (message: string) => void) => {
	const [passphrase, setPassphrase] = useState('');
	const [code, setCode] = useState<string | null>(null);

	const apply = async (next: string) => {
		if (DEMO) {
			setPassphrase(next);
			setCode(next ? DEMO_ROOM.code : null);
			return;
		}
		try {
			setCode(await invoke<string | null>('set_passphrase', { passphrase: next }));
			setPassphrase(next);
		} catch (e) {
			onError(String(e));
		}
	};

	useEffect(() => {
		if (DEMO) {
			setPassphrase(DEMO_ROOM.passphrase);
			setCode(DEMO_ROOM.code);
			return;
		}
		invoke<Config | null>('config_get')
			.then(async c => {
				const p = c?.passphrase ?? '';
				setPassphrase(p);
				if (p) setCode(await invoke<string | null>('room_code', { passphrase: p }));
			})
			.catch(() => {});
	}, []);

	/// Generated in Rust, from the operating system's randomness. Doing it in
	/// the webview would mean trusting whatever `crypto` the runtime provides,
	/// and a room key is the one thing that must not be guessable.
	const generate = async () => {
		if (DEMO) return apply(DEMO_ROOM.passphrase);
		try {
			await apply(await invoke<string>('room_new'));
		} catch (e) {
			onError(String(e));
		}
	};

	return { passphrase, code, generate, join: apply };
};
