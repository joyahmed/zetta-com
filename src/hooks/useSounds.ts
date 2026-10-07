import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { DEMO } from '../demo';
import { setSoundPrefs, sound } from '../utils/sounds';

const DEFAULTS: Sounds = {
	message: true,
	voice: true,
	key: false,
	presence: true,
	volume: 0.7
};

/// The sound settings, the per-PC watch list, and the listeners that play the
/// sounds Rust announces.
///
/// The voice and key sounds arrive as events because a poll would play them
/// late. Presence does not need one: "offline" already means seven seconds
/// without a heartbeat, so the 400 ms roster poll adds nothing anyone hears.
export const useSounds = (
	peers: Peer[],
	running: boolean,
	onError: (m: string) => void
) => {
	const [sounds, setSounds] = useState<Sounds>(DEFAULTS);
	const [watched, setWatched] = useState<string[]>([]);

	useEffect(() => {
		if (DEMO) return;
		invoke<Config | null>('config_get')
			.then(c => {
				if (!c) return;
				if (c.sounds) setSounds(c.sounds);
				setWatched(c.watched ?? []);
			})
			.catch(() => {});
	}, []);

	useEffect(() => setSoundPrefs(sounds), [sounds]);

	useEffect(() => {
		const pending = [
			listen('voice-start', () => sound('voice')),
			listen<boolean>('talk-key', e => sound(e.payload ? 'keyDown' : 'keyUp'))
		];
		return () => {
			for (const p of pending) p.then(f => f()).catch(() => {});
		};
	}, []);

	// Who was live at the last poll. Empty until the first poll after a start,
	// so starting the transport does not announce everybody coming online, and
	// cleared on stop, so stopping does not announce everybody leaving.
	const was = useRef<Map<string, boolean> | null>(null);
	useEffect(() => {
		if (!running) {
			was.current = null;
			return;
		}
		const now = new Map(peers.map(p => [p.addr, p.live]));
		const before = was.current;
		was.current = now;
		if (!before) return;
		// One sound per poll, however many changed. Gone wins over back: of the
		// two, it is the one somebody might have to act on.
		// A PC missing from the roster counts as not live, so one that drops out
		// of discovery entirely is "gone" and one that reappears is "back".
		const live = (m: Map<string, boolean>, a: string) => m.get(a) ?? false;
		const changed = watched.filter(a => live(before, a) !== live(now, a));
		if (changed.some(a => !live(now, a))) sound('offline');
		else if (changed.length > 0) sound('online');
	}, [peers, running, watched]);

	const choose = async (next: Partial<Sounds>) => {
		const merged = { ...sounds, ...next };
		setSounds(merged);
		if (DEMO) return;
		try {
			await invoke('set_sounds', { sounds: merged });
		} catch (e) {
			onError(String(e));
		}
	};

	const watch = async (addr: string, on: boolean) => {
		setWatched(w => (on ? [...w.filter(a => a !== addr), addr] : w.filter(a => a !== addr)));
		if (DEMO) return;
		try {
			setWatched(await invoke<string[]>('set_watched', { addr, on }));
		} catch (e) {
			onError(String(e));
		}
	};

	return { sounds, choose, watched, watch };
};
