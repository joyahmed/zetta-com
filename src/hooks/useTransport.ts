import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { loadSaved, saveSaved } from '../utils/storage';
import { isPortError, parsePort } from '../utils/validate';

/// How often the UI asks Rust what is happening. Deliberately a poll rather
/// than an event per packet: audio runs at fifty packets a second per peer, and
/// an IPC message each would melt the webview. Polling also decouples the
/// redraw rate from the packet rate without needing a throttle.
const POLL_MS = 400;

export const useTransport = () => {
	const saved = useRef(loadSaved()).current;
	const [port, setPort] = useState(saved.port);
	const [stats, setStats] = useState<NetStats | null>(null);
	const [peers, setPeers] = useState<Peer[]>([]);
	const [error, setError] = useState('');

	// `peer` is written back empty rather than dropped from storage: the manual
	// peers hook migrates whatever was in it into the PCs list on first launch,
	// and clearing it here is what stops that running a second time.
	useEffect(() => {
		saveSaved({ port, peer: '' });
	}, [port]);

	// One poll answers both questions: net_stats returns null exactly when the
	// transport is stopped, so there is no separate "is it running" call that
	// could drift out of agreement with reality.
	useEffect(() => {
		let alive = true;
		const id = setInterval(async () => {
			try {
				const [s, p] = await Promise.all([
					invoke<NetStats | null>('net_stats'),
					invoke<Peer[]>('net_peers')
				]);
				if (!alive) return;
				setStats(s);
				setPeers(p);
			} catch (e) {
				if (alive) setError(String(e));
			}
		}, POLL_MS);
		// Without this, every re-render stacks another interval and the poll
		// rate quietly climbs until the webview is doing nothing else.
		return () => {
			alive = false;
			clearInterval(id);
		};
	}, []);

	const running = stats !== null;

	const start = async () => {
		setError('');
		const parsed = parsePort(port);
		if (isPortError(parsed)) {
			setError(parsed);
			return;
		}
		// No address here any more: discovery finds everyone on a normal
		// network, and a PC it cannot reach is added to the PCs list, which the
		// session already merges into the same send list this used to seed.
		try {
			await invoke('net_start', { port: parsed, peer: '' });
		} catch (e) {
			setError(String(e));
		}
	};

	const stop = async () => {
		setError('');
		try {
			await invoke('net_stop');
			setStats(null);
			setPeers([]);
		} catch (e) {
			setError(String(e));
		}
	};

	// Going on and off air is the one thing worth a global key that the window
	// does not already own, so the listener lives here beside start and stop
	// rather than in the shortcut hook, which would have to be handed both.
	// Deliberately does not raise the window: yanking it to the front every
	// time somebody goes off air would be worse than the key being quiet.
	useEffect(() => {
		const pending = listen('toggle-transport', () => {
			if (running) stop();
			else start();
		});
		// Re-subscribed whenever running or the port changes, so the handler
		// never closes over a stale answer to "is it on?".
		return () => {
			pending.then(f => f()).catch(() => {});
		};
	}, [running, port]);

	return {
		port,
		setPort,
		stats,
		peers,
		error,
		// Exposed so sibling hooks can report through the one banner rather
		// than each growing an error surface of its own.
		setError,
		start,
		stop,
		running
	};
};
